use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::app::operation::ExecutionSurface;
use crate::log::raw::{RAW_LOG_ACK, RawLogConfig};
use crate::sequences::builtin::builtins;
use crate::transport::test_support::MockTransport;

use super::*;

struct TimeoutRecordingTransport {
    timeouts: Arc<Mutex<Vec<Duration>>>,
}

impl AtTransport for TimeoutRecordingTransport {
    fn open(&mut self) -> Result<()> {
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_command(&mut self, _command: &str) -> Result<()> {
        Ok(())
    }

    fn read_response(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        self.timeouts.lock().unwrap().push(timeout);
        Ok(b"\r\nOK\r\n".to_vec())
    }

    fn read_until(&mut self, timeout: Duration, _matcher: ResponseMatcher) -> Result<Vec<u8>> {
        self.timeouts.lock().unwrap().push(timeout);
        Ok(b"\r\nOK\r\n".to_vec())
    }
}

#[test]
fn resolved_total_timeout_caps_sequence_definition_and_step_hint() {
    let sequence = Sequence::built_in(
        "bounded-sequence",
        "Verify resolved total timeout ownership.",
        vec!["test".to_owned()],
        Some(180),
        Vec::new(),
        vec![SequenceStep {
            id: "bounded-step".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some("AT".to_owned()),
            expect: Some("OK".to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack: false,
            require_ping_success: false,
            timeout_secs: Some(90),
            evidence: None,
            cleanup_on_failure: None,
        }],
    );
    let timeouts = Arc::new(Mutex::new(Vec::new()));
    let transport = TimeoutRecordingTransport {
        timeouts: Arc::clone(&timeouts),
    };

    execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(7),
        true,
        None,
    )
    .unwrap();

    let observed = timeouts.lock().unwrap();
    assert_eq!(observed.len(), 1);
    assert!(observed[0] <= Duration::from_secs(7));
    assert!(observed[0] > Duration::from_secs(6));
}

#[test]
fn sequence_timeout_error_identifies_failing_step() {
    let sequence = Sequence::built_in(
        "timeout-sequence",
        "Verify timeout step identity.",
        vec!["test".to_owned()],
        None,
        Vec::new(),
        vec![SequenceStep {
            id: "wait-for-modem".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some("AT".to_owned()),
            expect: Some("OK".to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack: false,
            require_ping_success: false,
            timeout_secs: None,
            evidence: None,
            cleanup_on_failure: None,
        }],
    );
    let transport = MockTransport::with_responses(Vec::<Vec<u8>>::new());

    let error = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(5),
        true,
        None,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AtctlError::SequenceStepTimeout {
            ref sequence,
            ref step,
            timeout_secs: 5,
        } if sequence == "timeout-sequence" && step == "wait-for-modem"
    ));
}

#[test]
fn step_timeout_never_extends_a_subsecond_remaining_total_budget() {
    let sequence = Sequence::built_in(
        "subsecond-budget",
        "Verify the total deadline remains authoritative.",
        vec!["test".to_owned()],
        None,
        Vec::new(),
        vec![SequenceStep {
            id: "bounded-step".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some("AT".to_owned()),
            expect: Some("OK".to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack: false,
            require_ping_success: false,
            timeout_secs: Some(30),
            evidence: None,
            cleanup_on_failure: None,
        }],
    );
    let step = &sequence.steps[0];
    let deadline = Instant::now() + Duration::from_millis(500);

    let timeout = step_timeout(&sequence, step, Duration::from_secs(30), deadline).unwrap();

    assert!(timeout <= Duration::from_millis(500));
    assert!(!timeout.is_zero());
}

#[test]
fn sms_send_sequence_waits_for_prompt_writes_payload_and_masks_params() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-send-check")
        .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\nOK\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\n+CMGS: 12\r\n\r\nOK\r\n".to_vec(),
    ]);
    let params = vec![
        SequenceParamValue {
            name: "recipient".to_owned(),
            value: "+819012345678".to_owned(),
        },
        SequenceParamValue {
            name: "message".to_owned(),
            value: "hello from atctl".to_owned(),
        },
    ];
    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert!(execution.masked_transcript.contains("+************"));
    assert!(!execution.masked_transcript.contains("+819012345678"));
    assert!(!execution.masked_transcript.contains("hello from atctl"));
    assert!(execution.masked_transcript.contains("+CMGS"));
    assert!(execution.masked_transcript.contains("submit evidence"));
    assert!(execution.masked_transcript.contains("Command:"));
    assert!(execution.masked_transcript.contains("Payload:"));
    assert!(execution.masked_transcript.contains("Modem response:"));
    assert!(execution.masked_transcript.contains("Notes:"));
    assert!(
        execution
            .masked_transcript
            .contains("Step 1/3 Set SMS text mode\n\nCommand:\n> AT+CMGF=1\n\nModem response:")
    );
    assert!(execution.masked_transcript.contains(
            "Notes:\n- +CMGS plus OK is SMS submit evidence, not destination handset receipt proof.\n\nResult: OK duration="
        ));
    assert!(!execution.masked_transcript.contains("Command:\n\n>"));
    assert!(!execution.masked_transcript.contains("\n\n\n"));
    assert!(!execution.masked_transcript.contains("-----"));
    assert!(!execution.masked_transcript.contains("Evidence:"));
}

#[test]
fn quectel_sequence_waits_for_urc_and_send_ok() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let open_socket = sequence
        .steps
        .iter()
        .find(|step| step.id == "open-socket")
        .unwrap();
    assert_eq!(
        open_socket.expect_urc.as_deref(),
        Some("+QIOPEN: {{connect_id}},0")
    );
    assert_eq!(open_socket.timeout_secs, Some(150));
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n\r\n+QIOPEN: 0,0\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\nSEND OK\r\n".to_vec(),
        b"\r\n+QISEND: 4,4,0\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QIRD: 0\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
    ]);
    let params = [
        ("context_id", "1"),
        ("connect_id", "0"),
        ("host", "192.0.2.1"),
        ("port", "8009"),
        ("payload", "ping"),
        ("read_length", "1500"),
    ]
    .into_iter()
    .map(|(name, value)| SequenceParamValue {
        name: name.to_owned(),
        value: value.to_owned(),
    })
    .collect::<Vec<_>>();
    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(execution.masked_transcript.contains("+QIOPEN: 0,0"));
    assert!(execution.raw_transcript.contains("> AT+QIACT?"));
    assert!(!execution.raw_transcript.contains("> AT+QIACT=1"));
    assert!(
        execution
            .masked_transcript
            .contains("Quectel PDP context 1 is already active")
    );
    assert!(execution.masked_transcript.contains("SEND OK"));
    assert!(
        execution
            .masked_transcript
            .contains("TCP send counters: total=4 acknowledged=4 unacknowledged=0")
    );
    assert!(
        execution
            .masked_transcript
            .contains("Quectel receive data: no buffered response data")
    );
    assert!(execution.masked_transcript.contains("Analysis:"));
    assert!(!execution.masked_transcript.contains("Evidence:"));
    assert!(!execution.masked_transcript.contains("ping"));
}

#[test]
fn generic_qisend_counters_are_not_labelled_as_tcp_without_step_contract() {
    let sequence = single_command_sequence(
        "generic-qisend-counters",
        "AT+QISEND=0,0",
        "+QISEND:",
        false,
    );
    let transport = MockTransport::with_responses([b"\r\n+QISEND: 4,4,0\r\n\r\nOK\r\n".to_vec()]);

    let execution = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(execution.raw_transcript.contains("+QISEND: 4,4,0"));
    assert!(!execution.raw_transcript.contains("TCP send counters:"));
    assert!(!execution.raw_transcript.contains("TCP-acknowledged"));
}

#[test]
fn qird_counter_query_reports_three_counters_without_masking_terminal_result() {
    let sequence = single_command_sequence("qird-counters", "AT+QIRD=0,0", "+QIRD:", false);
    let transport = MockTransport::with_responses([b"\r\n+QIRD: 100,40,60\r\n\r\nOK\r\n".to_vec()]);

    let execution = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(execution.masked_transcript.contains("+QIRD: 100,40,60"));
    assert!(execution.masked_transcript.contains("OK"));
    assert!(execution.masked_transcript.contains(
        "Quectel receive counters: total=100 read=40 unread=60; this query did not return or consume buffered payload data."
    ));
    assert!(!execution.masked_transcript.contains("<masked qird data>"));
    assert!(!execution.masked_transcript.contains("TCP receive data:"));
}

#[test]
fn qird_positive_read_masks_data_and_reports_command_aware_length() {
    let sequence = single_command_sequence("qird-read", "AT+QIRD=0,4", "+QIRD:", false);
    let transport = MockTransport::with_responses([b"\r\n+QIRD: 4\r\npong\r\n\r\nOK\r\n".to_vec()]);

    let execution = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(execution.raw_transcript.contains("pong"));
    assert!(!execution.masked_transcript.contains("pong"));
    assert!(execution.masked_transcript.contains("<masked qird data>"));
    assert!(
        execution
            .masked_transcript
            .contains("Quectel receive data: 4 byte(s)")
    );
    assert!(!execution.masked_transcript.contains("TCP receive data:"));
}

#[test]
fn qird_udp_service_header_is_masked_without_tcp_or_counter_analysis() {
    let sequence = single_command_sequence("qird-udp-service", "AT+QIRD=0", "+QIRD:", false);
    let transport = MockTransport::with_responses([
        b"\r\n+QIRD: 4,\"192.0.2.1\",1234\r\npong\r\n\r\nOK\r\n".to_vec(),
    ]);

    let execution = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(
        execution
            .raw_transcript
            .contains("+QIRD: 4,\"192.0.2.1\",1234")
    );
    assert!(execution.raw_transcript.contains("pong"));
    assert!(!execution.masked_transcript.contains("pong"));
    assert!(execution.masked_transcript.contains("<masked qird data>"));
    assert!(!execution.masked_transcript.contains("TCP receive data:"));
    assert!(!execution.masked_transcript.contains("receive counters:"));
}

#[test]
fn quectel_sequence_retries_until_tcp_ack_is_complete() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n\r\n+QIOPEN: 0,0\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\nSEND OK\r\n".to_vec(),
        b"\r\n+QISEND: 4,0,4\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QISEND: 4,4,0\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QIRD: 0\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
    ]);
    let execution = execute_sequence(
        &sequence,
        &tcp_params(),
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert!(
        execution
            .raw_transcript
            .contains("TCP send counters: total=4 acknowledged=0 unacknowledged=4")
    );
    assert!(
        execution
            .raw_transcript
            .contains("TCP send counters: total=4 acknowledged=4 unacknowledged=0")
    );
    assert_eq!(
        execution
            .raw_transcript
            .matches("Command:\n> AT+QISEND=0,0")
            .count(),
        2
    );
}

#[test]
fn quectel_sequence_fails_when_tcp_ack_remains_incomplete() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n\r\n+QIOPEN: 0,0\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\nSEND OK\r\n".to_vec(),
        b"\r\n+QISEND: 4,0,4\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
    ]);
    let execution = execute_sequence(
        &sequence,
        &tcp_params(),
        transport,
        Duration::from_secs(sequence.timeout_secs.expect("Sequence timeout")),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Error);
    assert!(
        execution
            .raw_transcript
            .contains("TCP acknowledgement incomplete for payload_len=4")
    );
    assert!(
        execution
            .raw_transcript
            .contains("Result: failed duration=")
    );
    assert!(
        execution
            .raw_transcript
            .contains("Cleanup after Open TCP socket")
    );
    assert!(execution.raw_transcript.contains("> AT+QICLOSE=0"));
}

#[test]
fn quectel_ping_sequence_reports_received_replies() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "quectel-ping-check")
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
        b"\r\n+QPING: 0,\"93.184.216.34\",32,120,255\r\n\r\n+QPING: 0,4,1,3,120,120,120\r\n\r\nOK\r\n"
            .to_vec(),
    ]);
    let params = vec![SequenceParamValue {
        name: "host".to_owned(),
        value: "example.com".to_owned(),
    }];

    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert!(
        execution
            .raw_transcript
            .contains("> AT+QPING=1,\"example.com\",4,4")
    );
    assert!(
        execution
            .raw_transcript
            .contains("Ping reply: host=93.184.216.34 bytes=32 time_ms=120 ttl=255")
    );
    assert!(
        execution
            .raw_transcript
            .contains("Ping summary: sent=4 received=1 lost=3")
    );
}

#[test]
fn quectel_ping_sequence_fails_without_received_replies() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "quectel-ping-check")
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QPING: 0,4,0,4,0,0,0\r\n\r\nOK\r\n".to_vec(),
    ]);
    let params = vec![SequenceParamValue {
        name: "host".to_owned(),
        value: "example.com".to_owned(),
    }];

    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Error);
    assert!(
        execution
            .raw_transcript
            .contains("ping received no replies sent=4 received=0 lost=4")
    );
    assert!(
        execution
            .raw_transcript
            .contains("Result: failed duration=")
    );
}

#[test]
fn soracom_ping_sequence_uses_ping_response_service() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-ping-check")
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QPING: 0,\"100.127.100.127\",32,88,255\r\n\r\n+QPING: 0,4,1,3,88,88,88\r\n\r\nOK\r\n"
            .to_vec(),
    ]);

    let execution = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert!(
        execution
            .raw_transcript
            .contains("> AT+QPING=1,\"pong.soracom.io\",4,4")
    );
    assert!(!execution.raw_transcript.contains("beam.soracom.io"));
}

#[test]
fn quectel_sequence_activates_inactive_pdp_context_before_socket_open() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,0,1,\"0.0.0.0\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n\r\n+QIOPEN: 0,0\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\nSEND OK\r\n".to_vec(),
        b"\r\n+QISEND: 4,4,0\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QIRD: 0\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
    ]);
    let execution = execute_sequence(
        &sequence,
        &tcp_params(),
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert!(execution.raw_transcript.contains("> AT+QIACT?"));
    assert!(execution.raw_transcript.contains("> AT+QIACT=1"));
    assert!(
        execution
            .masked_transcript
            .contains("AT+QIACT=1 activated it")
    );
}

#[test]
fn quectel_sequence_cleans_up_open_socket_after_later_failure() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n\r\n+QIOPEN: 0,0\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\nERROR\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
    ]);
    let execution = execute_sequence(
        &sequence,
        &tcp_params(),
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Error);
    assert!(execution.raw_transcript.contains("Step 4/7 Write payload"));
    assert!(execution.raw_transcript.contains("Modem response:\nERROR"));
    assert!(
        execution
            .raw_transcript
            .contains("Cleanup after Open TCP socket")
    );
    assert!(execution.raw_transcript.contains("> AT+QICLOSE=0"));
    assert!(
        execution
            .raw_transcript
            .contains("Result: failed duration=")
    );
    assert!(
        execution
            .raw_transcript
            .contains("step `write-payload` did not produce expected marker `SEND OK`")
    );
    assert!(
        !execution
            .raw_transcript
            .contains("SEND OK means the Quectel module accepted")
    );
}

#[test]
fn quectel_sequence_surfaces_socket_open_terminal_error_in_transcript() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nERROR\r\n".to_vec(),
    ]);
    let execution = execute_sequence(
        &sequence,
        &tcp_params(),
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert_eq!(execution.status, AtStatus::Error);
    assert!(
        execution
            .raw_transcript
            .contains("Step 2/7 Open TCP socket")
    );
    assert!(execution.raw_transcript.contains("Modem response:\nERROR"));
    assert!(
        execution
            .raw_transcript
            .contains("step `open-socket` did not produce expected marker `+QIOPEN: 0,0`")
    );
    assert!(!execution.raw_transcript.contains("failed before response"));
}

#[test]
fn ucs2_hex_decoder_rejects_incomplete_code_unit() {
    assert_eq!(decode_ucs2_hex_body("004100"), None);
}

#[test]
fn sms_read_sequence_decodes_ucs2_body_and_masks_normal_output() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-read-message")
        .unwrap();
    let transport = MockTransport::with_responses([
            b"\r\nOK\r\n".to_vec(),
            b"\r\n+CMGR: \"REC READ\",\"901001\",,\"26/06/23,06:03:02+00\"\r\n00680065006C006C006F002100200073006F007200610063006F006D\r\n\r\nOK\r\n"
                .to_vec(),
        ]);
    let params = vec![SequenceParamValue {
        name: "index".to_owned(),
        value: "3".to_owned(),
    }];
    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(
        execution
            .raw_transcript
            .contains("SMS body (ucs2): hello! soracom")
    );
    assert!(execution.raw_transcript.contains("Decoded SMS:"));
    assert!(!execution.raw_transcript.contains("Decoded SMS body"));
    assert!(!execution.raw_transcript.contains("00680065006C006C"));
    assert!(
        execution
            .masked_transcript
            .contains("SMS body (ucs2): <masked sensitive body>")
    );
    assert!(execution.masked_transcript.contains("Decoded SMS:"));
    assert!(!execution.masked_transcript.contains("Evidence:"));
    assert!(!execution.masked_transcript.contains("hello! soracom"));
    assert!(!execution.masked_transcript.contains("00680065006C006C"));
}

#[test]
fn sms_receive_sequence_decodes_listed_bodies_in_unmasked_transcript() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-receive-check")
        .unwrap();
    let transport = MockTransport::with_responses([
            b"\r\nOK\r\n".to_vec(),
            b"\r\n+CMGL: 3,\"REC UNREAD\",\"901001\",,\"26/06/23,06:03:02+00\"\r\n00680065006C006C006F002100200073006F007200610063006F006D\r\n\r\nOK\r\n"
                .to_vec(),
        ]);
    let execution = execute_sequence(
        &sequence,
        &[],
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(
        execution
            .raw_transcript
            .contains("SMS body (ucs2): hello! soracom")
    );
    assert!(!execution.raw_transcript.contains("00680065006C006C"));
    assert!(
        execution
            .masked_transcript
            .contains("SMS body (ucs2): <masked sensitive body>")
    );
    assert!(execution.masked_transcript.contains("Decoded SMS:"));
    assert!(!execution.masked_transcript.contains("Evidence:"));
    assert!(!execution.masked_transcript.contains("hello! soracom"));
    assert!(!execution.masked_transcript.contains("00680065006C006C"));
    let candidate_set = execution
        .value_candidate_sets
        .iter()
        .find(|set| set.candidate == SequenceCandidateSource::SmsMessage)
        .expect("sms candidate set");
    assert_eq!(candidate_set.candidates.len(), 1);
    let candidate = &candidate_set.candidates[0];
    assert_eq!(candidate.value, "3");
    assert!(candidate.raw_label.contains("REC UNREAD"));
    assert!(candidate.raw_label.contains("901001"));
    assert!(candidate.raw_label.contains("26/06/23,06:03:02+00"));
    assert!(candidate.raw_label.contains("hello! soracom"));
    assert!(candidate.masked_label.contains("******"));
    assert!(candidate.masked_label.contains("<masked sensitive body>"));
}

#[test]
fn value_candidate_sets_extract_sms_and_standard_pdp_candidates() {
    let text = concat!(
        "AT+CGACT?\r\n",
        "+CGACT: 1,1\r\n",
        "+CGACT: 2,0\r\n",
        "AT+CGDCONT?\r\n",
        "+CGDCONT: 1,\"IP\",\"soracom.io\",\"0.0.0.0\",0,0,0,0\r\n",
        "OK\r\n",
    );

    let sets = value_candidate_sets_from_text(text);
    let pdp = sets
        .iter()
        .find(|set| set.candidate == SequenceCandidateSource::PdpContext)
        .expect("pdp candidates");
    assert!(pdp.candidates.iter().any(|candidate| {
        candidate.value == "1"
            && candidate.raw_label.contains("active")
            && candidate.raw_label.contains("soracom.io")
    }));
    assert!(
        pdp.candidates.iter().any(|candidate| {
            candidate.value == "2" && candidate.raw_label.contains("inactive")
        })
    );
}

#[test]
fn sms_reply_sequence_reads_sender_before_submit() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-reply-check")
        .unwrap();
    let transport = MockTransport::with_responses([
            b"\r\nOK\r\n".to_vec(),
            b"\r\n+CMGR: \"REC READ\",\"901001\",,\"26/06/23,06:03:02+00\"\r\n0074006500730074\r\n\r\nOK\r\n"
                .to_vec(),
            b"\r\n> ".to_vec(),
            b"\r\n+CMGS: 12\r\n\r\nOK\r\n".to_vec(),
        ]);
    let params = vec![
        SequenceParamValue {
            name: "index".to_owned(),
            value: "3".to_owned(),
        },
        SequenceParamValue {
            name: "message".to_owned(),
            value: "reply text".to_owned(),
        },
    ];
    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(execution.raw_transcript.contains("> AT+CMGR=3"));
    assert!(execution.raw_transcript.contains("> AT+CMGS=\"901001\""));
    assert!(!execution.masked_transcript.contains("> AT+CMGS=\"901001\""));
    assert!(
        execution
            .masked_transcript
            .contains("Reply recipient is the sender extracted")
    );
}

#[test]
fn missing_required_param_is_rejected_before_transport() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-send-check")
        .unwrap();

    let error = validate_and_bind_params(&sequence, &[]).unwrap_err();

    assert!(matches!(
        &error,
        AtctlError::MissingSequenceParam { param, .. } if param == "recipient"
    ));
    assert!(
        error
            .to_string()
            .contains("help: rerun with `--param recipient=<VALUE>`")
    );
}

#[test]
fn missing_select_param_includes_resolution_hint() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-read-message")
        .unwrap();

    let error = validate_and_bind_params(&sequence, &[]).unwrap_err();

    assert!(error.to_string().contains("sms-receive-check"));
    assert!(error.to_string().contains("AT+CMGL"));
    assert!(
        error
            .to_string()
            .contains("providers: Sequence `sms-receive-check`")
    );
    assert!(
        error
            .to_string()
            .contains("help: rerun with `--param index=<VALUE>`")
    );
}

#[test]
fn product_candidate_sources_reference_existing_sequence_and_presets() {
    let sms = SequenceCandidateSource::SmsMessage.providers();
    assert_eq!(sms.len(), 1);
    assert_eq!(
        sms[0].kind,
        crate::sequences::model::SequenceCandidateProviderKind::Sequence
    );
    assert_eq!(sms[0].name, "sms-receive-check");

    let pdp = SequenceCandidateSource::PdpContext.providers();
    assert_eq!(
        pdp.iter().map(|provider| provider.name).collect::<Vec<_>>(),
        vec!["active-pdp-contexts", "pdp-contexts"]
    );
    assert!(pdp.iter().all(|provider| {
        provider.kind == crate::sequences::model::SequenceCandidateProviderKind::Preset
    }));
}

#[test]
fn sequence_defaults_are_bound_before_review_and_execution() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-unified-endpoint-tcp-send-check")
    .unwrap();
    let params = vec![SequenceParamValue {
        name: "payload".to_owned(),
        value: "hello".to_owned(),
    }];

    let bindings = validate_and_bind_params(&sequence, &params).unwrap();

    assert_eq!(bindings.get("context_id").unwrap().value, "1");
    assert_eq!(bindings.get("connect_id").unwrap().value, "0");
    assert_eq!(bindings.get("read_length").unwrap().value, "1500");
    let review = render_sequence_review(&sequence, &params).unwrap();
    assert!(
        review.iter().any(|item| {
            item.label == "Destination" && item.value == "unified.soracom.io:23080"
        })
    );
    assert!(
        review
            .iter()
            .any(|item| item.label == "Read length" && item.value == "1500")
    );
}

#[test]
fn sequence_execution_review_uses_bound_command_values() {
    let sequence = Sequence::built_in(
        "dynamic-functionality",
        "Set the requested modem functionality.",
        vec!["test".to_owned()],
        Some(30),
        vec![SequenceParam {
            name: "mode".to_owned(),
            label: "Mode".to_owned(),
            required: true,
            sensitive: false,
            default_value: None,
            source: crate::sequences::model::SequenceParamSource::User,
            candidate: None,
            hint: None,
        }],
        vec![SequenceStep {
            id: "set-mode".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some("AT+CFUN={{mode}}".to_owned()),
            expect: Some("OK".to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack: false,
            require_ping_success: false,
            timeout_secs: None,
            evidence: None,
            cleanup_on_failure: None,
        }],
    );
    let params = vec![SequenceParamValue {
        name: "mode".to_owned(),
        value: "1,1".to_owned(),
    }];

    assert!(
        !sequence
            .review
            .effects
            .contains(&crate::at::review::Effect::DeleteResetOrRecover)
    );
    let review = review_sequence_execution(&sequence, &params).unwrap();
    assert!(
        review
            .effects
            .contains(&crate::at::review::Effect::DeleteResetOrRecover)
    );

    let execution = execute_sequence(
        &sequence,
        &params,
        MockTransport::with_response(b"\r\nOK\r\n".to_vec()),
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();
    assert_eq!(execution.review, review);
}

#[test]
fn sensitive_template_positions_are_masked_without_blanket_short_number_masking() {
    let mut sequence = Sequence::built_in(
        "sensitive-template",
        "Exercise sensitive template provenance.",
        vec!["test".to_owned()],
        Some(30),
        vec![SequenceParam {
            name: "token".to_owned(),
            label: "Token".to_owned(),
            required: true,
            sensitive: true,
            default_value: None,
            source: crate::sequences::model::SequenceParamSource::User,
            candidate: None,
            hint: None,
        }],
        vec![SequenceStep {
            id: "send-token".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some("AT+VENDOR={{token}},7".to_owned()),
            expect: Some("OK".to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack: false,
            require_ping_success: false,
            timeout_secs: None,
            evidence: None,
            cleanup_on_failure: None,
        }],
    );
    sequence.success_notes = vec!["credential={{token}} result=7".to_owned()];
    let params = [SequenceParamValue {
        name: "token".to_owned(),
        value: "901001".to_owned(),
    }];
    let transport =
        MockTransport::with_response(b"AT+VENDOR=901001,7\r\n+VENDOR: 1,7\r\nOK\r\n".to_vec());

    let execution = execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        None,
    )
    .unwrap();

    assert!(execution.raw_transcript.contains("AT+VENDOR=901001,7"));
    assert!(execution.masked_transcript.contains("AT+VENDOR=******,7"));
    assert!(
        execution
            .masked_transcript
            .contains("credential=****** result=7")
    );
    assert!(execution.masked_transcript.contains("+VENDOR: 1,7"));
    assert!(!execution.masked_transcript.contains("901001"));
}

#[test]
fn sequence_execution_review_rejects_bound_command_framing_before_transport() {
    let mut sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-read-message")
        .unwrap();
    sequence.steps[0].send = Some("AT+CMGR={{index}}".to_owned());
    let params = vec![SequenceParamValue {
        name: "index".to_owned(),
        value: "1\rAT+CFUN=1,1".to_owned(),
    }];

    assert!(matches!(
        review_sequence_execution(&sequence, &params),
        Err(AtctlError::OperationOutsideReviewedBoundary { .. })
    ));
}

#[test]
fn sequence_execution_review_keeps_runtime_derived_command_boundaries() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-reply-check")
        .unwrap();
    let params = vec![
        SequenceParamValue {
            name: "index".to_owned(),
            value: "7".to_owned(),
        },
        SequenceParamValue {
            name: "message".to_owned(),
            value: "reply".to_owned(),
        },
    ];

    let review = review_sequence_execution(&sequence, &params).unwrap();

    assert_eq!(review.assessment, Assessment::Known);
    assert!(
        review
            .effects
            .contains(&crate::at::review::Effect::CommunicateExternally)
    );
}

#[test]
fn raw_log_records_each_sequence_exchange() {
    let sequence = builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-send-check")
        .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\nOK\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\n+CMGS: 12\r\n\r\nOK\r\n".to_vec(),
    ]);
    let params = vec![
        SequenceParamValue {
            name: "recipient".to_owned(),
            value: "+819012345678".to_owned(),
        },
        SequenceParamValue {
            name: "message".to_owned(),
            value: "hello from atctl".to_owned(),
        },
    ];
    let path = unique_temp_dir("sequence-raw").join("case.rawlog");
    let mut raw_log =
        RawLogSink::create(RawLogConfig::new(path.clone(), ExecutionSurface::Cli)).unwrap();
    let _ = RAW_LOG_ACK;
    execute_sequence(
        &sequence,
        &params,
        transport,
        Duration::from_secs(30),
        true,
        Some(&mut raw_log),
    )
    .unwrap();

    let raw = fs::read_to_string(path).unwrap();
    assert!(raw.contains("\"event\":\"exchange\""));
    assert!(raw.contains("\"operation\":{\"kind\":\"sequence\""));
    assert!(raw.contains("\"name\":\"sms-send-check\""));
    assert!(!raw.contains("\"source\""));
    assert!(raw.contains("\"command_name\":\"set-text-mode\""));
    assert!(raw.contains("\"command_name\":\"write-message\""));
    assert!(raw.contains("hello from atctl"));
}

#[test]
fn tcp_sequence_fixed_length_payload_does_not_append_ctrl_z() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../../examples/sequences/quectel.toml"
    ))
    .unwrap()
    .into_iter()
    .next()
    .unwrap();
    let transport = MockTransport::with_responses([
        b"\r\n+QIACT: 1,1,1,\"10.0.0.1\"\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n\r\n+QIOPEN: 0,0\r\n".to_vec(),
        b"\r\n> ".to_vec(),
        b"\r\nSEND OK\r\n".to_vec(),
        b"\r\n+QISEND: 4,4,0\r\n\r\nOK\r\n".to_vec(),
        b"\r\n+QIRD: 0\r\n\r\nOK\r\n".to_vec(),
        b"\r\nOK\r\n".to_vec(),
    ]);
    let path = unique_temp_dir("tcp-raw").join("case.rawlog");
    let mut raw_log =
        RawLogSink::create(RawLogConfig::new(path.clone(), ExecutionSurface::Cli)).unwrap();
    execute_sequence(
        &sequence,
        &tcp_params(),
        transport,
        Duration::from_secs(30),
        true,
        Some(&mut raw_log),
    )
    .unwrap();

    let raw = fs::read_to_string(path).unwrap();
    assert!(raw.contains("\"command_name\":\"write-payload\""));
    assert!(raw.contains("\"tx_preview\":\"ping\""));
    assert!(!raw.contains("\\x1A"));
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("atctl-{name}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn tcp_params() -> Vec<SequenceParamValue> {
    [
        ("context_id", "1"),
        ("connect_id", "0"),
        ("host", "192.0.2.1"),
        ("port", "8009"),
        ("payload", "ping"),
        ("read_length", "1500"),
    ]
    .into_iter()
    .map(|(name, value)| SequenceParamValue {
        name: name.to_owned(),
        value: value.to_owned(),
    })
    .collect()
}

fn single_command_sequence(
    name: &str,
    command: &str,
    expect: &str,
    require_tcp_ack: bool,
) -> Sequence {
    Sequence::built_in(
        name,
        "Exercise one command.",
        vec!["test".to_owned()],
        None,
        Vec::new(),
        vec![SequenceStep {
            id: "command".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some(command.to_owned()),
            expect: Some(expect.to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack,
            require_ping_success: false,
            timeout_secs: None,
            evidence: None,
            cleanup_on_failure: None,
        }],
    )
}
