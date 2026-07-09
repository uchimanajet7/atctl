use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::log::raw::{RAW_LOG_ACK, RawLogConfig};
use crate::sequences::builtin::builtins;
use crate::transport::test_support::MockTransport;

use super::*;

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
    assert!(execution.masked_transcript.contains("+819*******78"));
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
            .contains("TCP receive data: no buffered response data")
    );
    assert!(execution.masked_transcript.contains("Analysis:"));
    assert!(!execution.masked_transcript.contains("Evidence:"));
    assert!(!execution.masked_transcript.contains("ping"));
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
        Duration::from_secs(1),
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
    assert!(candidate.masked_label.contains("90****"));
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
        error,
        AtctlError::MissingSequenceParam { param, .. } if param == "recipient"
    ));
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
        RawLogSink::create(RawLogConfig::new(path.clone(), "cli", "sequence:sms")).unwrap();
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
        RawLogSink::create(RawLogConfig::new(path.clone(), "cli", "sequence:tcp")).unwrap();
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
