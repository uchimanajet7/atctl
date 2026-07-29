use std::fmt;

use serde::Serialize;

use crate::Result;
use crate::app::errors::AtctlError;
use crate::at::command::normalize_command;
use crate::at::mask::mask_sensitive_values;

pub const CONFIRMATION_FLOW_TAG: &str = "[confirm]";
pub const UNKNOWN_EFFECTS_NOTICE: &str = "Effects are not fully known.";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Assessment {
    Known,
    Unknown,
}

impl fmt::Display for Assessment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Known => "known",
            Self::Unknown => "unknown",
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effect {
    ObserveData,
    ChangeRuntimeState,
    ChangePersistentState,
    CommunicateExternally,
    DeleteResetOrRecover,
    InterruptServiceOrConnectivity,
}

impl Effect {
    pub fn label(self) -> &'static str {
        match self {
            Self::ObserveData => "observe or read data",
            Self::ChangeRuntimeState => "change runtime state",
            Self::ChangePersistentState => "change persistent state",
            Self::CommunicateExternally => "communicate or transmit externally",
            Self::DeleteResetOrRecover => "delete, reset, or recover",
            Self::InterruptServiceOrConnectivity => "interrupt service or connectivity",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum DataHandling {
    #[serde(rename = "sensitive-input")]
    Input,
    #[serde(rename = "sensitive-output")]
    Output,
    #[serde(rename = "sensitive-external-disclosure")]
    ExternalDisclosure,
}

impl DataHandling {
    pub fn label(self) -> &'static str {
        match self {
            Self::Input => "sensitive input",
            Self::Output => "sensitive output",
            Self::ExternalDisclosure => "sensitive external disclosure",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reversibility {
    NotApplicable,
    ImmediatelyReversible,
    RestartRequired,
    PersistentUntilChanged,
    Irreversible,
    NotEstablished,
}

impl Reversibility {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotApplicable => "not applicable",
            Self::ImmediatelyReversible => "immediately reversible",
            Self::RestartRequired => "restart required",
            Self::PersistentUntilChanged => "persistent until changed",
            Self::Irreversible => "irreversible",
            Self::NotEstablished => "not established",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutionReview {
    pub normalized_operation: String,
    pub assessment: Assessment,
    pub effects: Vec<Effect>,
    pub data_handling: Vec<DataHandling>,
    pub reversibility: Reversibility,
    pub consequences: Vec<String>,
    pub not_established: Vec<String>,
    pub confirmation_required: bool,
    #[serde(rename = "reasons")]
    pub confirmation_reasons: Vec<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReviewPresentationSectionKind {
    WhatWillHappen,
    Known,
    PossibleConsequences,
    SensitiveData,
    PersistsRecovery,
    NotEstablished,
}

impl ReviewPresentationSectionKind {
    pub fn heading(self) -> &'static str {
        match self {
            Self::WhatWillHappen => "What will happen",
            Self::Known => "Known",
            Self::PossibleConsequences => "Possible consequences",
            Self::SensitiveData => "Sensitive data",
            Self::PersistsRecovery => "Persists / Recovery",
            Self::NotEstablished => "Not established",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewPresentationSection {
    pub kind: ReviewPresentationSectionKind,
    pub items: Vec<String>,
}

impl ExecutionReview {
    pub fn requires_confirmation(&self) -> bool {
        self.confirmation_required
    }

    pub fn confirmation_token(&self) -> &'static str {
        if self.assessment == Assessment::Unknown {
            "unknown"
        } else {
            "run"
        }
    }

    pub fn confirmation_flow_tag(&self) -> Option<&'static str> {
        self.requires_confirmation()
            .then_some(CONFIRMATION_FLOW_TAG)
    }

    pub fn masked(&self) -> Self {
        let mut masked = self.clone();
        masked.normalized_operation = mask_sensitive_values(&masked.normalized_operation);
        masked
    }

    pub fn presentation_sections(&self) -> Vec<ReviewPresentationSection> {
        let mut sections = Vec::new();

        if self.assessment == Assessment::Known {
            sections.push(ReviewPresentationSection {
                kind: ReviewPresentationSectionKind::WhatWillHappen,
                items: self.consequence_items(),
            });
        } else {
            let known = if self.effects.is_empty() {
                vec!["No material effect has been established.".to_owned()]
            } else {
                self.effects
                    .iter()
                    .map(|effect| effect.label().to_owned())
                    .collect()
            };
            sections.push(ReviewPresentationSection {
                kind: ReviewPresentationSectionKind::Known,
                items: known,
            });
            sections.push(ReviewPresentationSection {
                kind: ReviewPresentationSectionKind::PossibleConsequences,
                items: self.consequence_items(),
            });
        }

        if !self.data_handling.is_empty() {
            sections.push(ReviewPresentationSection {
                kind: ReviewPresentationSectionKind::SensitiveData,
                items: self
                    .data_handling
                    .iter()
                    .map(|handling| handling.label().to_owned())
                    .collect(),
            });
        }

        if !matches!(
            self.reversibility,
            Reversibility::NotApplicable | Reversibility::NotEstablished
        ) {
            sections.push(ReviewPresentationSection {
                kind: ReviewPresentationSectionKind::PersistsRecovery,
                items: vec![self.reversibility.label().to_owned()],
            });
        }

        if self.assessment == Assessment::Unknown {
            sections.push(ReviewPresentationSection {
                kind: ReviewPresentationSectionKind::NotEstablished,
                items: self.not_established.clone(),
            });
        }

        sections
    }

    fn consequence_items(&self) -> Vec<String> {
        if self.consequences.is_empty() {
            self.effects
                .iter()
                .map(|effect| effect.label().to_owned())
                .collect()
        } else {
            self.consequences.clone()
        }
    }

    pub fn sequence(
        name: &str,
        component_reviews: impl IntoIterator<Item = ExecutionReview>,
        has_sensitive_input: bool,
        has_payload: bool,
    ) -> Self {
        let mut reviews = component_reviews.into_iter().collect::<Vec<_>>();
        let mut review = Self {
            normalized_operation: format!("Sequence: {name}"),
            assessment: Assessment::Known,
            effects: Vec::new(),
            data_handling: Vec::new(),
            reversibility: Reversibility::NotApplicable,
            consequences: Vec::new(),
            not_established: Vec::new(),
            confirmation_required: false,
            confirmation_reasons: Vec::new(),
        };

        for component in reviews.drain(..) {
            if component.assessment == Assessment::Unknown {
                review.assessment = Assessment::Unknown;
            }
            extend_unique(&mut review.effects, component.effects);
            extend_unique(&mut review.data_handling, component.data_handling);
            extend_unique(&mut review.consequences, component.consequences);
            extend_unique(&mut review.not_established, component.not_established);
            extend_unique(
                &mut review.confirmation_reasons,
                component.confirmation_reasons,
            );
            review.reversibility =
                combine_reversibility(review.reversibility, component.reversibility);
            review.confirmation_required |= component.confirmation_required;
        }

        if has_sensitive_input {
            push_unique(&mut review.data_handling, DataHandling::Input);
        }
        if has_payload
            && has_sensitive_input
            && review.effects.contains(&Effect::CommunicateExternally)
        {
            push_unique(&mut review.data_handling, DataHandling::ExternalDisclosure);
            review.confirmation_required = true;
            push_unique(
                &mut review.confirmation_reasons,
                "The operation transmits operator-provided data externally.".to_owned(),
            );
        }
        if review.assessment == Assessment::Unknown {
            review.confirmation_required = true;
            push_unique(
                &mut review.confirmation_reasons,
                "One or more operation boundaries are not established.".to_owned(),
            );
        }
        review
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExecutionAuthorization {
    NotRequired,
    InteractiveRequired,
    NonInteractiveApproved,
}

pub fn review_direct_command(command: &str) -> ExecutionReview {
    let normalized = normalize_command(command);

    if is_known_observation(&normalized) {
        return known_observation(normalized, false);
    }
    if is_known_sensitive_observation(&normalized) {
        return known_observation(normalized, true);
    }
    if normalized == "AT+CFUN=1,1" {
        return known_material(
            normalized,
            vec![
                Effect::ChangeRuntimeState,
                Effect::InterruptServiceOrConnectivity,
                Effect::DeleteResetOrRecover,
            ],
            Vec::new(),
            Reversibility::RestartRequired,
            "Changes modem functionality and can interrupt connectivity or restart the modem.",
        );
    }
    if normalized.starts_with("AT+CFUN=") {
        return known_material(
            normalized,
            vec![
                Effect::ChangeRuntimeState,
                Effect::InterruptServiceOrConnectivity,
            ],
            Vec::new(),
            Reversibility::ImmediatelyReversible,
            "Changes modem functionality and can interrupt connectivity until changed again.",
        );
    }
    if normalized == "AT+QPOWD" {
        return known_material(
            normalized,
            vec![
                Effect::ChangeRuntimeState,
                Effect::InterruptServiceOrConnectivity,
            ],
            Vec::new(),
            Reversibility::RestartRequired,
            "Powers down the modem and interrupts service until it is started again.",
        );
    }
    if normalized == "AT+QCFG=\"NWSCANMODE\",0,1" {
        return known_material(
            normalized,
            vec![Effect::ChangePersistentState],
            Vec::new(),
            Reversibility::PersistentUntilChanged,
            "Changes a modem setting that remains in effect until changed again.",
        );
    }
    if normalized == "AT+CMGL" || normalized.starts_with("AT+CMGL=") {
        return known_material(
            normalized,
            vec![Effect::ObserveData, Effect::ChangePersistentState],
            vec![DataHandling::Output],
            Reversibility::PersistentUntilChanged,
            "Lists stored SMS content and changes received-unread messages to received-read.",
        );
    }
    if normalized == "AT+QIRD=?" {
        return known_observation(normalized, false);
    }
    if normalized.starts_with("AT+QIRD=") {
        if is_qird_size_query(&normalized) {
            return known_observation(normalized, true);
        }
        return known_material(
            normalized,
            vec![Effect::ObserveData, Effect::ChangeRuntimeState],
            vec![DataHandling::Output],
            Reversibility::Irreversible,
            "Retrieves received socket data and advances the module's read/unread buffer state.",
        );
    }
    if normalized == "AT+QISEND=?" {
        return known_observation(normalized, false);
    }
    if is_qisend_counter_query(&normalized) {
        return known_observation(normalized, true);
    }
    if is_external_communication(&normalized) {
        let mut data = Vec::new();
        if is_sensitive_input_command(&normalized) {
            data.push(DataHandling::Input);
            data.push(DataHandling::ExternalDisclosure);
        }
        return known_material(
            normalized,
            vec![Effect::CommunicateExternally],
            data,
            Reversibility::Irreversible,
            "Sends data or initiates communication outside the modem.",
        );
    }
    if normalized.starts_with("AT+CMGR=") {
        return known_material(
            normalized,
            vec![Effect::ObserveData, Effect::ChangeRuntimeState],
            vec![DataHandling::Output],
            Reversibility::PersistentUntilChanged,
            "Reads an SMS and may change its stored unread status to read.",
        );
    }
    if normalized.starts_with("AT+CMGW") {
        return known_material(
            normalized,
            vec![Effect::ChangePersistentState],
            vec![DataHandling::Input],
            Reversibility::PersistentUntilChanged,
            "Stores message material on the modem until it is changed or deleted.",
        );
    }
    if is_known_persistent_change(&normalized) {
        let mut data = Vec::new();
        if normalized.starts_with("AT+CGAUTH=") {
            data.push(DataHandling::Input);
        }
        return known_material(
            normalized,
            vec![Effect::ChangePersistentState],
            data,
            Reversibility::PersistentUntilChanged,
            "Changes stored modem configuration until changed again.",
        );
    }
    if is_known_runtime_change(&normalized) {
        let mut effects = vec![Effect::ChangeRuntimeState];
        if changes_connectivity(&normalized) {
            effects.push(Effect::InterruptServiceOrConnectivity);
        }
        return known_material(
            normalized,
            effects,
            Vec::new(),
            Reversibility::ImmediatelyReversible,
            "Changes modem runtime state until changed again or the session is reset.",
        );
    }

    unknown_review(normalized)
}

pub fn authorize_execution(
    review: &ExecutionReview,
    yes: bool,
    allow_unknown: bool,
) -> Result<ExecutionAuthorization> {
    if allow_unknown && !yes {
        return Err(AtctlError::AllowUnknownRequiresYes);
    }
    if allow_unknown && review.assessment != Assessment::Unknown {
        return Err(AtctlError::AllowUnknownNotApplicable);
    }
    if !review.requires_confirmation() {
        return Ok(ExecutionAuthorization::NotRequired);
    }
    if !yes {
        return Ok(ExecutionAuthorization::InteractiveRequired);
    }
    if review.assessment == Assessment::Unknown && !allow_unknown {
        return Err(AtctlError::UnknownOperationRequiresAllowance);
    }
    Ok(ExecutionAuthorization::NonInteractiveApproved)
}

pub fn is_prompt_required_command(command: &str) -> bool {
    let normalized = normalize_command(command);
    if matches!(
        normalized.as_str(),
        "AT+CMGS=?" | "AT+CMGW=?" | "AT+CMGC=?" | "AT+CNMA=?"
    ) {
        return false;
    }
    matches!(
        normalized.as_str(),
        "AT+CMGS" | "AT+CMGW" | "AT+CMGC" | "AT+CNMA"
    ) || normalized.starts_with("AT+CMGS=")
        || normalized.starts_with("AT+CMGW=")
        || normalized.starts_with("AT+CMGC=")
        || normalized.starts_with("AT+CNMA=")
        || (normalized.starts_with("AT+QISEND=")
            && normalized != "AT+QISEND=?"
            && !is_qisend_counter_query(&normalized))
}

pub fn is_supported_bridge_prompt_command(command: &str) -> bool {
    let normalized = normalize_command(command);
    matches!(
        normalized.as_str(),
        "AT+CMGS" | "AT+CMGW" | "AT+CMGC" | "AT+CNMA"
    ) || normalized.starts_with("AT+CMGS=")
        || normalized.starts_with("AT+CMGW=")
        || normalized.starts_with("AT+CMGC=")
        || normalized.starts_with("AT+CNMA=")
}

fn known_observation(normalized_operation: String, sensitive: bool) -> ExecutionReview {
    ExecutionReview {
        normalized_operation,
        assessment: Assessment::Known,
        effects: vec![Effect::ObserveData],
        data_handling: if sensitive {
            vec![DataHandling::Output]
        } else {
            Vec::new()
        },
        reversibility: Reversibility::NotApplicable,
        consequences: vec![if sensitive {
            "Reads modem, subscriber, message, or operational network data; the normal masked-output policy is applied by default."
                .to_owned()
        } else {
            "Reads modem information without changing device state.".to_owned()
        }],
        not_established: Vec::new(),
        confirmation_required: false,
        confirmation_reasons: Vec::new(),
    }
}

fn known_material(
    normalized_operation: String,
    effects: Vec<Effect>,
    data_handling: Vec<DataHandling>,
    reversibility: Reversibility,
    consequence: &str,
) -> ExecutionReview {
    ExecutionReview {
        normalized_operation,
        assessment: Assessment::Known,
        effects,
        data_handling,
        reversibility,
        consequences: vec![consequence.to_owned()],
        not_established: Vec::new(),
        confirmation_required: true,
        confirmation_reasons: vec!["The reviewed operation has a material effect.".to_owned()],
    }
}

fn unknown_review(normalized_operation: String) -> ExecutionReview {
    ExecutionReview {
        normalized_operation,
        assessment: Assessment::Unknown,
        effects: Vec::new(),
        data_handling: Vec::new(),
        reversibility: Reversibility::NotEstablished,
        consequences: vec![
            "Proceeding may change modem, network, service, stored data, or external systems in ways atctl cannot establish."
                .to_owned(),
        ],
        not_established: vec![
            "operation effects".to_owned(),
            "data disclosure".to_owned(),
            "persistence and reversibility".to_owned(),
            "complete consequences".to_owned(),
        ],
        confirmation_required: true,
        confirmation_reasons: vec![
            "The complete semantics of this operation are not established.".to_owned(),
        ],
    }
}

fn combine_reversibility(left: Reversibility, right: Reversibility) -> Reversibility {
    use Reversibility::{
        ImmediatelyReversible, Irreversible, NotApplicable, NotEstablished, PersistentUntilChanged,
        RestartRequired,
    };
    match (left, right) {
        (NotEstablished, _) | (_, NotEstablished) => NotEstablished,
        (Irreversible, _) | (_, Irreversible) => Irreversible,
        (PersistentUntilChanged, _) | (_, PersistentUntilChanged) => PersistentUntilChanged,
        (RestartRequired, _) | (_, RestartRequired) => RestartRequired,
        (ImmediatelyReversible, _) | (_, ImmediatelyReversible) => ImmediatelyReversible,
        (NotApplicable, NotApplicable) => NotApplicable,
    }
}

fn push_unique<T: PartialEq>(target: &mut Vec<T>, value: T) {
    if !target.contains(&value) {
        target.push(value);
    }
}

fn extend_unique<T: PartialEq>(target: &mut Vec<T>, values: impl IntoIterator<Item = T>) {
    for value in values {
        push_unique(target, value);
    }
}

fn is_known_observation(command: &str) -> bool {
    matches!(
        command,
        "AT" | "ATI"
            | "AT+CGMI"
            | "AT+CGMM"
            | "AT+CGMR"
            | "AT+CPIN?"
            | "AT+CPIN=?"
            | "AT+CPAS"
            | "AT+CPAS=?"
            | "AT+WS46?"
            | "AT+WS46=?"
            | "AT+COPS?"
            | "AT+COPS=?"
            | "AT+CREG?"
            | "AT+CREG=?"
            | "AT+CEREG?"
            | "AT+CEREG=?"
            | "AT+CGREG?"
            | "AT+CGREG=?"
            | "AT+CEER"
            | "AT+CMEE?"
            | "AT+CMEE=?"
            | "AT+CSQ"
            | "AT+CSQ=?"
            | "AT+CESQ"
            | "AT+CESQ=?"
            | "AT+CGDCONT?"
            | "AT+CGAUTH=?"
            | "AT+CFUN?"
            | "AT+CGATT?"
            | "AT+CGATT=?"
            | "AT+CGACT?"
            | "AT+CGACT=?"
            | "AT+CGPADDR"
            | "AT+CGPADDR=?"
            | "AT+CGCONTRDP"
            | "AT+CSMS?"
            | "AT+CSMS=?"
            | "AT+CMGF?"
            | "AT+CMGF=?"
            | "AT+CMGL=?"
            | "AT+CMGR=?"
            | "AT+CMGS=?"
            | "AT+CMGW=?"
            | "AT+CMGC=?"
            | "AT+CNMA=?"
            | "AT+CPMS?"
            | "AT+CPMS=?"
            | "AT+CFUN=?"
            | "AT+QCSQ"
            | "AT+QNWINFO"
            | "AT+QENG=\"SERVINGCELL\""
            | "AT+QENG=\"NEIGHBOURCELL\""
            | "AT+QINISTAT"
            | "AT+QSPN"
            | "AT+QLTS"
            | "AT+QCFG=?"
            | "AT+QIACT=?"
            | "AT+QIDEACT=?"
            | "AT+QIOPEN=?"
            | "AT+QICLOSE=?"
            | "AT+QISENDEX=?"
            | "AT+QPING=?"
    )
}

fn is_known_sensitive_observation(command: &str) -> bool {
    matches!(
        command,
        "AT+CIMI"
            | "AT+CGAUTH?"
            | "AT+QCCID"
            | "AT+CGSN"
            | "AT+QCFG?"
            | "AT+QCFG=\"NWSCANMODE\""
            | "AT+QPINC?"
            | "AT+QMBNCFG=\"LIST\""
            | "AT+QIACT?"
            | "AT+QISTATE"
    ) || command.starts_with("AT+CGSN")
}

fn is_qird_size_query(command: &str) -> bool {
    let Some(arguments) = command.strip_prefix("AT+QIRD=") else {
        return false;
    };
    let mut fields = arguments.split(',').map(str::trim);
    matches!(fields.next(), Some(connect_id) if connect_id.parse::<u8>().is_ok())
        && matches!(fields.next(), Some(length) if length.parse::<usize>() == Ok(0))
        && fields.next().is_none()
}

fn is_qisend_counter_query(command: &str) -> bool {
    let Some(arguments) = command.strip_prefix("AT+QISEND=") else {
        return false;
    };
    let mut fields = arguments.split(',').map(str::trim);
    matches!(fields.next(), Some(connect_id) if connect_id.parse::<u8>().is_ok())
        && matches!(fields.next(), Some(length) if length.parse::<usize>() == Ok(0))
        && fields.next().is_none()
}

fn is_external_communication(command: &str) -> bool {
    command.starts_with("AT+CMGS")
        || command.starts_with("AT+CMGC")
        || command.starts_with("AT+CNMA")
        || command.starts_with("AT+QIOPEN=")
        || command.starts_with("AT+QISEND=")
        || command.starts_with("AT+QISENDEX=")
        || command.starts_with("AT+QPING=")
}

fn is_sensitive_input_command(command: &str) -> bool {
    command.starts_with("AT+CMGS")
        || command.starts_with("AT+CMGC")
        || command.starts_with("AT+QISEND=")
        || command.starts_with("AT+QISENDEX=")
}

fn is_known_runtime_change(command: &str) -> bool {
    command.starts_with("AT+WS46=")
        || command.starts_with("AT+COPS=")
        || command.starts_with("AT+CREG=")
        || command.starts_with("AT+CGREG=")
        || command.starts_with("AT+CEREG=")
        || command.starts_with("AT+CGACT=")
        || command.starts_with("AT+CMEE=")
        || command.starts_with("AT+CMGF=")
        || command.starts_with("AT+QIACT=")
        || command.starts_with("AT+QIDEACT=")
        || command.starts_with("AT+QICLOSE=")
        || command == "ATE0"
}

fn is_known_persistent_change(command: &str) -> bool {
    command.starts_with("AT+CGDCONT=") || command.starts_with("AT+CGAUTH=")
}

fn changes_connectivity(command: &str) -> bool {
    command.starts_with("AT+COPS=")
        || command.starts_with("AT+CGACT=")
        || command.starts_with("AT+QIDEACT=")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_observation_needs_no_confirmation() {
        let review = review_direct_command("AT+CSQ");
        assert_eq!(review.assessment, Assessment::Known);
        assert_eq!(review.effects, vec![Effect::ObserveData]);
        assert!(!review.confirmation_required);
        assert_eq!(review.confirmation_token(), "run");
        assert_eq!(review.confirmation_flow_tag(), None);
    }

    #[test]
    fn confirmation_flow_tag_is_identical_for_known_and_unknown_operations() {
        let known = review_direct_command("AT+CMEE=2");
        let unknown = review_direct_command("AT+VENDOR=1");

        assert_eq!(known.assessment, Assessment::Known);
        assert_eq!(known.confirmation_token(), "run");
        assert_eq!(known.confirmation_flow_tag(), Some(CONFIRMATION_FLOW_TAG));
        assert_eq!(unknown.assessment, Assessment::Unknown);
        assert_eq!(unknown.confirmation_token(), "unknown");
        assert_eq!(unknown.confirmation_flow_tag(), Some(CONFIRMATION_FLOW_TAG));
    }

    #[test]
    fn sensitive_output_is_not_an_operation_effect() {
        let review = review_direct_command("AT+CIMI");
        assert_eq!(review.effects, vec![Effect::ObserveData]);
        assert_eq!(review.data_handling, vec![DataHandling::Output]);
        assert!(!review.confirmation_required);

        let json = serde_json::to_value(&review).unwrap();
        assert_eq!(json["data_handling"][0], "sensitive-output");
    }

    #[test]
    fn masked_review_does_not_repeat_sensitive_command_values() {
        let review = review_direct_command("AT+CGAUTH=1,2,\"sora\",\"secret\"");
        let masked = review.masked();

        assert!(!masked.normalized_operation.contains("sora"));
        assert!(!masked.normalized_operation.contains("secret"));
        assert!(masked.normalized_operation.contains('*'));
    }

    #[test]
    fn unknown_read_syntax_remains_unknown_and_requires_confirmation() {
        let review = review_direct_command("AT+EXAMPLE?");
        assert_eq!(review.assessment, Assessment::Unknown);
        assert!(review.confirmation_required);
        assert!(!review.not_established.is_empty());
    }

    #[test]
    fn external_send_reports_effect_and_disclosure_separately() {
        let review = review_direct_command("AT+CMGS=\"+819012345678\"");
        assert!(review.effects.contains(&Effect::CommunicateExternally));
        assert!(
            review
                .data_handling
                .contains(&DataHandling::ExternalDisclosure)
        );
        assert!(review.confirmation_required);
    }

    #[test]
    fn listing_sms_reports_the_read_status_change() {
        let review = review_direct_command("AT+CMGL=\"ALL\"");

        assert_eq!(review.assessment, Assessment::Known);
        assert!(review.effects.contains(&Effect::ObserveData));
        assert!(review.effects.contains(&Effect::ChangePersistentState));
        assert!(review.data_handling.contains(&DataHandling::Output));
        assert!(review.confirmation_required);
    }

    #[test]
    fn qird_data_read_reports_receive_buffer_state_change() {
        let review = review_direct_command("AT+QIRD=0,1500");

        assert_eq!(review.assessment, Assessment::Known);
        assert!(review.effects.contains(&Effect::ObserveData));
        assert!(review.effects.contains(&Effect::ChangeRuntimeState));
        assert_eq!(review.reversibility, Reversibility::Irreversible);
        assert!(review.confirmation_required);
    }

    #[test]
    fn qird_size_queries_do_not_report_buffer_consumption() {
        for command in ["AT+QIRD=?", "AT+QIRD=0,0"] {
            let review = review_direct_command(command);

            assert_eq!(review.assessment, Assessment::Known, "{command}");
            assert_eq!(review.effects, vec![Effect::ObserveData], "{command}");
            assert!(!review.confirmation_required, "{command}");
        }
    }

    #[test]
    fn qisend_counter_and_capability_queries_are_observations() {
        for command in ["AT+QISEND=?", "AT+QISEND=0,0"] {
            let review = review_direct_command(command);

            assert_eq!(review.assessment, Assessment::Known, "{command}");
            assert_eq!(review.effects, vec![Effect::ObserveData], "{command}");
            assert!(!review.confirmation_required, "{command}");
            assert!(!is_prompt_required_command(command), "{command}");
        }
    }

    #[test]
    fn qisend_data_entry_requires_a_prompt_capable_surface() {
        assert!(is_prompt_required_command("AT+QISEND=0,4"));
        assert!(!is_supported_bridge_prompt_command("AT+QISEND=0,4"));
    }

    #[test]
    fn known_test_forms_are_observations_not_family_writes() {
        for command in [
            "AT+CFUN=?",
            "AT+CREG=?",
            "AT+CGACT=?",
            "AT+CMGR=?",
            "AT+CMGS=?",
            "AT+QIACT=?",
            "AT+QIOPEN=?",
            "AT+QPING=?",
        ] {
            let review = review_direct_command(command);

            assert_eq!(review.assessment, Assessment::Known, "{command}");
            assert_eq!(review.effects, vec![Effect::ObserveData], "{command}");
            assert!(!review.confirmation_required, "{command}");
            assert!(!is_prompt_required_command(command), "{command}");
        }
    }

    #[test]
    fn unmaintained_qcfg_forms_remain_unknown() {
        let review = review_direct_command("AT+QCFG=\"vendor-option\",1");

        assert_eq!(review.assessment, Assessment::Unknown);
        assert!(review.confirmation_required);
    }

    #[test]
    fn unknown_noninteractive_execution_needs_both_flags() {
        let review = review_direct_command("AT+EXAMPLE=1");
        assert!(matches!(
            authorize_execution(&review, true, false),
            Err(AtctlError::UnknownOperationRequiresAllowance)
        ));
        assert_eq!(
            authorize_execution(&review, true, true).unwrap(),
            ExecutionAuthorization::NonInteractiveApproved
        );
    }

    #[test]
    fn allow_unknown_is_rejected_for_known_operations() {
        let review = review_direct_command("AT+CFUN=0");
        assert!(matches!(
            authorize_execution(&review, true, true),
            Err(AtctlError::AllowUnknownNotApplicable)
        ));
    }

    #[test]
    fn detects_sms_prompt_required_commands() {
        assert!(is_prompt_required_command("AT+CMGS=\"+819012345678\""));
        assert!(is_prompt_required_command("AT+CMGW"));
        assert!(!is_prompt_required_command("AT+CMGF?"));
    }

    #[test]
    fn known_presentation_omits_empty_optional_sections() {
        let review = review_direct_command("AT+CFUN=0");
        let sections = review.presentation_sections();

        assert_eq!(
            sections
                .iter()
                .map(|section| section.kind)
                .collect::<Vec<_>>(),
            vec![
                ReviewPresentationSectionKind::WhatWillHappen,
                ReviewPresentationSectionKind::PersistsRecovery,
            ]
        );
        assert!(!sections.iter().any(|section| section.items.is_empty()));
    }

    #[test]
    fn unknown_presentation_separates_known_possible_and_unestablished() {
        let review = review_direct_command("AT+EXAMPLE=1");
        let sections = review.presentation_sections();

        assert_eq!(
            sections
                .iter()
                .map(|section| section.kind)
                .collect::<Vec<_>>(),
            vec![
                ReviewPresentationSectionKind::Known,
                ReviewPresentationSectionKind::PossibleConsequences,
                ReviewPresentationSectionKind::NotEstablished,
            ]
        );
        assert_eq!(
            sections[0].items,
            vec!["No material effect has been established."]
        );
        assert!(!sections[2].items.is_empty());
    }
}
