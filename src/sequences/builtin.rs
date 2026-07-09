use crate::at::risk::RiskLevel;
use crate::sequences::definition::{SequenceDefinition, definitions_into_sequences};
use crate::sequences::model::{
    Sequence, SequenceCandidateSource, SequenceOrigin, SequenceParam, SequenceParamSource,
    SequenceReviewItem, SequenceStep, StepTerminator,
};

pub fn builtins() -> Vec<Sequence> {
    definitions_into_sequences(builtin_definitions(), SequenceOrigin::BuiltIn)
}

fn builtin_definitions() -> Vec<SequenceDefinition> {
    vec![
        sms_send_check(),
        sms_receive_check(),
        sms_read_message(),
        sms_reply_check(),
    ]
}

fn sms_send_check() -> SequenceDefinition {
    SequenceDefinition::new(
        "sms-send-check",
        "Send a standard SMS and report modem submit evidence.",
        RiskLevel::Write,
        vec!["sms".to_owned()],
        Some(180),
        vec![
            sensitive_param("recipient", "Recipient")
                .with_source(SequenceParamSource::User)
                .with_hint("Enter the SMS destination address."),
            sensitive_param("message", "Message body")
                .with_source(SequenceParamSource::User)
                .with_hint("Enter the SMS body to submit."),
        ],
        vec![
            SequenceStep {
                id: "set-text-mode".to_owned(),
                label: Some("Set SMS text mode".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGF=1".to_owned()),
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
            },
            SequenceStep {
                id: "start-submit".to_owned(),
                label: Some("Start SMS submit".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGS=\"{{recipient}}\"".to_owned()),
                expect: None,
                expect_prompt: Some(">".to_owned()),
                expect_urc: None,
                payload: None,
                terminator: StepTerminator::None,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(30),
                evidence: None,
                cleanup_on_failure: None,
            },
            SequenceStep {
                id: "write-message".to_owned(),
                label: Some("Write message body".to_owned()),
                ensure_pdp_context_active: None,
                send: None,
                expect: Some("+CMGS:".to_owned()),
                expect_prompt: None,
                expect_urc: None,
                payload: Some("{{message}}".to_owned()),
                terminator: StepTerminator::CtrlZ,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(120),
                evidence: None,
                cleanup_on_failure: None,
            },
        ],
    )
    .with_review_items(vec![
        review_item("Destination", "{{recipient}}", true),
        review_item("Message body", "{{message}}", true),
    ])
    .with_success_notes(vec![
        "+CMGS plus OK is SMS submit evidence, not destination handset receipt proof.".to_owned(),
    ])
}

fn sms_receive_check() -> SequenceDefinition {
    SequenceDefinition::new(
        "sms-receive-check",
        "List received SMS material and decode message bodies while keeping sensitive values masked.",
        RiskLevel::Write,
        vec!["sms".to_owned()],
        Some(120),
        Vec::new(),
        vec![
            SequenceStep {
                id: "set-text-mode".to_owned(),
                label: Some("Set SMS text mode".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGF=1".to_owned()),
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
            },
            SequenceStep {
                id: "list-messages".to_owned(),
                label: Some("List SMS messages".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGL=\"ALL\"".to_owned()),
                expect: Some("OK".to_owned()),
                expect_prompt: None,
                expect_urc: None,
                payload: None,
                terminator: StepTerminator::None,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(90),
                evidence: Some(
                    "REC READ and REC UNREAD are modem message status values; sender and body values stay masked by default."
                        .to_owned(),
                ),
                cleanup_on_failure: None,
            },
        ],
    )
}

fn sms_read_message() -> SequenceDefinition {
    SequenceDefinition::new(
        "sms-read-message",
        "Read one SMS storage index; the modem may change unread status to read.",
        RiskLevel::Write,
        vec!["sms".to_owned()],
        Some(120),
        vec![
            plain_param("index", "SMS storage index")
                .with_source(SequenceParamSource::Select)
                .with_candidate(SequenceCandidateSource::SmsMessage)
                .with_hint("Use the SMS storage index shown by sms-receive-check or AT+CMGL."),
        ],
        vec![
            SequenceStep {
                id: "set-text-mode".to_owned(),
                label: Some("Set SMS text mode".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGF=1".to_owned()),
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
            },
            SequenceStep {
                id: "read-message".to_owned(),
                label: Some("Read SMS message".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGR={{index}}".to_owned()),
                expect: Some("OK".to_owned()),
                expect_prompt: None,
                expect_urc: None,
                payload: None,
                terminator: StepTerminator::None,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(90),
                evidence: Some(
                    "AT+CMGR may change an unread message to read state on the modem.".to_owned(),
                ),
                cleanup_on_failure: None,
            },
        ],
    )
    .with_review_items(vec![review_item("SMS storage index", "{{index}}", false)])
}

fn sms_reply_check() -> SequenceDefinition {
    SequenceDefinition::new(
        "sms-reply-check",
        "Reply to the sender of one SMS storage index and report modem submit evidence.",
        RiskLevel::Write,
        vec!["sms".to_owned()],
        Some(180),
        vec![
            plain_param("index", "SMS storage index")
                .with_source(SequenceParamSource::Select)
                .with_candidate(SequenceCandidateSource::SmsMessage)
                .with_hint("Use the SMS storage index shown by sms-receive-check or AT+CMGL."),
            sensitive_param("message", "Reply body")
                .with_source(SequenceParamSource::User)
                .with_hint("Enter the reply body. The recipient is derived from AT+CMGR."),
        ],
        vec![
            SequenceStep {
                id: "set-text-mode".to_owned(),
                label: Some("Set SMS text mode".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGF=1".to_owned()),
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
            },
            SequenceStep {
                id: "read-original-message".to_owned(),
                label: Some("Read original SMS message".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGR={{index}}".to_owned()),
                expect: Some("OK".to_owned()),
                expect_prompt: None,
                expect_urc: None,
                payload: None,
                terminator: StepTerminator::None,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(90),
                evidence: Some(
                    "AT+CMGR may change an unread message to read state; reply recipient is resolved from the returned sender."
                        .to_owned(),
                ),
                cleanup_on_failure: None,
            },
            SequenceStep {
                id: "start-reply-submit".to_owned(),
                label: Some("Start SMS reply submit".to_owned()),
                ensure_pdp_context_active: None,
                send: Some("AT+CMGS=\"{{sms_sender}}\"".to_owned()),
                expect: None,
                expect_prompt: Some(">".to_owned()),
                expect_urc: None,
                payload: None,
                terminator: StepTerminator::None,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(30),
                evidence: None,
                cleanup_on_failure: None,
            },
            SequenceStep {
                id: "write-reply".to_owned(),
                label: Some("Write reply body".to_owned()),
                ensure_pdp_context_active: None,
                send: None,
                expect: Some("+CMGS:".to_owned()),
                expect_prompt: None,
                expect_urc: None,
                payload: Some("{{message}}".to_owned()),
                terminator: StepTerminator::CtrlZ,
                require_tcp_ack: false,
                require_ping_success: false,
                timeout_secs: Some(120),
                evidence: None,
                cleanup_on_failure: None,
            },
        ],
    )
    .with_review_items(vec![
        review_item("SMS storage index", "{{index}}", false),
        review_item("Reply body", "{{message}}", true),
    ])
    .with_success_notes(vec![
        "Reply recipient is the sender extracted from AT+CMGR for the reviewed SMS storage index.".to_owned(),
        "+CMGS plus OK is SMS submit evidence, not destination handset receipt proof.".to_owned(),
    ])
}

fn sensitive_param(name: &str, label: &str) -> SequenceParam {
    SequenceParam {
        name: name.to_owned(),
        label: label.to_owned(),
        required: true,
        sensitive: true,
        default_value: None,
        source: SequenceParamSource::User,
        candidate: None,
        hint: None,
    }
}

fn plain_param(name: &str, label: &str) -> SequenceParam {
    SequenceParam {
        name: name.to_owned(),
        label: label.to_owned(),
        required: true,
        sensitive: false,
        default_value: None,
        source: SequenceParamSource::User,
        candidate: None,
        hint: None,
    }
}

trait SequenceParamBuilder {
    fn with_source(self, source: SequenceParamSource) -> Self;
    fn with_candidate(self, candidate: SequenceCandidateSource) -> Self;
    fn with_hint(self, hint: &str) -> Self;
}

impl SequenceParamBuilder for SequenceParam {
    fn with_source(mut self, source: SequenceParamSource) -> Self {
        self.source = source;
        self
    }

    fn with_candidate(mut self, candidate: SequenceCandidateSource) -> Self {
        self.candidate = Some(candidate);
        self
    }

    fn with_hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_owned());
        self
    }
}

fn review_item(label: &str, value: &str, sensitive: bool) -> SequenceReviewItem {
    SequenceReviewItem {
        label: label.to_owned(),
        value: value.to_owned(),
        sensitive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_sequences_keep_product_origin_after_definition_conversion() {
        let sequences = builtins();
        let names = sequences
            .iter()
            .map(|sequence| sequence.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "sms-send-check",
                "sms-receive-check",
                "sms-read-message",
                "sms-reply-check"
            ]
        );
        assert!(
            sequences
                .iter()
                .all(|sequence| sequence.origin == SequenceOrigin::BuiltIn)
        );
    }
}
