use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::app::errors::AtctlError;
use crate::at::command::normalize_command;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLevel {
    Safe,
    Sensitive,
    Write,
    Persistent,
    Dangerous,
    Unknown,
}

impl RiskLevel {
    pub fn requires_confirmation(self) -> bool {
        matches!(
            self,
            Self::Write | Self::Persistent | Self::Dangerous | Self::Unknown
        )
    }

    fn enforcement_rank(self) -> u8 {
        match self {
            Self::Safe => 0,
            Self::Sensitive => 1,
            Self::Write => 2,
            Self::Unknown => 3,
            Self::Persistent => 4,
            Self::Dangerous => 5,
        }
    }

    pub fn stricter(self, other: Self) -> Self {
        if self.enforcement_rank() >= other.enforcement_rank() {
            self
        } else {
            other
        }
    }
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Safe => "safe",
            Self::Sensitive => "sensitive",
            Self::Write => "write",
            Self::Persistent => "persistent",
            Self::Dangerous => "dangerous",
            Self::Unknown => "unknown",
        };
        f.write_str(value)
    }
}

impl FromStr for RiskLevel {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "safe" => Ok(Self::Safe),
            "sensitive" => Ok(Self::Sensitive),
            "write" => Ok(Self::Write),
            "persistent" => Ok(Self::Persistent),
            "dangerous" => Ok(Self::Dangerous),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("unknown risk level: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskClassification {
    pub normalized_command: String,
    pub risk: RiskLevel,
    pub reason: &'static str,
}

impl RiskClassification {
    pub fn requires_confirmation(&self) -> bool {
        self.risk.requires_confirmation()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectSendConfirmation {
    NotRequired,
    InteractiveRequired { risk: RiskLevel },
    AutomationBypassApproved,
}

pub fn classify_direct_command(command: &str) -> RiskClassification {
    let normalized = normalize_command(command);
    let risk = if is_known_safe_read(&normalized) {
        (RiskLevel::Safe, "known safe read command")
    } else if is_known_sensitive_read(&normalized) {
        (RiskLevel::Sensitive, "known sensitive read command")
    } else if is_known_dangerous(&normalized) {
        (RiskLevel::Dangerous, "known dangerous command family")
    } else if is_known_persistent(&normalized) {
        (RiskLevel::Persistent, "known persistent command family")
    } else if is_known_write(&normalized) {
        (RiskLevel::Write, "known write command family")
    } else if is_read_or_test(&normalized) {
        (
            RiskLevel::Sensitive,
            "unknown read/test command treated as sensitive",
        )
    } else {
        (
            RiskLevel::Unknown,
            "command cannot be confidently classified as read/test",
        )
    };

    RiskClassification {
        normalized_command: normalized,
        risk: risk.0,
        reason: risk.1,
    }
}

pub fn direct_send_confirmation(
    classification: &RiskClassification,
    yes: bool,
    acknowledged: Option<RiskLevel>,
) -> Result<DirectSendConfirmation> {
    if let Some(acknowledged) = acknowledged
        && acknowledged != classification.risk
    {
        return Err(AtctlError::RiskAckMismatch {
            classified: classification.risk,
            acknowledged,
        });
    }

    if !classification.requires_confirmation() {
        return Ok(DirectSendConfirmation::NotRequired);
    }

    if !yes {
        return Ok(DirectSendConfirmation::InteractiveRequired {
            risk: classification.risk,
        });
    }

    if acknowledged.is_none() {
        return Err(AtctlError::MissingRiskAck {
            risk: classification.risk,
        });
    }

    Ok(DirectSendConfirmation::AutomationBypassApproved)
}

#[cfg(test)]
fn validate_direct_send_ack(
    classification: &RiskClassification,
    yes: bool,
    acknowledged: Option<RiskLevel>,
) -> Result<()> {
    if let DirectSendConfirmation::InteractiveRequired { risk } =
        direct_send_confirmation(classification, yes, acknowledged)?
    {
        return Err(AtctlError::ConfirmationRequired { risk });
    }

    Ok(())
}

pub fn is_prompt_required_command(command: &str) -> bool {
    let normalized = normalize_command(command);
    matches!(
        normalized.as_str(),
        "AT+CMGS" | "AT+CMGW" | "AT+CMGC" | "AT+CNMA"
    ) || normalized.starts_with("AT+CMGS=")
        || normalized.starts_with("AT+CMGW=")
        || normalized.starts_with("AT+CMGC=")
        || normalized.starts_with("AT+CNMA=")
}

fn is_read_or_test(command: &str) -> bool {
    command.starts_with("AT") && (command.ends_with('?') || !command.contains('='))
}

fn is_known_safe_read(command: &str) -> bool {
    matches!(
        command,
        "AT" | "ATI"
            | "AT+CGMI"
            | "AT+CGMM"
            | "AT+CGMR"
            | "AT+CPIN?"
            | "AT+CPAS"
            | "AT+WS46?"
            | "AT+WS46=?"
            | "AT+COPS?"
            | "AT+COPS=?"
            | "AT+CREG?"
            | "AT+CEREG?"
            | "AT+CGREG?"
            | "AT+CEER"
            | "AT+CMEE?"
            | "AT+CMEE=?"
            | "AT+CSQ"
            | "AT+CESQ"
            | "AT+CESQ=?"
            | "AT+CGDCONT?"
            | "AT+CGAUTH=?"
            | "AT+CFUN?"
            | "AT+CGATT?"
            | "AT+CGACT?"
            | "AT+CGPADDR"
            | "AT+CGPADDR=?"
            | "AT+CGCONTRDP"
            | "AT+CSMS?"
            | "AT+CMGF?"
            | "AT+CPMS?"
            | "AT+QCSQ"
            | "AT+QNWINFO"
            | "AT+QENG=\"SERVINGCELL\""
            | "AT+QENG=\"NEIGHBOURCELL\""
            | "AT+QINISTAT"
            | "AT+QSPN"
            | "AT+QLTS"
    )
}

fn is_known_sensitive_read(command: &str) -> bool {
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
    ) || command.starts_with("AT+CGSN")
        || command.starts_with("AT+QIRD=")
}

fn is_known_dangerous(command: &str) -> bool {
    command.starts_with("AT+CFUN=") || command == "AT+QPOWD"
}

fn is_known_persistent(command: &str) -> bool {
    command.starts_with("AT+QCFG=")
}

fn is_known_write(command: &str) -> bool {
    command.starts_with("AT+CGDCONT=")
        || command.starts_with("AT+CGAUTH=")
        || command.starts_with("AT+WS46=")
        || command.starts_with("AT+COPS=")
        || command.starts_with("AT+CREG=")
        || command.starts_with("AT+CGREG=")
        || command.starts_with("AT+CEREG=")
        || command.starts_with("AT+CGACT=")
        || command.starts_with("AT+CMEE=")
        || command.starts_with("AT+CMGF=")
        || command.starts_with("AT+CMGL=")
        || command.starts_with("AT+CMGR=")
        || command.starts_with("AT+CMGS")
        || command.starts_with("AT+CMGW")
        || command.starts_with("AT+CMGC")
        || command.starts_with("AT+CNMA")
        || command.starts_with("AT+QIACT=")
        || command.starts_with("AT+QIDEACT=")
        || command.starts_with("AT+QIOPEN=")
        || command.starts_with("AT+QICLOSE=")
        || command.starts_with("AT+QISEND=")
        || command.starts_with("AT+QISENDEX=")
        || command.starts_with("AT+QPING=")
        || command == "ATE0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_plain_safe_command() {
        let classification = classify_direct_command("AT");

        assert_eq!(classification.risk, RiskLevel::Safe);
        assert!(!classification.requires_confirmation());
    }

    #[test]
    fn classifies_sensitive_identifier_read() {
        let classification = classify_direct_command("AT+CIMI");

        assert_eq!(classification.risk, RiskLevel::Sensitive);
        assert!(!classification.requires_confirmation());
    }

    #[test]
    fn unknown_read_is_sensitive_by_default() {
        let classification = classify_direct_command("AT+EXAMPLE?");

        assert_eq!(classification.risk, RiskLevel::Sensitive);
        assert!(!classification.requires_confirmation());
    }

    #[test]
    fn classifies_standard_workflow_reads_as_safe() {
        for command in [
            "AT+CGMI",
            "AT+CGMM",
            "AT+CGMR",
            "AT+WS46?",
            "AT+WS46=?",
            "AT+COPS?",
            "AT+COPS=?",
            "AT+CREG?",
            "AT+CEREG?",
            "AT+CGREG?",
            "AT+CEER",
            "AT+CMEE?",
            "AT+CMEE=?",
            "AT+CPAS",
            "AT+CGDCONT?",
            "AT+CGAUTH=?",
            "AT+CFUN?",
            "AT+CGATT?",
            "AT+CGACT?",
            "AT+CGPADDR",
            "AT+CGPADDR=?",
            "AT+CGCONTRDP",
            "AT+CESQ",
            "AT+CESQ=?",
            "AT+CSMS?",
            "AT+CMGF?",
            "AT+CPMS?",
            "AT+QINISTAT",
            "AT+QSPN",
            "AT+QLTS",
        ] {
            assert_eq!(classify_direct_command(command).risk, RiskLevel::Safe);
        }
    }

    #[test]
    fn classifies_known_sensitive_vendor_reads() {
        for command in [
            "AT+CGAUTH?",
            "AT+QCCID",
            "AT+QPINC?",
            "AT+QCFG?",
            "AT+QCFG=\"nwscanmode\"",
            "AT+QMBNCFG=\"List\"",
        ] {
            assert_eq!(classify_direct_command(command).risk, RiskLevel::Sensitive);
        }
    }

    #[test]
    fn classifies_verbose_error_reporting_as_write() {
        let classification = classify_direct_command("AT+CMEE=2");

        assert_eq!(classification.risk, RiskLevel::Write);
        assert!(classification.requires_confirmation());
    }

    #[test]
    fn classifies_known_sms_and_socket_operations_as_write_or_sensitive() {
        for command in [
            "AT+CMGF=1",
            "AT+CMGS=\"+819012345678\"",
            "AT+CMGL=\"ALL\"",
            "AT+QIACT=1",
            "AT+QIOPEN=1,0,\"TCP\",\"192.0.2.1\",8009,0,0",
            "AT+QISEND=0,4",
            "AT+QICLOSE=0",
            "AT+QPING=1,\"pong.soracom.io\",4,4",
        ] {
            assert_eq!(
                classify_direct_command(command).risk,
                RiskLevel::Write,
                "{command}"
            );
        }

        assert_eq!(
            classify_direct_command("AT+QIRD=0,1500").risk,
            RiskLevel::Sensitive
        );
    }

    #[test]
    fn classifies_connectivity_runtime_changes_as_write() {
        for command in [
            "AT+CGAUTH=1,2,\"sora\",\"sora\"",
            "AT+WS46=28",
            "AT+COPS=3,2",
            "AT+COPS=0",
            "AT+CREG=2",
            "AT+CGREG=2",
            "AT+CEREG=5",
        ] {
            let classification = classify_direct_command(command);

            assert_eq!(classification.risk, RiskLevel::Write, "{command}");
            assert!(classification.requires_confirmation(), "{command}");
        }
    }

    #[test]
    fn stricter_risk_preserves_higher_enforcement() {
        assert_eq!(
            RiskLevel::Sensitive.stricter(RiskLevel::Write),
            RiskLevel::Write
        );
        assert_eq!(
            RiskLevel::Dangerous.stricter(RiskLevel::Safe),
            RiskLevel::Dangerous
        );
    }

    #[test]
    fn detects_sms_prompt_required_commands() {
        assert!(is_prompt_required_command("AT+CMGS=\"+819012345678\""));
        assert!(is_prompt_required_command("AT+CMGW"));
        assert!(!is_prompt_required_command("AT+CMGF?"));
    }

    #[test]
    fn dangerous_command_requires_matching_ack() {
        for command in ["AT+CFUN=0", "AT+CFUN=1", "AT+CFUN=1,1", "AT+QPOWD"] {
            let classification = classify_direct_command(command);

            assert_eq!(classification.risk, RiskLevel::Dangerous);
            assert!(
                validate_direct_send_ack(&classification, true, Some(RiskLevel::Dangerous)).is_ok()
            );
            assert!(matches!(
                validate_direct_send_ack(&classification, true, Some(RiskLevel::Write)),
                Err(AtctlError::RiskAckMismatch { .. })
            ));
        }
    }

    #[test]
    fn mismatched_ack_is_rejected_even_when_confirmation_is_not_required() {
        let classification = classify_direct_command("AT");

        assert!(matches!(
            validate_direct_send_ack(&classification, true, Some(RiskLevel::Dangerous)),
            Err(AtctlError::RiskAckMismatch { .. })
        ));
    }

    #[test]
    fn yes_without_risk_ack_is_rejected() {
        let classification = classify_direct_command("AT+CGDCONT=1,\"IP\",\"soracom.io\"");

        assert_eq!(classification.risk, RiskLevel::Write);
        assert!(matches!(
            validate_direct_send_ack(&classification, true, None),
            Err(AtctlError::MissingRiskAck { .. })
        ));
    }

    #[test]
    fn risky_command_without_yes_requires_interactive_confirmation() {
        let classification = classify_direct_command("AT+CGDCONT=1,\"IP\",\"soracom.io\"");

        assert_eq!(
            direct_send_confirmation(&classification, false, None).unwrap(),
            DirectSendConfirmation::InteractiveRequired {
                risk: RiskLevel::Write
            }
        );
    }
}
