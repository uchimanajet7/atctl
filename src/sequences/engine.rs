use std::collections::{BTreeMap, BTreeSet};
use std::thread;
use std::time::{Duration, Instant};

use crate::at::command::command_with_terminator;
use crate::at::mask::{mask_identifier, mask_sensitive_values};
use crate::at::parser::parse_response;
use crate::at::response::AtStatus;
use crate::at::risk::{
    DirectSendConfirmation, RiskClassification, RiskLevel, direct_send_confirmation,
};
use crate::log::raw::{RawLogExchange, RawLogSink, RawLogTransportError};
use crate::sequences::model::{
    Sequence, SequenceCandidateSource, SequenceParam, SequenceStep, StepTerminator,
};
use crate::transport::traits::{AtTransport, ResponseMatcher};
use crate::{AtctlError, Result};

const TCP_ACK_RETRY_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceParamValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceReviewValue {
    pub label: String,
    pub value: String,
    pub sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceStepResult {
    pub id: String,
    pub label: Option<String>,
    pub status: AtStatus,
    pub masked_analysis: Option<String>,
    pub raw_analysis: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsMessageCandidate {
    pub index: String,
    pub status: Option<String>,
    pub sender: Option<String>,
    pub masked_sender: Option<String>,
    pub timestamp: Option<String>,
    pub raw_body_preview: Option<String>,
    pub masked_body_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceValueCandidate {
    pub value: String,
    pub raw_label: String,
    pub masked_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceValueCandidateSet {
    pub candidate: SequenceCandidateSource,
    pub candidates: Vec<SequenceValueCandidate>,
}

#[derive(Debug, Clone)]
pub struct SequenceExecution {
    pub name: String,
    pub risk: RiskLevel,
    pub status: AtStatus,
    pub steps: Vec<SequenceStepResult>,
    pub masked_notes: Vec<String>,
    pub raw_notes: Vec<String>,
    pub value_candidate_sets: Vec<SequenceValueCandidateSet>,
    pub masked_transcript: String,
    pub raw_transcript: String,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StepExecutionOutcome {
    status: AtStatus,
    response_text: String,
    expectation_failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnsurePdpContextOutcome {
    status: AtStatus,
    response_texts: Vec<String>,
    analysis: Vec<SequenceAnalysisLine>,
    expectation_failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredCleanup {
    id: String,
    label: String,
    command: String,
}

pub fn execute_sequence<T>(
    sequence: &Sequence,
    params: &[SequenceParamValue],
    mut transport: T,
    timeout: Duration,
    _masked: bool,
    mut raw_log: Option<&mut RawLogSink>,
) -> Result<SequenceExecution>
where
    T: AtTransport,
{
    let mut bindings = validate_and_bind_params(sequence, params)?;
    let started = Instant::now();
    let total_timeout = sequence
        .timeout_secs
        .map(Duration::from_secs)
        .unwrap_or(timeout)
        .max(Duration::from_secs(1));
    let deadline = started + total_timeout;
    let mut raw_transcript = Vec::new();
    let mut masked_transcript = Vec::new();
    let mut step_results = Vec::new();
    let mut value_candidate_sets = Vec::new();
    let mut deferred_cleanups = Vec::new();

    transport.open()?;
    for (index, step) in sequence.steps.iter().enumerate() {
        push_transcript_header(
            &mut raw_transcript,
            index,
            sequence.steps.len(),
            step,
            false,
            &bindings,
        );
        push_transcript_header(
            &mut masked_transcript,
            index,
            sequence.steps.len(),
            step,
            true,
            &bindings,
        );

        let mut status = AtStatus::Ok;
        let mut response_texts = Vec::new();
        let mut extra_analysis = Vec::new();
        let mut failure_reason = None;
        let mut rendered_send = None;

        if let Some(template) = &step.ensure_pdp_context_active {
            let outcome = execute_ensure_pdp_context_active(
                sequence,
                step,
                template,
                &mut transport,
                step_timeout(step, timeout, deadline)?,
                raw_log.as_deref_mut(),
                &mut raw_transcript,
                &mut masked_transcript,
                &bindings,
            )?;
            status = outcome.status;
            response_texts.extend(outcome.response_texts);
            extra_analysis.extend(outcome.analysis);
            failure_reason =
                step_failure_reason(sequence, step, &status, outcome.expectation_failure);
        }

        if failure_reason.is_none()
            && let Some(template) = &step.send
        {
            let command = render_template(template, &bindings, sequence, &step.id)?;
            rendered_send = Some(command.clone());
            let outcome = execute_command_step(
                sequence,
                step,
                &command,
                &mut transport,
                step_timeout(step, timeout, deadline)?,
                raw_log.as_deref_mut(),
                &mut raw_transcript,
                &mut masked_transcript,
                &bindings,
            )?;
            status = outcome.status;
            response_texts.push(outcome.response_text);
            failure_reason =
                step_failure_reason(sequence, step, &status, outcome.expectation_failure);
        }

        if failure_reason.is_none()
            && let Some(template) = &step.payload
        {
            let payload = render_template(template, &bindings, sequence, &step.id)?;
            let outcome = execute_payload_step(
                sequence,
                step,
                &payload,
                &mut transport,
                step_timeout(step, timeout, deadline)?,
                raw_log.as_deref_mut(),
                &mut raw_transcript,
                &mut masked_transcript,
                &bindings,
            )?;
            status = outcome.status;
            response_texts.push(outcome.response_text);
            failure_reason =
                step_failure_reason(sequence, step, &status, outcome.expectation_failure);
        }

        let (raw_analysis, masked_analysis) = push_step_analysis(
            sequence,
            step,
            &response_texts,
            extra_analysis,
            failure_reason.is_none(),
            &mut raw_transcript,
            &mut masked_transcript,
            &mut bindings,
        )?;
        value_candidate_sets.extend(value_candidate_sets_from_responses(&response_texts));
        step_results.push(SequenceStepResult {
            id: step.id.clone(),
            label: step.label.clone(),
            status: status.clone(),
            masked_analysis,
            raw_analysis,
        });

        if let Some(reason) = failure_reason {
            run_deferred_cleanups(
                sequence,
                &deferred_cleanups,
                &mut transport,
                cleanup_timeout(deadline),
                raw_log.as_deref_mut(),
                &mut raw_transcript,
                &mut masked_transcript,
                &bindings,
            )?;
            let close_result = transport.close();
            let duration = started.elapsed();
            push_failure_result(&mut raw_transcript, duration, &reason);
            push_failure_result(
                &mut masked_transcript,
                duration,
                &mask_sequence_text(&reason, &bindings),
            );
            close_result?;
            return Ok(SequenceExecution {
                name: sequence.name.clone(),
                risk: sequence.risk,
                status,
                steps: step_results,
                masked_notes: Vec::new(),
                raw_notes: Vec::new(),
                value_candidate_sets,
                masked_transcript: masked_transcript.join("\n"),
                raw_transcript: raw_transcript.join("\n"),
                duration,
            });
        }

        if let Some(command) = &rendered_send {
            deferred_cleanups.retain(|cleanup: &DeferredCleanup| cleanup.command != *command);
        }
        if let Some(template) = &step.cleanup_on_failure {
            let command = render_template(template, &bindings, sequence, &step.id)?;
            deferred_cleanups.push(DeferredCleanup {
                id: format!("cleanup-{}", step.id),
                label: format!(
                    "Cleanup after {}",
                    step.label.as_deref().unwrap_or(&step.id)
                ),
                command,
            });
        }
    }

    let close_result = transport.close();
    let duration = started.elapsed();
    let (raw_notes, masked_notes) = push_success_notes(
        sequence,
        &bindings,
        &mut raw_transcript,
        &mut masked_transcript,
    )?;
    push_transcript_line_block(
        &mut raw_transcript,
        format!("Result: OK duration={}ms", duration.as_millis()),
    );
    push_transcript_line_block(
        &mut masked_transcript,
        format!("Result: OK duration={}ms", duration.as_millis()),
    );
    close_result?;

    Ok(SequenceExecution {
        name: sequence.name.clone(),
        risk: sequence.risk,
        status: AtStatus::Ok,
        steps: step_results,
        masked_notes,
        raw_notes,
        value_candidate_sets,
        masked_transcript: masked_transcript.join("\n"),
        raw_transcript: raw_transcript.join("\n"),
        duration,
    })
}

pub fn validate_sequence_confirmation(
    sequence: &Sequence,
    yes: bool,
    acknowledged: Option<RiskLevel>,
) -> Result<DirectSendConfirmation> {
    direct_send_confirmation(&sequence_classification(sequence), yes, acknowledged)
}

pub fn sequence_classification(sequence: &Sequence) -> RiskClassification {
    RiskClassification {
        normalized_command: format!("SEQUENCE {}", sequence.name),
        risk: sequence.risk,
        reason: "sequence effective risk level",
    }
}

pub fn render_sequence_review(
    sequence: &Sequence,
    values: &[SequenceParamValue],
) -> Result<Vec<SequenceReviewValue>> {
    let bindings = validate_and_bind_params(sequence, values)?;
    if sequence.review_items.is_empty() {
        return Ok(sequence
            .params
            .iter()
            .filter_map(|param| {
                bindings.get(&param.name).map(|value| SequenceReviewValue {
                    label: param.label.clone(),
                    value: value.value.clone(),
                    sensitive: param.sensitive,
                })
            })
            .collect());
    }

    sequence
        .review_items
        .iter()
        .map(|item| {
            Ok(SequenceReviewValue {
                label: item.label.clone(),
                value: render_template(&item.value, &bindings, sequence, "review")?,
                sensitive: item.sensitive,
            })
        })
        .collect()
}

pub fn validate_and_bind_params(
    sequence: &Sequence,
    values: &[SequenceParamValue],
) -> Result<BTreeMap<String, BoundParam>> {
    let mut bound = BTreeMap::new();
    for value in values {
        if !sequence.params.iter().any(|param| param.name == value.name) {
            return Err(AtctlError::InvalidSequenceParam {
                param: value.name.clone(),
                reason: format!(
                    "sequence `{}` does not define this parameter",
                    sequence.name
                ),
            });
        }
        bound.insert(
            value.name.clone(),
            BoundParam {
                value: value.value.clone(),
                sensitive: sequence
                    .params
                    .iter()
                    .find(|param| param.name == value.name)
                    .is_some_and(|param| param.sensitive),
            },
        );
    }

    for param in &sequence.params {
        if !bound.contains_key(&param.name)
            && let Some(default_value) = &param.default_value
        {
            bound.insert(
                param.name.clone(),
                BoundParam {
                    value: default_value.clone(),
                    sensitive: param.sensitive,
                },
            );
        }
        if param.required
            && bound
                .get(&param.name)
                .is_none_or(|value| value.value.is_empty())
        {
            return Err(AtctlError::MissingSequenceParam {
                sequence: sequence.name.clone(),
                param: param.name.clone(),
                hint: missing_param_hint_suffix(param),
            });
        }
    }

    let derived = bound
        .iter()
        .map(|(name, value)| {
            (
                format!("{name}_len"),
                BoundParam {
                    value: value.value.len().to_string(),
                    sensitive: false,
                },
            )
        })
        .collect::<Vec<_>>();
    bound.extend(derived);
    Ok(bound)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundParam {
    pub value: String,
    pub sensitive: bool,
}

#[allow(clippy::too_many_arguments)]
fn execute_command_step<T>(
    sequence: &Sequence,
    step: &SequenceStep,
    command: &str,
    transport: &mut T,
    timeout: Duration,
    mut raw_log: Option<&mut RawLogSink>,
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    bindings: &BTreeMap<String, BoundParam>,
) -> Result<StepExecutionOutcome>
where
    T: AtTransport,
{
    let deadline = Instant::now() + timeout;
    let mut response_texts = Vec::new();
    let mut last_tcp_ack_failure = None;

    let (final_status, final_expectation_failure) = loop {
        push_transcript_section(raw_transcript, "Command:", [format!("> {command}")]);
        push_transcript_section(
            masked_transcript,
            "Command:",
            [format!("> {}", mask_sequence_text(command, bindings))],
        );
        let started = Instant::now();
        let tx = command_with_terminator(command);
        if let Err(error) = transport.write_command(&tx) {
            append_sequence_raw_error(
                raw_log,
                sequence,
                step,
                command,
                started.elapsed(),
                "write_command",
                &error,
                tx.as_bytes(),
            )?;
            return Err(error);
        }

        let matcher = response_matcher_for_step(sequence, step, bindings)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let raw_response =
            match transport.read_until(remaining.max(Duration::from_secs(1)), matcher) {
                Ok(raw) => raw,
                Err(error) => {
                    if step.require_tcp_ack && last_tcp_ack_failure.is_some() {
                        break (AtStatus::Error, last_tcp_ack_failure);
                    }
                    append_sequence_raw_error(
                        raw_log,
                        sequence,
                        step,
                        command,
                        started.elapsed(),
                        "read_response",
                        &error,
                        tx.as_bytes(),
                    )?;
                    return Err(error);
                }
            };
        let response = parse_response(&raw_response);
        let duration = started.elapsed();
        push_response_transcript(raw_transcript, masked_transcript, &response.text, bindings);
        let mut expectation_failure =
            step_expectation_failure(sequence, step, &response.text, bindings)?;
        let mut status = step_status(sequence, step, bindings, &response.text, &response.status)?;
        if expectation_failure.is_none() && step.require_tcp_ack {
            let ack_failure = tcp_ack_requirement_failure(&response.text, bindings);
            if ack_failure.is_some() {
                last_tcp_ack_failure = ack_failure.clone();
            }
            expectation_failure = ack_failure;
        }
        if expectation_failure.is_none() && step.require_ping_success {
            expectation_failure = ping_success_requirement_failure(&response.text);
        }
        if expectation_failure.is_some() {
            status = AtStatus::Error;
        }
        if step.require_tcp_ack && last_tcp_ack_failure.is_none() {
            last_tcp_ack_failure = expectation_failure.clone();
        }
        if let Some(raw_log) = raw_log.as_deref_mut() {
            raw_log.append_exchange(RawLogExchange {
                command_name: Some(&step.id),
                command,
                risk: sequence.risk,
                status: &status,
                duration,
                tx_bytes: tx.as_bytes(),
                rx_bytes: &response.raw,
            })?;
        }
        response_texts.push(response.text);

        if !step.require_tcp_ack || expectation_failure.is_none() {
            return Ok(StepExecutionOutcome {
                status,
                response_text: response_texts.join("\n"),
                expectation_failure,
            });
        }

        if Instant::now() + TCP_ACK_RETRY_INTERVAL >= deadline {
            break (status, expectation_failure);
        }
        thread::sleep(TCP_ACK_RETRY_INTERVAL);
    };

    Ok(StepExecutionOutcome {
        status: final_status,
        response_text: response_texts.join("\n"),
        expectation_failure: final_expectation_failure,
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_payload_step<T>(
    sequence: &Sequence,
    step: &SequenceStep,
    payload: &str,
    transport: &mut T,
    timeout: Duration,
    raw_log: Option<&mut RawLogSink>,
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    bindings: &BTreeMap<String, BoundParam>,
) -> Result<StepExecutionOutcome>
where
    T: AtTransport,
{
    push_transcript_section(raw_transcript, "Payload:", [format!("> payload {payload}")]);
    push_transcript_section(
        masked_transcript,
        "Payload:",
        [format!(
            "> payload {}",
            mask_sequence_text(payload, bindings)
        )],
    );
    let tx = payload_with_terminator(payload, step.terminator);
    let started = Instant::now();
    if let Err(error) = transport.write_command(&tx) {
        append_sequence_raw_error(
            raw_log,
            sequence,
            step,
            "payload",
            started.elapsed(),
            "write_payload",
            &error,
            tx.as_bytes(),
        )?;
        return Err(error);
    }

    let matcher = response_matcher_for_step(sequence, step, bindings)?;
    let raw_response = match transport.read_until(timeout, matcher) {
        Ok(raw) => raw,
        Err(error) => {
            append_sequence_raw_error(
                raw_log,
                sequence,
                step,
                "payload",
                started.elapsed(),
                "read_payload_response",
                &error,
                tx.as_bytes(),
            )?;
            return Err(error);
        }
    };
    let response = parse_response(&raw_response);
    let duration = started.elapsed();
    push_response_transcript(raw_transcript, masked_transcript, &response.text, bindings);
    let expectation_failure = step_expectation_failure(sequence, step, &response.text, bindings)?;
    let mut status = step_status(sequence, step, bindings, &response.text, &response.status)?;
    if expectation_failure.is_some() {
        status = AtStatus::Error;
    }
    if let Some(raw_log) = raw_log {
        raw_log.append_exchange(RawLogExchange {
            command_name: Some(&step.id),
            command: "payload",
            risk: sequence.risk,
            status: &status,
            duration,
            tx_bytes: tx.as_bytes(),
            rx_bytes: &response.raw,
        })?;
    }
    Ok(StepExecutionOutcome {
        status,
        response_text: response.text,
        expectation_failure,
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_ensure_pdp_context_active<T>(
    sequence: &Sequence,
    step: &SequenceStep,
    context_id_template: &str,
    transport: &mut T,
    timeout: Duration,
    mut raw_log: Option<&mut RawLogSink>,
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    bindings: &BTreeMap<String, BoundParam>,
) -> Result<EnsurePdpContextOutcome>
where
    T: AtTransport,
{
    let context_id = render_template(context_id_template, bindings, sequence, &step.id)?;
    let check_step = internal_command_step(
        format!("{}-check", step.id),
        "Check Quectel PDP context state",
        "OK",
    );
    let check = execute_command_step(
        sequence,
        &check_step,
        "AT+QIACT?",
        transport,
        timeout,
        raw_log.as_deref_mut(),
        raw_transcript,
        masked_transcript,
        bindings,
    )?;
    let mut response_texts = vec![check.response_text];
    let mut status = check.status;
    let mut expectation_failure = check.expectation_failure;
    let mut analysis = Vec::new();

    if expectation_failure.is_none()
        && status.is_success()
        && qiact_context_is_active(&response_texts[0], &context_id)
    {
        let text = format!(
            "Quectel PDP context {context_id} is already active; AT+QIACT={context_id} was not sent."
        );
        analysis.push(SequenceAnalysisLine {
            kind: SequenceAnalysisKind::General,
            raw: text.clone(),
            masked: text,
        });
        return Ok(EnsurePdpContextOutcome {
            status,
            response_texts,
            analysis,
            expectation_failure,
        });
    }

    if expectation_failure.is_none() && status.is_success() {
        let command = format!("AT+QIACT={context_id}");
        let activate_step = internal_command_step(
            format!("{}-activate", step.id),
            "Activate Quectel PDP context",
            "OK",
        );
        let activate = execute_command_step(
            sequence,
            &activate_step,
            &command,
            transport,
            timeout,
            raw_log,
            raw_transcript,
            masked_transcript,
            bindings,
        )?;
        status = activate.status;
        expectation_failure = activate.expectation_failure;
        response_texts.push(activate.response_text);
        if expectation_failure.is_none() && status.is_success() {
            let text = format!(
                "Quectel PDP context {context_id} was not active in AT+QIACT? output; AT+QIACT={context_id} activated it."
            );
            analysis.push(SequenceAnalysisLine {
                kind: SequenceAnalysisKind::General,
                raw: text.clone(),
                masked: text,
            });
        }
    }

    Ok(EnsurePdpContextOutcome {
        status,
        response_texts,
        analysis,
        expectation_failure,
    })
}

fn internal_command_step(id: String, label: &str, expect: &str) -> SequenceStep {
    SequenceStep {
        id,
        label: Some(label.to_owned()),
        ensure_pdp_context_active: None,
        send: None,
        expect: Some(expect.to_owned()),
        expect_prompt: None,
        expect_urc: None,
        payload: None,
        terminator: StepTerminator::None,
        require_tcp_ack: false,
        require_ping_success: false,
        timeout_secs: None,
        evidence: None,
        cleanup_on_failure: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_step_analysis(
    sequence: &Sequence,
    step: &SequenceStep,
    response_texts: &[String],
    extra_analysis: Vec<SequenceAnalysisLine>,
    include_step_evidence: bool,
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    bindings: &mut BTreeMap<String, BoundParam>,
) -> Result<(Option<String>, Option<String>)> {
    let mut analysis = Vec::new();
    if include_step_evidence && let Some(template) = &step.evidence {
        let raw = render_template(template, bindings, sequence, &step.id)?;
        let masked = mask_sequence_text(&raw, bindings);
        analysis.push(SequenceAnalysisLine {
            kind: SequenceAnalysisKind::General,
            raw,
            masked,
        });
    }
    analysis.extend(extra_analysis);
    analysis.extend(derive_response_analysis(response_texts, bindings));

    if analysis.is_empty() {
        return Ok((None, None));
    }

    push_analysis_transcript(raw_transcript, masked_transcript, &analysis);
    Ok((
        Some(
            analysis
                .iter()
                .map(|line| line.raw.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        Some(
            analysis
                .iter()
                .map(|line| line.masked.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    ))
}

fn push_success_notes(
    sequence: &Sequence,
    bindings: &BTreeMap<String, BoundParam>,
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut raw_notes = Vec::new();
    let mut masked_notes = Vec::new();
    for note in &sequence.success_notes {
        let raw = render_template(note, bindings, sequence, "success_notes")?;
        let masked = mask_sequence_text(&raw, bindings);
        raw_notes.push(raw);
        masked_notes.push(masked);
    }
    if !raw_notes.is_empty() {
        push_transcript_section(
            raw_transcript,
            "Notes:",
            raw_notes.iter().map(|note| format!("- {note}")),
        );
        push_transcript_section(
            masked_transcript,
            "Notes:",
            masked_notes.iter().map(|note| format!("- {note}")),
        );
    }
    Ok((raw_notes, masked_notes))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SequenceAnalysisLine {
    kind: SequenceAnalysisKind,
    raw: String,
    masked: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceAnalysisKind {
    General,
    DecodedSms,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SmsMessage {
    index: Option<String>,
    status: Option<String>,
    sender: Option<String>,
    timestamp: Option<String>,
    body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedSmsBody {
    charset: &'static str,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CsvField {
    value: String,
    quoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PingReply {
    host: String,
    bytes: Option<usize>,
    time_ms: Option<usize>,
    ttl: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PingSummary {
    sent: usize,
    received: usize,
    lost: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PingLine {
    Reply(PingReply),
    Summary(PingSummary),
    ResultCode(String),
}

fn derive_response_analysis(
    response_texts: &[String],
    bindings: &mut BTreeMap<String, BoundParam>,
) -> Vec<SequenceAnalysisLine> {
    let mut analysis = Vec::new();
    for response in response_texts {
        let sms_messages = parse_sms_messages(response);
        if let Some(sender) = sms_messages.iter().find_map(|message| {
            message
                .sender
                .as_deref()
                .filter(|sender| !sender.is_empty())
        }) {
            insert_derived_binding(bindings, "sms_sender", sender, true);
        }
        for message in sms_messages {
            analysis.push(sms_metadata_analysis(&message));
            if let Some(decoded) = decode_sms_body(&message.body) {
                analysis.push(SequenceAnalysisLine {
                    kind: SequenceAnalysisKind::DecodedSms,
                    raw: format!("SMS body ({}): {}", decoded.charset, decoded.value),
                    masked: format!("SMS body ({}): <masked sensitive body>", decoded.charset),
                });
            } else if !message.body.trim().is_empty() {
                analysis.push(SequenceAnalysisLine {
                    kind: SequenceAnalysisKind::DecodedSms,
                    raw: "SMS body: <undecodable>".to_owned(),
                    masked: "SMS body: <undecodable>".to_owned(),
                });
            }
        }
        analysis.extend(parse_tcp_analysis(response, bindings));
        analysis.extend(parse_ping_analysis(response));
    }
    analysis
}

pub fn sms_candidates_from_text(text: &str) -> Vec<SmsMessageCandidate> {
    sms_candidates_from_responses(&[text.to_owned()])
}

pub fn value_candidate_sets_from_text(text: &str) -> Vec<SequenceValueCandidateSet> {
    value_candidate_sets_from_responses(&[text.to_owned()])
}

fn value_candidate_sets_from_responses(
    response_texts: &[String],
) -> Vec<SequenceValueCandidateSet> {
    let mut sets = BTreeMap::<SequenceCandidateSource, Vec<SequenceValueCandidate>>::new();
    for response in response_texts {
        push_value_candidate_set(
            &mut sets,
            SequenceCandidateSource::SmsMessage,
            sms_candidates_from_text(response)
                .into_iter()
                .map(sequence_value_candidate_from_sms)
                .collect(),
        );
        push_value_candidate_set(
            &mut sets,
            SequenceCandidateSource::PdpContext,
            pdp_context_candidates_from_text(response),
        );
    }
    sets.into_iter()
        .filter_map(|(candidate, candidates)| {
            dedupe_candidates(candidates).map(|candidates| SequenceValueCandidateSet {
                candidate,
                candidates,
            })
        })
        .collect()
}

fn push_value_candidate_set(
    sets: &mut BTreeMap<SequenceCandidateSource, Vec<SequenceValueCandidate>>,
    candidate: SequenceCandidateSource,
    candidates: Vec<SequenceValueCandidate>,
) {
    if !candidates.is_empty() {
        sets.entry(candidate).or_default().extend(candidates);
    }
}

fn dedupe_candidates(
    candidates: Vec<SequenceValueCandidate>,
) -> Option<Vec<SequenceValueCandidate>> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for candidate in candidates {
        if seen.insert(candidate.value.clone()) {
            unique.push(candidate);
        }
    }
    (!unique.is_empty()).then_some(unique)
}

fn sms_candidates_from_responses(response_texts: &[String]) -> Vec<SmsMessageCandidate> {
    response_texts
        .iter()
        .flat_map(|response| parse_sms_messages(response))
        .filter_map(|message| sms_candidate_from_message(&message))
        .collect()
}

fn sms_candidate_from_message(message: &SmsMessage) -> Option<SmsMessageCandidate> {
    let index = message.index.as_ref()?.trim();
    if index.is_empty() {
        return None;
    }
    let decoded = decode_sms_body(&message.body);
    let raw_body_preview = decoded
        .as_ref()
        .map(|decoded| sms_body_preview(&decoded.value))
        .or_else(|| {
            let body = message.body.trim();
            if body.is_empty() {
                None
            } else {
                Some("<undecodable>".to_owned())
            }
        });
    let masked_body_preview = raw_body_preview
        .as_ref()
        .map(|_| "<masked sensitive body>".to_owned());
    Some(SmsMessageCandidate {
        index: index.to_owned(),
        status: message.status.clone(),
        sender: message.sender.clone(),
        masked_sender: message
            .sender
            .as_ref()
            .map(|sender| mask_identifier(sender)),
        timestamp: message.timestamp.clone(),
        raw_body_preview,
        masked_body_preview,
    })
}

fn sequence_value_candidate_from_sms(candidate: SmsMessageCandidate) -> SequenceValueCandidate {
    SequenceValueCandidate {
        value: candidate.index.clone(),
        raw_label: sms_candidate_label_text(&candidate, false),
        masked_label: sms_candidate_label_text(&candidate, true),
    }
}

fn sms_candidate_label_text(
    candidate: &SmsMessageCandidate,
    output_masking_enabled: bool,
) -> String {
    let status = candidate.status.as_deref().unwrap_or("-");
    let sender = if output_masking_enabled {
        candidate.masked_sender.as_deref()
    } else {
        candidate.sender.as_deref()
    }
    .unwrap_or("-");
    let timestamp = candidate.timestamp.as_deref().unwrap_or("-");
    let preview = if output_masking_enabled {
        candidate.masked_body_preview.as_deref()
    } else {
        candidate.raw_body_preview.as_deref()
    }
    .unwrap_or("-");
    format!(
        "storage={}  {}  {}  {}  {}",
        candidate.index, status, sender, timestamp, preview
    )
}

fn pdp_context_candidates_from_text(text: &str) -> Vec<SequenceValueCandidate> {
    let mut contexts = BTreeMap::<String, BTreeSet<String>>::new();
    for line in response_lines(text) {
        if let Some((id, detail)) = parse_cgact_context(&line) {
            contexts.entry(id).or_default().insert(detail);
        }
        if let Some((id, details)) = parse_cgdcont_context(&line) {
            contexts.entry(id).or_default().extend(details);
        }
    }
    contexts
        .into_iter()
        .map(|(value, details)| {
            let detail = details.into_iter().collect::<Vec<_>>().join("  ");
            let label = if detail.is_empty() {
                value.clone()
            } else {
                format!("{value}  {detail}")
            };
            SequenceValueCandidate {
                value,
                raw_label: label.clone(),
                masked_label: label,
            }
        })
        .collect()
}

fn parse_cgact_context(line: &str) -> Option<(String, String)> {
    let fields = split_csv_fields(line.strip_prefix("+CGACT:")?);
    let id = csv_value(&fields, 0)?;
    let state = match csv_value(&fields, 1).as_deref() {
        Some("1") => "active".to_owned(),
        Some("0") => "inactive".to_owned(),
        Some(value) => format!("state={value}"),
        None => "-".to_owned(),
    };
    Some((id, state))
}

fn parse_cgdcont_context(line: &str) -> Option<(String, BTreeSet<String>)> {
    let fields = split_csv_fields(line.strip_prefix("+CGDCONT:")?);
    let id = csv_value(&fields, 0)?;
    let mut details = BTreeSet::new();
    if let Some(pdp_type) = csv_value(&fields, 1) {
        details.insert(pdp_type);
    }
    if let Some(apn) = csv_value(&fields, 2) {
        details.insert(apn);
    }
    Some((id, details))
}

fn sms_body_preview(value: &str) -> String {
    const MAX_CHARS: usize = 32;
    let preview = value.chars().take(MAX_CHARS).collect::<String>();
    if value.chars().count() > MAX_CHARS {
        format!("{preview}...")
    } else {
        preview
    }
}

fn insert_derived_binding(
    bindings: &mut BTreeMap<String, BoundParam>,
    name: &str,
    value: &str,
    sensitive: bool,
) {
    bindings.insert(
        name.to_owned(),
        BoundParam {
            value: value.to_owned(),
            sensitive,
        },
    );
    bindings.insert(
        format!("{name}_len"),
        BoundParam {
            value: value.len().to_string(),
            sensitive: false,
        },
    );
}

fn sms_metadata_analysis(message: &SmsMessage) -> SequenceAnalysisLine {
    let mut raw_parts = Vec::new();
    let mut masked_parts = Vec::new();
    if let Some(index) = &message.index {
        raw_parts.push(format!("index={index}"));
        masked_parts.push(format!("index={index}"));
    }
    if let Some(status) = &message.status {
        raw_parts.push(format!("status={status}"));
        masked_parts.push(format!("status={status}"));
    }
    if let Some(sender) = &message.sender {
        raw_parts.push(format!("sender={sender}"));
        masked_parts.push(format!("sender={}", mask_identifier(sender)));
    }
    if let Some(timestamp) = &message.timestamp {
        raw_parts.push(format!("timestamp={timestamp}"));
        masked_parts.push(format!("timestamp={timestamp}"));
    }

    SequenceAnalysisLine {
        kind: SequenceAnalysisKind::General,
        raw: format!("SMS message: {}", raw_parts.join(" ")),
        masked: format!("SMS message: {}", masked_parts.join(" ")),
    }
}

fn parse_sms_messages(text: &str) -> Vec<SmsMessage> {
    let lines = response_lines(text);
    let mut messages = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].as_str();
        let Some((kind, fields)) = parse_sms_header(line) else {
            index += 1;
            continue;
        };
        let mut body_lines = Vec::new();
        index += 1;
        while index < lines.len() {
            let next = lines[index].as_str();
            if parse_sms_header(next).is_some() {
                break;
            }
            if is_terminal_response_line(next) || is_command_echo_line(next) {
                index += 1;
                break;
            }
            body_lines.push(next.to_owned());
            index += 1;
        }

        messages.push(sms_message_from_fields(
            kind,
            &fields,
            body_lines.join("\n"),
        ));
    }
    messages
}

fn response_lines(text: &str) -> Vec<String> {
    text.split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_sms_header(line: &str) -> Option<(&'static str, Vec<CsvField>)> {
    if let Some(rest) = line.strip_prefix("+CMGL:") {
        return Some(("cmgl", split_csv_fields(rest)));
    }
    if let Some(rest) = line.strip_prefix("+CMGR:") {
        return Some(("cmgr", split_csv_fields(rest)));
    }
    None
}

fn sms_message_from_fields(kind: &str, fields: &[CsvField], body: String) -> SmsMessage {
    match kind {
        "cmgl" => SmsMessage {
            index: csv_value(fields, 0),
            status: csv_value(fields, 1),
            sender: csv_value(fields, 2),
            timestamp: last_non_empty_csv_value(fields, 3),
            body,
        },
        _ => SmsMessage {
            index: None,
            status: csv_value(fields, 0),
            sender: csv_value(fields, 1),
            timestamp: last_non_empty_csv_value(fields, 2),
            body,
        },
    }
}

fn csv_value(fields: &[CsvField], index: usize) -> Option<String> {
    fields
        .get(index)
        .map(|field| field.value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn last_non_empty_csv_value(fields: &[CsvField], start: usize) -> Option<String> {
    fields
        .iter()
        .skip(start)
        .rev()
        .map(|field| field.value.trim().to_owned())
        .find(|value| !value.is_empty())
}

fn split_csv_fields(input: &str) -> Vec<CsvField> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut quoted = false;
    for ch in input.trim().chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                quoted = true;
            }
            ',' if !in_quotes => {
                fields.push(CsvField {
                    value: field.trim().to_owned(),
                    quoted,
                });
                field.clear();
                quoted = false;
            }
            _ => field.push(ch),
        }
    }
    fields.push(CsvField {
        value: field.trim().to_owned(),
        quoted,
    });
    fields
}

fn render_csv_fields(fields: &[CsvField]) -> String {
    fields
        .iter()
        .map(|field| {
            if field.quoted {
                format!("\"{}\"", field.value)
            } else {
                field.value.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_sms_body(body: &str) -> Option<DecodedSmsBody> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    if let Some(value) = decode_ucs2_hex_body(body) {
        return Some(DecodedSmsBody {
            charset: "ucs2",
            value,
        });
    }
    Some(DecodedSmsBody {
        charset: "text",
        value: body.to_owned(),
    })
}

fn decode_ucs2_hex_body(body: &str) -> Option<String> {
    let compact = body
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect::<String>();
    if compact.len() < 4
        || compact.len() % 4 != 0
        || !compact.chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return None;
    }

    let mut units = Vec::new();
    for chunk in compact.as_bytes().chunks_exact(4) {
        let hex = std::str::from_utf8(chunk).ok()?;
        units.push(u16::from_str_radix(hex, 16).ok()?);
    }
    let decoded = String::from_utf16(&units).ok()?;
    if decoded
        .chars()
        .all(|ch| !ch.is_control() || ch == '\n' || ch == '\t')
    {
        Some(decoded)
    } else {
        None
    }
}

fn parse_tcp_analysis(
    text: &str,
    bindings: &BTreeMap<String, BoundParam>,
) -> Vec<SequenceAnalysisLine> {
    let mut analysis = Vec::new();
    for line in response_lines(text) {
        if let Some((total, acknowledged, unacknowledged)) = parse_qisend_counters(&line) {
            analysis.push(qisend_analysis(
                total,
                acknowledged,
                unacknowledged,
                bindings,
            ));
        }
        if let Some(length) = parse_qird_length(&line) {
            analysis.push(qird_analysis(length));
        }
    }
    analysis
}

fn parse_qisend_counters(line: &str) -> Option<(usize, usize, usize)> {
    let rest = line.strip_prefix("+QISEND:")?;
    let fields = rest
        .split(',')
        .map(str::trim)
        .map(str::parse::<usize>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    if fields.len() == 3 {
        Some((fields[0], fields[1], fields[2]))
    } else {
        None
    }
}

fn tcp_ack_requirement_failure(
    text: &str,
    bindings: &BTreeMap<String, BoundParam>,
) -> Option<String> {
    let payload_len = bindings
        .get("payload_len")
        .and_then(|value| value.value.parse::<usize>().ok())?;
    let mut last_counters = None;
    for line in response_lines(text) {
        if let Some(counters) = parse_qisend_counters(&line) {
            last_counters = Some(counters);
        }
    }
    let Some((total, acknowledged, unacknowledged)) = last_counters else {
        return Some(format!(
            "condition: TCP acknowledgement counters were not returned for payload_len={payload_len}"
        ));
    };
    if acknowledged >= payload_len && unacknowledged == 0 {
        return None;
    }
    Some(format!(
        "condition: TCP acknowledgement incomplete for payload_len={payload_len} total={total} acknowledged={acknowledged} unacknowledged={unacknowledged}"
    ))
}

fn parse_ping_analysis(text: &str) -> Vec<SequenceAnalysisLine> {
    response_lines(text)
        .into_iter()
        .filter_map(|line| parse_qping_line(&line))
        .map(|line| match line {
            PingLine::Reply(reply) => {
                let mut parts = vec![format!("host={}", reply.host)];
                if let Some(bytes) = reply.bytes {
                    parts.push(format!("bytes={bytes}"));
                }
                if let Some(time_ms) = reply.time_ms {
                    parts.push(format!("time_ms={time_ms}"));
                }
                if let Some(ttl) = reply.ttl {
                    parts.push(format!("ttl={ttl}"));
                }
                let text = format!("Ping reply: {}", parts.join(" "));
                SequenceAnalysisLine {
                    kind: SequenceAnalysisKind::General,
                    raw: text.clone(),
                    masked: text,
                }
            }
            PingLine::Summary(summary) => {
                let text = format!(
                    "Ping summary: sent={} received={} lost={}",
                    summary.sent, summary.received, summary.lost
                );
                SequenceAnalysisLine {
                    kind: SequenceAnalysisKind::General,
                    raw: text.clone(),
                    masked: text,
                }
            }
            PingLine::ResultCode(code) => {
                let text = format!("Ping result code: {code}");
                SequenceAnalysisLine {
                    kind: SequenceAnalysisKind::General,
                    raw: text.clone(),
                    masked: text,
                }
            }
        })
        .collect()
}

fn ping_success_requirement_failure(text: &str) -> Option<String> {
    let mut saw_ping_line = false;
    let mut last_summary = None;
    let mut last_result_code = None;
    for line in response_lines(text) {
        if !line.starts_with("+QPING:") {
            continue;
        }
        saw_ping_line = true;
        match parse_qping_line(&line) {
            Some(PingLine::Reply(_)) => return None,
            Some(PingLine::Summary(summary)) if summary.received > 0 => return None,
            Some(PingLine::Summary(summary)) => last_summary = Some(summary),
            Some(PingLine::ResultCode(code)) => last_result_code = Some(code),
            None => {}
        }
    }
    if let Some(summary) = last_summary {
        return Some(format!(
            "condition: ping received no replies sent={} received={} lost={}",
            summary.sent, summary.received, summary.lost
        ));
    }
    if let Some(code) = last_result_code {
        return Some(format!(
            "condition: ping result code {code} did not report a successful reply"
        ));
    }
    if saw_ping_line {
        Some("condition: ping did not report a successful reply".to_owned())
    } else {
        Some("condition: ping response lines were not returned".to_owned())
    }
}

fn parse_qping_line(line: &str) -> Option<PingLine> {
    let fields = split_csv_fields(line.strip_prefix("+QPING:")?);
    let result = csv_value(&fields, 0)?;
    if result != "0" {
        return Some(PingLine::ResultCode(result));
    }

    if let Some(reply) = parse_qping_reply(&fields) {
        return Some(PingLine::Reply(reply));
    }
    parse_qping_summary(&fields).map(PingLine::Summary)
}

fn parse_qping_reply(fields: &[CsvField]) -> Option<PingReply> {
    let host = fields.get(1)?;
    if !host.quoted
        && !host.value.contains('.')
        && !host.value.contains(':')
        && !host.value.chars().any(|ch| ch.is_ascii_alphabetic())
    {
        return None;
    }

    Some(PingReply {
        host: host.value.clone(),
        bytes: csv_usize(fields, 2),
        time_ms: csv_usize(fields, 3),
        ttl: csv_usize(fields, 4),
    })
}

fn parse_qping_summary(fields: &[CsvField]) -> Option<PingSummary> {
    Some(PingSummary {
        sent: csv_usize(fields, 1)?,
        received: csv_usize(fields, 2)?,
        lost: csv_usize(fields, 3)?,
    })
}

fn csv_usize(fields: &[CsvField], index: usize) -> Option<usize> {
    csv_value(fields, index)?.parse().ok()
}

fn qisend_analysis(
    total: usize,
    acknowledged: usize,
    unacknowledged: usize,
    bindings: &BTreeMap<String, BoundParam>,
) -> SequenceAnalysisLine {
    let payload_len = bindings
        .get("payload_len")
        .and_then(|value| value.value.parse::<usize>().ok());
    let verdict = match payload_len {
        Some(length) if acknowledged >= length && unacknowledged == 0 => {
            "payload bytes are TCP-acknowledged by the peer; this is not application processing proof"
        }
        Some(_) => {
            "payload bytes are not fully TCP-acknowledged yet; this is not end-to-end success"
        }
        None if unacknowledged == 0 => {
            "TCP counters report no unacknowledged bytes; this is not application processing proof"
        }
        None => "TCP counters report unacknowledged bytes; this is not end-to-end success",
    };
    let payload = payload_len
        .map(|length| format!(" payload_len={length}"))
        .unwrap_or_default();
    let text = format!(
        "TCP send counters: total={total} acknowledged={acknowledged} unacknowledged={unacknowledged}{payload}; {verdict}."
    );
    SequenceAnalysisLine {
        kind: SequenceAnalysisKind::General,
        raw: text.clone(),
        masked: text,
    }
}

fn parse_qird_length(line: &str) -> Option<usize> {
    let rest = line.strip_prefix("+QIRD:")?.trim();
    rest.split(',')
        .next()
        .map(str::trim)
        .and_then(|value| value.parse::<usize>().ok())
}

fn qird_analysis(length: usize) -> SequenceAnalysisLine {
    let text = if length == 0 {
        "TCP receive data: no buffered response data (+QIRD: 0).".to_owned()
    } else {
        format!(
            "TCP receive data: {length} byte(s) of buffered response data were returned; this is response data."
        )
    };
    SequenceAnalysisLine {
        kind: SequenceAnalysisKind::General,
        raw: text.clone(),
        masked: text,
    }
}

fn qiact_context_is_active(text: &str, context_id: &str) -> bool {
    response_lines(text).into_iter().any(|line| {
        parse_qiact_context_state(&line).is_some_and(|(id, state)| id == context_id && state == "1")
    })
}

fn parse_qiact_context_state(line: &str) -> Option<(String, String)> {
    let fields = split_csv_fields(line.strip_prefix("+QIACT:")?);
    let id = csv_value(&fields, 0)?;
    let state = csv_value(&fields, 1)?;
    Some((id, state))
}

fn mask_sequence_protocol_data(text: &str) -> String {
    let mut output = Vec::new();
    let mut mask_next_sms_body = false;
    let mut mask_next_qird_data = false;

    for line in response_lines_preserving_empty(text) {
        if let Some(masked_header) = mask_sms_header_sender(&line) {
            output.push(masked_header);
            mask_next_sms_body = true;
            mask_next_qird_data = false;
            continue;
        }
        if let Some(length) = parse_qird_length(line.trim()) {
            output.push(line);
            mask_next_qird_data = length > 0;
            mask_next_sms_body = false;
            continue;
        }
        if mask_next_sms_body {
            if line.trim().is_empty() {
                output.push(line);
                continue;
            }
            if is_terminal_response_line(line.trim()) {
                output.push(line);
                mask_next_sms_body = false;
            } else {
                output.push("<masked sms body>".to_owned());
            }
            continue;
        }
        if mask_next_qird_data {
            if line.trim().is_empty() {
                output.push(line);
                continue;
            }
            if is_terminal_response_line(line.trim()) {
                output.push(line);
                mask_next_qird_data = false;
            } else {
                output.push("<masked qird data>".to_owned());
            }
            continue;
        }
        output.push(line);
    }

    output.join("\n")
}

fn response_lines_preserving_empty(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|line| line.trim_end_matches('\r').to_owned())
        .collect()
}

fn render_sequence_modem_response_for_display(text: &str) -> String {
    let mut output = Vec::new();
    let mut collect_sms_body = false;

    for line in response_lines_preserving_empty(text) {
        let trimmed = line.trim();
        if parse_sms_header(trimmed).is_some() {
            output.push(line);
            collect_sms_body = true;
            continue;
        }

        if collect_sms_body {
            if is_terminal_response_line(trimmed) || is_command_echo_line(trimmed) {
                output.push(line);
                collect_sms_body = false;
                continue;
            }
            continue;
        }

        output.push(line);
    }

    output.join("\n")
}

fn mask_sms_header_sender(line: &str) -> Option<String> {
    let (prefix, sender_index, rest) = if let Some(rest) = line.trim().strip_prefix("+CMGL:") {
        ("+CMGL:", 2, rest)
    } else if let Some(rest) = line.trim().strip_prefix("+CMGR:") {
        ("+CMGR:", 1, rest)
    } else {
        return None;
    };
    let mut fields = split_csv_fields(rest);
    if let Some(field) = fields.get_mut(sender_index)
        && !field.value.is_empty()
    {
        field.value = mask_identifier(&field.value);
        field.quoted = true;
    }
    Some(format!("{prefix} {}", render_csv_fields(&fields)))
}

fn is_terminal_response_line(line: &str) -> bool {
    line == "OK"
        || line == "ERROR"
        || line == "NO CARRIER"
        || line.starts_with("+CME ERROR:")
        || line.starts_with("+CMS ERROR:")
}

fn is_command_echo_line(line: &str) -> bool {
    line.starts_with("AT")
}

fn response_matcher_for_step(
    sequence: &Sequence,
    step: &SequenceStep,
    bindings: &BTreeMap<String, BoundParam>,
) -> Result<ResponseMatcher> {
    if let Some(prompt) = &step.expect_prompt {
        return Ok(ResponseMatcher::ContainsOrErrorTerminal(render_template(
            prompt, bindings, sequence, &step.id,
        )?));
    }
    if let Some(urc) = &step.expect_urc {
        return Ok(ResponseMatcher::ContainsOrErrorTerminal(render_template(
            urc, bindings, sequence, &step.id,
        )?));
    }
    if let Some(expect) = &step.expect {
        let expect = render_template(expect, bindings, sequence, &step.id)?;
        if expect != "OK" && expect != "ERROR" {
            return Ok(ResponseMatcher::TerminalOrContains(expect));
        }
    }
    Ok(ResponseMatcher::Terminal)
}

fn step_expectation_failure(
    sequence: &Sequence,
    step: &SequenceStep,
    text: &str,
    bindings: &BTreeMap<String, BoundParam>,
) -> Result<Option<String>> {
    for expected in [&step.expect_prompt, &step.expect_urc, &step.expect]
        .into_iter()
        .flatten()
    {
        let expected = render_template(expected, bindings, sequence, &step.id)?;
        if !text.contains(&expected) {
            return Ok(Some(expected));
        }
    }
    Ok(None)
}

fn step_status(
    sequence: &Sequence,
    step: &SequenceStep,
    bindings: &BTreeMap<String, BoundParam>,
    text: &str,
    parsed: &AtStatus,
) -> Result<AtStatus> {
    if parsed.is_terminal() {
        Ok(parsed.clone())
    } else if [&step.expect_prompt, &step.expect_urc, &step.expect]
        .into_iter()
        .flatten()
        .map(|expected| render_template(expected, bindings, sequence, &step.id))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .any(|expected| text.contains(expected))
    {
        Ok(AtStatus::Ok)
    } else {
        Ok(parsed.clone())
    }
}

fn step_timeout(
    step: &SequenceStep,
    default_timeout: Duration,
    deadline: Instant,
) -> Result<Duration> {
    let now = Instant::now();
    if now >= deadline {
        return Err(AtctlError::Timeout);
    }
    let remaining = deadline - now;
    let step_timeout = step
        .timeout_secs
        .map(Duration::from_secs)
        .unwrap_or(default_timeout);
    Ok(step_timeout.min(remaining).max(Duration::from_secs(1)))
}

fn payload_with_terminator(payload: &str, terminator: StepTerminator) -> String {
    match terminator {
        StepTerminator::None => payload.to_owned(),
        StepTerminator::CtrlZ => format!("{payload}\u{1a}"),
        StepTerminator::Esc => format!("{payload}\u{1b}"),
    }
}

fn render_template(
    template: &str,
    bindings: &BTreeMap<String, BoundParam>,
    sequence: &Sequence,
    step: &str,
) -> Result<String> {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            return Err(AtctlError::InvalidSequenceParam {
                param: step.to_owned(),
                reason: "unterminated template placeholder".to_owned(),
            });
        };
        let name = after_start[..end].trim();
        let value = bindings
            .get(name)
            .ok_or_else(|| AtctlError::InvalidSequenceParam {
                param: name.to_owned(),
                reason: format!(
                    "sequence `{}` does not define this parameter",
                    sequence.name
                ),
            })?;
        output.push_str(&value.value);
        rest = &after_start[end + 2..];
    }
    output.push_str(rest);
    Ok(output)
}

fn push_transcript_header(
    transcript: &mut Vec<String>,
    index: usize,
    total: usize,
    step: &SequenceStep,
    masked: bool,
    bindings: &BTreeMap<String, BoundParam>,
) {
    push_transcript_block_gap(transcript);
    let label = step.label.as_deref().unwrap_or(&step.id);
    let label = if masked {
        mask_sequence_text(label, bindings)
    } else {
        label.to_owned()
    };
    transcript.push(format!("Step {}/{} {label}", index + 1, total));
}

fn push_transcript_section<I>(transcript: &mut Vec<String>, heading: &str, lines: I)
where
    I: IntoIterator<Item = String>,
{
    push_transcript_block_gap(transcript);
    transcript.push(heading.to_owned());
    transcript.extend(lines);
}

fn push_transcript_line_block(transcript: &mut Vec<String>, line: String) {
    push_transcript_block_gap(transcript);
    transcript.push(line);
}

fn push_transcript_block_gap(transcript: &mut Vec<String>) {
    if transcript.last().is_some_and(|line| !line.is_empty()) {
        transcript.push(String::new());
    }
}

fn push_response_transcript(
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    response: &str,
    bindings: &BTreeMap<String, BoundParam>,
) {
    let raw = response.trim_matches(['\r', '\n']);
    if !raw.is_empty() {
        let modem_response = render_sequence_modem_response_for_display(raw);
        if !modem_response.trim().is_empty() {
            push_transcript_section(
                raw_transcript,
                "Modem response:",
                response_lines_preserving_empty(&modem_response),
            );
            push_transcript_section(
                masked_transcript,
                "Modem response:",
                response_lines_preserving_empty(&mask_sequence_text(&modem_response, bindings)),
            );
        }
    }
}

fn push_analysis_transcript(
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    analysis: &[SequenceAnalysisLine],
) {
    let decoded = analysis
        .iter()
        .filter(|line| line.kind == SequenceAnalysisKind::DecodedSms)
        .collect::<Vec<_>>();
    if !decoded.is_empty() {
        push_transcript_section(
            raw_transcript,
            "Decoded SMS:",
            decoded.iter().map(|line| format!("- {}", line.raw)),
        );
        push_transcript_section(
            masked_transcript,
            "Decoded SMS:",
            decoded.iter().map(|line| format!("- {}", line.masked)),
        );
    }

    let general = analysis
        .iter()
        .filter(|line| line.kind == SequenceAnalysisKind::General)
        .collect::<Vec<_>>();
    if !general.is_empty() {
        push_transcript_section(
            raw_transcript,
            "Analysis:",
            general.iter().map(|line| format!("- {}", line.raw)),
        );
        push_transcript_section(
            masked_transcript,
            "Analysis:",
            general.iter().map(|line| format!("- {}", line.masked)),
        );
    }
}

fn step_failure_reason(
    sequence: &Sequence,
    step: &SequenceStep,
    status: &AtStatus,
    expectation_failure: Option<String>,
) -> Option<String> {
    if let Some(expected) = expectation_failure {
        if let Some(condition) = expected.strip_prefix("condition: ") {
            return Some(format!(
                "sequence `{}` step `{}` did not satisfy condition `{condition}`",
                sequence.name, step.id
            ));
        }
        return Some(format!(
            "sequence `{}` step `{}` did not produce expected marker `{expected}`",
            sequence.name, step.id
        ));
    }
    if !status.is_success() {
        return Some(format!(
            "sequence `{}` step `{}` ended with status `{status}`",
            sequence.name, step.id
        ));
    }
    None
}

fn push_failure_result(transcript: &mut Vec<String>, duration: Duration, reason: &str) {
    push_transcript_line_block(
        transcript,
        format!("Result: failed duration={}ms", duration.as_millis()),
    );
    transcript.push(format!("Reason: {reason}"));
}

#[allow(clippy::too_many_arguments)]
fn run_deferred_cleanups<T>(
    sequence: &Sequence,
    cleanups: &[DeferredCleanup],
    transport: &mut T,
    timeout: Duration,
    mut raw_log: Option<&mut RawLogSink>,
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    bindings: &BTreeMap<String, BoundParam>,
) -> Result<()>
where
    T: AtTransport,
{
    for cleanup in cleanups.iter().rev() {
        push_cleanup_header(raw_transcript, cleanup, false, bindings);
        push_cleanup_header(masked_transcript, cleanup, true, bindings);
        let cleanup_step = internal_command_step(cleanup.id.clone(), &cleanup.label, "OK");
        match execute_command_step(
            sequence,
            &cleanup_step,
            &cleanup.command,
            transport,
            timeout,
            raw_log.as_deref_mut(),
            raw_transcript,
            masked_transcript,
            bindings,
        ) {
            Ok(outcome) => {
                if let Some(reason) = step_failure_reason(
                    sequence,
                    &cleanup_step,
                    &outcome.status,
                    outcome.expectation_failure,
                ) {
                    push_cleanup_analysis(raw_transcript, masked_transcript, &reason, bindings);
                }
            }
            Err(error) => {
                let reason = format!("cleanup command failed before response: {error}");
                push_cleanup_analysis(raw_transcript, masked_transcript, &reason, bindings);
            }
        }
    }
    Ok(())
}

fn push_cleanup_header(
    transcript: &mut Vec<String>,
    cleanup: &DeferredCleanup,
    masked: bool,
    bindings: &BTreeMap<String, BoundParam>,
) {
    push_transcript_block_gap(transcript);
    let label = if masked {
        mask_sequence_text(&cleanup.label, bindings)
    } else {
        cleanup.label.clone()
    };
    transcript.push(label);
}

fn push_cleanup_analysis(
    raw_transcript: &mut Vec<String>,
    masked_transcript: &mut Vec<String>,
    reason: &str,
    bindings: &BTreeMap<String, BoundParam>,
) {
    push_transcript_section(raw_transcript, "Analysis:", [format!("- {reason}")]);
    push_transcript_section(
        masked_transcript,
        "Analysis:",
        [format!("- {}", mask_sequence_text(reason, bindings))],
    );
}

fn cleanup_timeout(deadline: Instant) -> Duration {
    let now = Instant::now();
    if now >= deadline {
        Duration::from_secs(1)
    } else {
        (deadline - now)
            .min(Duration::from_secs(10))
            .max(Duration::from_secs(1))
    }
}

fn mask_sequence_text(text: &str, bindings: &BTreeMap<String, BoundParam>) -> String {
    let mut output = text.to_owned();
    for value in bindings.values().filter(|value| value.sensitive) {
        if !value.value.is_empty() {
            output = output.replace(&value.value, &mask_identifier(&value.value));
        }
    }
    output = mask_sequence_protocol_data(&output);
    mask_sensitive_values(&output)
}

#[allow(clippy::too_many_arguments)]
fn append_sequence_raw_error(
    raw_log: Option<&mut RawLogSink>,
    sequence: &Sequence,
    step: &SequenceStep,
    command: &str,
    duration: Duration,
    stage: &'static str,
    error: &AtctlError,
    tx_bytes: &[u8],
) -> Result<()> {
    let Some(raw_log) = raw_log else {
        return Ok(());
    };
    let error = error.to_string();
    raw_log.append_transport_error(RawLogTransportError {
        command_name: Some(&step.id),
        command,
        risk: sequence.risk,
        duration,
        stage,
        error: &error,
        tx_bytes,
        rx_bytes: b"",
    })
}

pub fn required_param_summary(sequence: &Sequence) -> String {
    let required = sequence
        .params
        .iter()
        .filter(|param| param.required)
        .map(param_display)
        .collect::<Vec<_>>();
    if required.is_empty() {
        "-".to_owned()
    } else {
        required.join(",")
    }
}

fn param_display(param: &SequenceParam) -> String {
    let mut display = if let Some(default_value) = &param.default_value {
        format!("{}={default_value}", param.name)
    } else {
        param.name.clone()
    };
    let mut flags = Vec::new();
    if param.sensitive {
        flags.push("sensitive");
    }
    if param.source != crate::sequences::model::SequenceParamSource::User {
        flags.push(param.source.label());
    }
    if let Some(candidate) = param.candidate {
        flags.push(candidate.label());
    }
    if !flags.is_empty() {
        display.push('(');
        display.push_str(&flags.join(","));
        display.push(')');
    }
    display
}

pub fn format_missing_sequence_param(param: &SequenceParam) -> String {
    let mut message = format!("Sequence parameter `{}` is required.", param.name);
    if let Some(hint) = param.hint.as_deref() {
        message.push(' ');
        message.push_str(hint);
    } else if let Some(default_value) = &param.default_value {
        message.push_str(&format!(" Default value: `{default_value}`."));
    } else if let Some(candidate) = param.candidate {
        message.push_str(&format!(" Candidate source: {}.", candidate.label()));
    } else if param.source != crate::sequences::model::SequenceParamSource::User {
        message.push_str(&format!(" Value source: {}.", param.source.label()));
    }
    message
}

fn missing_param_hint_suffix(param: &SequenceParam) -> String {
    let mut parts = Vec::new();
    if let Some(default_value) = &param.default_value {
        parts.push(format!("default value `{default_value}` is available"));
    }
    if param.source != crate::sequences::model::SequenceParamSource::User {
        parts.push(format!("source={}", param.source.label()));
    }
    if let Some(candidate) = param.candidate {
        parts.push(format!("candidate={}", candidate.label()));
    }
    if let Some(hint) = &param.hint {
        parts.push(hint.clone());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(": {}", parts.join("; "))
    }
}

#[cfg(test)]
mod tests;
