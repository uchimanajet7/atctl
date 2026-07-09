use crate::at::risk::RiskLevel;
use crate::sequences::model::{
    Sequence, SequenceOrigin, SequenceParam, SequenceReviewItem, SequenceStep,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceDefinition {
    pub name: String,
    pub summary: String,
    pub risk: RiskLevel,
    pub categories: Vec<String>,
    pub timeout_secs: Option<u64>,
    pub before_running: Vec<String>,
    pub params: Vec<SequenceParam>,
    pub review_items: Vec<SequenceReviewItem>,
    pub success_notes: Vec<String>,
    pub steps: Vec<SequenceStep>,
}

impl SequenceDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        summary: impl Into<String>,
        risk: RiskLevel,
        categories: Vec<String>,
        timeout_secs: Option<u64>,
        params: Vec<SequenceParam>,
        steps: Vec<SequenceStep>,
    ) -> Self {
        Self {
            name: name.into(),
            summary: summary.into(),
            risk,
            categories,
            timeout_secs,
            before_running: Vec::new(),
            params,
            review_items: Vec::new(),
            success_notes: Vec::new(),
            steps,
        }
    }

    pub fn with_before_running(mut self, before_running: Vec<String>) -> Self {
        self.before_running = before_running;
        self
    }

    pub fn with_review_items(mut self, review_items: Vec<SequenceReviewItem>) -> Self {
        self.review_items = review_items;
        self
    }

    pub fn with_success_notes(mut self, success_notes: Vec<String>) -> Self {
        self.success_notes = success_notes;
        self
    }

    pub fn into_sequence(self, origin: SequenceOrigin) -> Sequence {
        Sequence::new(
            self.name,
            self.summary,
            self.risk,
            self.categories,
            origin,
            self.timeout_secs,
            self.params,
            self.steps,
        )
        .with_before_running(self.before_running)
        .with_review_items(self.review_items)
        .with_success_notes(self.success_notes)
    }
}

pub fn definitions_into_sequences(
    definitions: impl IntoIterator<Item = SequenceDefinition>,
    origin: SequenceOrigin,
) -> Vec<Sequence> {
    definitions
        .into_iter()
        .map(|definition| definition.into_sequence(origin.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequences::model::{SequenceParamSource, StepTerminator};

    #[test]
    fn converts_definition_into_origin_aware_sequence() {
        let sequence = SequenceDefinition::new(
            "sms-send-check",
            "Send an SMS.",
            RiskLevel::Write,
            vec!["sms".to_owned()],
            Some(180),
            vec![SequenceParam {
                name: "message".to_owned(),
                label: "Message body".to_owned(),
                required: true,
                sensitive: true,
                default_value: None,
                source: SequenceParamSource::User,
                candidate: None,
                hint: Some("Enter the SMS body.".to_owned()),
            }],
            vec![SequenceStep {
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
            }],
        )
        .with_before_running(vec!["Confirm SIM state before sending.".to_owned()])
        .with_review_items(vec![SequenceReviewItem {
            label: "Message body".to_owned(),
            value: "{{message}}".to_owned(),
            sensitive: true,
        }])
        .with_success_notes(vec!["+CMGS plus OK is SMS submit evidence.".to_owned()])
        .into_sequence(SequenceOrigin::BuiltIn);

        assert_eq!(sequence.name, "sms-send-check");
        assert_eq!(sequence.origin, SequenceOrigin::BuiltIn);
        assert_eq!(sequence.categories, vec!["sms"]);
        assert_eq!(sequence.timeout_secs, Some(180));
        assert_eq!(sequence.before_running.len(), 1);
        assert_eq!(sequence.params.len(), 1);
        assert_eq!(sequence.review_items.len(), 1);
        assert_eq!(sequence.success_notes.len(), 1);
        assert_eq!(sequence.steps.len(), 1);
    }
}
