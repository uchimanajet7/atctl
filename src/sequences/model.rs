use crate::at::review::{ExecutionReview, review_direct_command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    pub name: String,
    pub summary: String,
    pub review: ExecutionReview,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SequenceCandidateProviderKind {
    Preset,
    Sequence,
}

impl SequenceCandidateProviderKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Preset => "Preset",
            Self::Sequence => "Sequence",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SequenceCandidateProvider {
    pub kind: SequenceCandidateProviderKind,
    pub name: &'static str,
}

impl SequenceCandidateSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::SmsMessage => "sms-message",
            Self::PdpContext => "pdp-context",
        }
    }

    pub fn providers(self) -> &'static [SequenceCandidateProvider] {
        const SMS_MESSAGE: &[SequenceCandidateProvider] = &[SequenceCandidateProvider {
            kind: SequenceCandidateProviderKind::Sequence,
            name: "sms-receive-check",
        }];
        const PDP_CONTEXT: &[SequenceCandidateProvider] = &[
            SequenceCandidateProvider {
                kind: SequenceCandidateProviderKind::Preset,
                name: "active-pdp-contexts",
            },
            SequenceCandidateProvider {
                kind: SequenceCandidateProviderKind::Preset,
                name: "pdp-contexts",
            },
        ];

        match self {
            Self::SmsMessage => SMS_MESSAGE,
            Self::PdpContext => PDP_CONTEXT,
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
        categories: Vec<String>,
        origin: SequenceOrigin,
        timeout_secs: Option<u64>,
        params: Vec<SequenceParam>,
        steps: Vec<SequenceStep>,
    ) -> Self {
        let name = name.into();
        let review = review_sequence(&name, &params, &steps);
        Self {
            name,
            summary: summary.into(),
            review,
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
        categories: Vec<String>,
        timeout_secs: Option<u64>,
        params: Vec<SequenceParam>,
        steps: Vec<SequenceStep>,
    ) -> Self {
        Self::new(
            name,
            summary,
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

pub fn review_sequence(
    name: &str,
    params: &[SequenceParam],
    steps: &[SequenceStep],
) -> ExecutionReview {
    let mut components = Vec::new();
    for step in steps {
        if step.ensure_pdp_context_active.is_some() {
            components.push(review_direct_command("AT+QIACT?"));
            components.push(review_direct_command("AT+QIACT=1"));
        }
        if let Some(command) = &step.send {
            components.push(review_direct_command(command));
        }
        if let Some(command) = &step.cleanup_on_failure {
            components.push(review_direct_command(command));
        }
    }
    ExecutionReview::sequence(
        name,
        components,
        params.iter().any(|param| param.sensitive),
        steps.iter().any(|step| step.payload.is_some()),
    )
}
