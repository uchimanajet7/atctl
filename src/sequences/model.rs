use crate::at::risk::{RiskLevel, classify_direct_command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    pub name: String,
    pub summary: String,
    pub declared_risk: RiskLevel,
    pub risk: RiskLevel,
    pub origin: SequenceOrigin,
    pub categories: Vec<String>,
    pub timeout_secs: Option<u64>,
    pub before_running: Vec<String>,
    pub params: Vec<SequenceParam>,
    pub review_items: Vec<SequenceReviewItem>,
    pub success_notes: Vec<String>,
    pub steps: Vec<SequenceStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceOrigin {
    BuiltIn,
    File {
        title: String,
        path: String,
        description: Option<String>,
    },
}

impl SequenceOrigin {
    pub fn file(
        title: impl Into<String>,
        path: impl Into<String>,
        description: Option<String>,
    ) -> Self {
        Self::File {
            title: title.into(),
            path: path.into(),
            description,
        }
    }

    pub fn id(&self) -> String {
        match self {
            Self::BuiltIn => "built-in-sequences".to_owned(),
            Self::File { path, .. } => format!("file:{path}"),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::BuiltIn => "Product Sequences",
            Self::File { title, .. } => title,
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::BuiltIn => None,
            Self::File { title, .. } => Some(title),
        }
    }

    pub fn file_path(&self) -> Option<&str> {
        match self {
            Self::BuiltIn => None,
            Self::File { path, .. } => Some(path),
        }
    }

    pub fn is_built_in(&self) -> bool {
        matches!(self, Self::BuiltIn)
    }

    pub fn sort_key(&self) -> (u8, String) {
        let kind = match self {
            Self::BuiltIn => 0,
            Self::File { .. } => 1,
        };
        (kind, self.label().to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceParam {
    pub name: String,
    pub label: String,
    pub required: bool,
    pub sensitive: bool,
    pub default_value: Option<String>,
    pub source: SequenceParamSource,
    pub candidate: Option<SequenceCandidateSource>,
    pub hint: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SequenceParamSource {
    #[default]
    User,
    Default,
    Modem,
    Select,
    Sequence,
    Derived,
    External,
}

impl SequenceParamSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Default => "default",
            Self::Modem => "modem",
            Self::Select => "select",
            Self::Sequence => "sequence",
            Self::Derived => "derived",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SequenceCandidateSource {
    SmsMessage,
    PdpContext,
}

impl SequenceCandidateSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::SmsMessage => "sms-message",
            Self::PdpContext => "pdp-context",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceReviewItem {
    pub label: String,
    pub value: String,
    pub sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceStep {
    pub id: String,
    pub label: Option<String>,
    pub ensure_pdp_context_active: Option<String>,
    pub send: Option<String>,
    pub expect: Option<String>,
    pub expect_prompt: Option<String>,
    pub expect_urc: Option<String>,
    pub payload: Option<String>,
    pub terminator: StepTerminator,
    pub require_tcp_ack: bool,
    pub require_ping_success: bool,
    pub timeout_secs: Option<u64>,
    pub evidence: Option<String>,
    pub cleanup_on_failure: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepTerminator {
    None,
    CtrlZ,
    Esc,
}

impl Sequence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        summary: impl Into<String>,
        declared_risk: RiskLevel,
        categories: Vec<String>,
        origin: SequenceOrigin,
        timeout_secs: Option<u64>,
        params: Vec<SequenceParam>,
        steps: Vec<SequenceStep>,
    ) -> Self {
        let risk = aggregate_sequence_risk(declared_risk, &params, &steps);
        Self {
            name: name.into(),
            summary: summary.into(),
            declared_risk,
            risk,
            origin,
            categories,
            timeout_secs,
            before_running: Vec::new(),
            params,
            review_items: Vec::new(),
            success_notes: Vec::new(),
            steps,
        }
    }

    #[cfg(test)]
    pub(crate) fn built_in(
        name: impl Into<String>,
        summary: impl Into<String>,
        declared_risk: RiskLevel,
        categories: Vec<String>,
        timeout_secs: Option<u64>,
        params: Vec<SequenceParam>,
        steps: Vec<SequenceStep>,
    ) -> Self {
        Self::new(
            name,
            summary,
            declared_risk,
            categories,
            SequenceOrigin::BuiltIn,
            timeout_secs,
            params,
            steps,
        )
    }

    pub fn with_review_items(mut self, review_items: Vec<SequenceReviewItem>) -> Self {
        self.review_items = review_items;
        self
    }

    pub fn with_success_notes(mut self, success_notes: Vec<String>) -> Self {
        self.success_notes = success_notes;
        self
    }

    pub fn with_before_running(mut self, before_running: Vec<String>) -> Self {
        self.before_running = before_running;
        self
    }
}

pub fn aggregate_sequence_risk(
    declared_risk: RiskLevel,
    params: &[SequenceParam],
    steps: &[SequenceStep],
) -> RiskLevel {
    let mut risk = declared_risk;
    if params.iter().any(|param| param.sensitive) || steps.iter().any(|step| step.payload.is_some())
    {
        risk = risk.stricter(RiskLevel::Sensitive);
    }
    for step in steps {
        if step.ensure_pdp_context_active.is_some() {
            risk = risk.stricter(RiskLevel::Write);
        }
        if let Some(command) = &step.send {
            risk = risk.stricter(classify_direct_command(command).risk);
        }
        if let Some(command) = &step.cleanup_on_failure {
            risk = risk.stricter(classify_direct_command(command).risk);
        }
    }
    risk
}
