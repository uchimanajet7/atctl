use std::fs;
use std::io;
use std::path::Path;

use serde::de::Error as _;

use crate::at::risk::RiskLevel;
use crate::sequences::definition::{SequenceDefinition, definitions_into_sequences};
use crate::sequences::model::{
    Sequence, SequenceOrigin, SequenceParam, SequenceParamSource, SequenceReviewItem, SequenceStep,
    StepTerminator,
};
use crate::{AtctlError, Result};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SequenceFile {
    title: String,
    description: Option<String>,
    sequences: Vec<TomlSequenceDefinition>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlSequenceDefinition {
    name: String,
    summary: String,
    risk: RiskLevel,
    #[serde(default)]
    categories: Vec<String>,
    timeout_secs: Option<u64>,
    #[serde(default)]
    before_running: Vec<String>,
    #[serde(default)]
    params: Vec<SequenceParamDefinition>,
    #[serde(default)]
    review: Vec<SequenceReviewItemDefinition>,
    #[serde(default)]
    success_notes: Vec<String>,
    steps: Vec<SequenceStepDefinition>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SequenceParamDefinition {
    name: String,
    label: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    sensitive: bool,
    #[serde(default)]
    default: Option<String>,
    #[serde(default)]
    source: SequenceParamSource,
    candidate: Option<crate::sequences::model::SequenceCandidateSource>,
    hint: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SequenceReviewItemDefinition {
    label: String,
    value: String,
    #[serde(default)]
    sensitive: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SequenceStepDefinition {
    id: String,
    label: Option<String>,
    ensure_pdp_context_active: Option<String>,
    send: Option<String>,
    expect: Option<String>,
    expect_prompt: Option<String>,
    expect_urc: Option<String>,
    payload: Option<String>,
    #[serde(default = "default_terminator")]
    terminator: StepTerminator,
    #[serde(default)]
    require_tcp_ack: bool,
    #[serde(default)]
    require_ping_success: bool,
    timeout_secs: Option<u64>,
    evidence: Option<String>,
    cleanup_on_failure: Option<String>,
}

#[cfg(test)]
pub(crate) fn parse_sequences(input: &str) -> std::result::Result<Vec<Sequence>, toml::de::Error> {
    parse_sequences_with_source(input, "<inline>")
}

pub fn parse_sequences_with_source(
    input: &str,
    origin_path: &str,
) -> std::result::Result<Vec<Sequence>, toml::de::Error> {
    let file = toml::from_str::<SequenceFile>(input)?;
    validate_sequence_file(&file)?;
    let origin = SequenceOrigin::file(file.title, origin_path, file.description);
    Ok(definitions_into_sequences(
        file.sequences
            .into_iter()
            .map(TomlSequenceDefinition::into_definition),
        origin,
    ))
}

fn validate_sequence_file(file: &SequenceFile) -> std::result::Result<(), toml::de::Error> {
    for sequence in &file.sequences {
        for step in &sequence.steps {
            validate_step_completion_contract(sequence, step)?;
        }
    }
    Ok(())
}

fn validate_step_completion_contract(
    sequence: &TomlSequenceDefinition,
    step: &SequenceStepDefinition,
) -> std::result::Result<(), toml::de::Error> {
    if step.require_ping_success {
        if step
            .expect
            .as_deref()
            .is_some_and(|expect| expect.trim() == "OK")
        {
            return Err(sequence_validation_error(
                sequence,
                step,
                "require_ping_success must use expect_urc containing \"+QPING:\", not expect = \"OK\", because AT+QPING returns result lines after the command-accepted OK",
            ));
        }
        if !step
            .expect_urc
            .as_deref()
            .is_some_and(|expect_urc| expect_urc.contains("+QPING:"))
        {
            return Err(sequence_validation_error(
                sequence,
                step,
                "require_ping_success requires expect_urc containing \"+QPING:\" so the step waits for ping result lines",
            ));
        }
    }

    if step.require_tcp_ack {
        let has_qisend_marker = step
            .expect
            .as_deref()
            .is_some_and(|expect| expect.contains("+QISEND:"))
            || step
                .expect_urc
                .as_deref()
                .is_some_and(|expect_urc| expect_urc.contains("+QISEND:"));
        if !has_qisend_marker {
            return Err(sequence_validation_error(
                sequence,
                step,
                "require_tcp_ack requires expect or expect_urc containing \"+QISEND:\" so the step reads acknowledgement counters",
            ));
        }
    }

    Ok(())
}

fn sequence_validation_error(
    sequence: &TomlSequenceDefinition,
    step: &SequenceStepDefinition,
    message: &str,
) -> toml::de::Error {
    toml::de::Error::custom(format!(
        "invalid Sequence `{}` step `{}`: {message}",
        sequence.name, step.id
    ))
}

impl TomlSequenceDefinition {
    fn into_definition(self) -> SequenceDefinition {
        SequenceDefinition::new(
            self.name,
            self.summary,
            self.risk,
            self.categories,
            self.timeout_secs,
            self.params
                .into_iter()
                .map(SequenceParamDefinition::into_param)
                .collect(),
            self.steps
                .into_iter()
                .map(SequenceStepDefinition::into_step)
                .collect(),
        )
        .with_before_running(self.before_running)
        .with_review_items(
            self.review
                .into_iter()
                .map(SequenceReviewItemDefinition::into_review_item)
                .collect(),
        )
        .with_success_notes(self.success_notes)
    }
}

impl SequenceParamDefinition {
    fn into_param(self) -> SequenceParam {
        SequenceParam {
            name: self.name,
            label: self.label,
            required: self.required,
            sensitive: self.sensitive,
            default_value: self.default,
            source: self.source,
            candidate: self.candidate,
            hint: self.hint,
        }
    }
}

impl SequenceReviewItemDefinition {
    fn into_review_item(self) -> SequenceReviewItem {
        SequenceReviewItem {
            label: self.label,
            value: self.value,
            sensitive: self.sensitive,
        }
    }
}

impl SequenceStepDefinition {
    fn into_step(self) -> SequenceStep {
        SequenceStep {
            id: self.id,
            label: self.label,
            ensure_pdp_context_active: self.ensure_pdp_context_active,
            send: self.send,
            expect: self.expect,
            expect_prompt: self.expect_prompt,
            expect_urc: self.expect_urc,
            payload: self.payload,
            terminator: self.terminator,
            require_tcp_ack: self.require_tcp_ack,
            require_ping_success: self.require_ping_success,
            timeout_secs: self.timeout_secs,
            evidence: self.evidence,
            cleanup_on_failure: self.cleanup_on_failure,
        }
    }
}

fn default_terminator() -> StepTerminator {
    StepTerminator::None
}

pub fn load_sequences_if_exists(path: &Path) -> Result<Vec<Sequence>> {
    load_sequences_file(path, MissingPathBehavior::Empty)
}

pub fn load_sequences_file_required(path: &Path) -> Result<Vec<Sequence>> {
    load_sequences_file(path, MissingPathBehavior::Error)
}

fn load_sequences_file(path: &Path, missing: MissingPathBehavior) -> Result<Vec<Sequence>> {
    let input = match fs::read_to_string(path) {
        Ok(input) => input,
        Err(error)
            if error.kind() == io::ErrorKind::NotFound && missing == MissingPathBehavior::Empty =>
        {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(AtctlError::ReadFile {
                path: path.display().to_string(),
                source: error,
            });
        }
    };

    parse_sequences_with_source(&input, &origin_path_for_file(path)).map_err(|source| {
        AtctlError::TomlFile {
            path: path.display().to_string(),
            source,
        }
    })
}

#[cfg(test)]
pub fn load_sequences_dir_if_exists(path: &Path) -> Result<Vec<Sequence>> {
    load_sequences_dir(path, MissingPathBehavior::Empty)
}

pub fn load_sequences_dir_required(path: &Path) -> Result<Vec<Sequence>> {
    load_sequences_dir(path, MissingPathBehavior::Error)
}

fn load_sequences_dir(path: &Path, missing: MissingPathBehavior) -> Result<Vec<Sequence>> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error)
            if error.kind() == io::ErrorKind::NotFound && missing == MissingPathBehavior::Empty =>
        {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(AtctlError::ReadFile {
                path: path.display().to_string(),
                source: error,
            });
        }
    };

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| AtctlError::ReadFile {
            path: path.display().to_string(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| AtctlError::ReadFile {
            path: entry.path().display().to_string(),
            source,
        })?;
        if file_type.is_file() && entry.path().extension().is_some_and(|ext| ext == "toml") {
            paths.push(entry.path());
        }
    }

    paths.sort();
    let mut sequences = Vec::new();
    for path in paths {
        sequences.extend(load_sequences_if_exists(&path)?);
    }
    Ok(sequences)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MissingPathBehavior {
    Empty,
    Error,
}

pub fn validate_unique_sequence_names(sequences: &[Sequence]) -> Result<()> {
    let mut seen: std::collections::BTreeMap<&str, String> = std::collections::BTreeMap::new();
    for sequence in sequences {
        let origin_id = sequence.origin.id();
        if let Some(first_source) = seen.insert(&sequence.name, origin_id.clone()) {
            return Err(AtctlError::DuplicateSequence {
                name: sequence.name.clone(),
                first_source,
                duplicate_source: origin_id,
            });
        }
    }
    Ok(())
}

fn origin_path_for_file(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn parses_sequence_file() {
        let sequences = parse_sequences(
            r#"
            title = "Custom Sequences"

            [[sequences]]
            name = "custom-sequence"
            summary = "Custom check."
            risk = "write"
            categories = ["data"]
            timeout_secs = 180
            success_notes = ["Custom note for {{payload}}."]

            [[sequences.params]]
            name = "payload"
            label = "Payload"
            required = true
            sensitive = true
            source = "user"
            hint = "Enter the payload to send."

            [[sequences.review]]
            label = "Payload"
            value = "{{payload}}"
            sensitive = true

            [[sequences.steps]]
            id = "send"
            ensure_pdp_context_active = "1"
            send = "AT+EXAMPLE={{payload}}"
            expect = "OK"
            timeout_secs = 30
            evidence = "Custom evidence for {{payload}}."
            cleanup_on_failure = "AT+EXAMPLECLOSE"
            "#,
        )
        .unwrap();

        assert_eq!(sequences.len(), 1);
        assert_eq!(sequences[0].name, "custom-sequence");
        assert_eq!(sequences[0].origin.label(), "Custom Sequences");
        assert_eq!(sequences[0].params[0].name, "payload");
        assert_eq!(sequences[0].params[0].source, SequenceParamSource::User);
        assert_eq!(
            sequences[0].params[0].hint.as_deref(),
            Some("Enter the payload to send.")
        );
        assert_eq!(sequences[0].review_items[0].label, "Payload");
        assert_eq!(
            sequences[0].success_notes[0],
            "Custom note for {{payload}}."
        );
        assert_eq!(
            sequences[0].steps[0].evidence.as_deref(),
            Some("Custom evidence for {{payload}}.")
        );
        assert_eq!(
            sequences[0].steps[0].ensure_pdp_context_active.as_deref(),
            Some("1")
        );
        assert_eq!(
            sequences[0].steps[0].cleanup_on_failure.as_deref(),
            Some("AT+EXAMPLECLOSE")
        );
        assert_eq!(sequences[0].risk, RiskLevel::Unknown);
    }

    #[test]
    fn rejects_ping_success_steps_that_only_wait_for_ok() {
        let error = parse_sequences(
            r#"
            title = "Custom Sequences"

            [[sequences]]
            name = "bad-ping"
            summary = "Bad ping."
            risk = "write"

            [[sequences.steps]]
            id = "ping"
            send = "AT+QPING=1,\"example.com\",4,4"
            expect = "OK"
            require_ping_success = true
            "#,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("invalid Sequence `bad-ping` step `ping`"));
        assert!(error.contains("expect_urc"));
        assert!(error.contains("+QPING:"));
    }

    #[test]
    fn rejects_tcp_ack_steps_without_qisend_counter_marker() {
        let error = parse_sequences(
            r#"
            title = "Custom Sequences"

            [[sequences]]
            name = "bad-tcp-ack"
            summary = "Bad TCP ack."
            risk = "write"

            [[sequences.steps]]
            id = "ack"
            send = "AT+QISEND=0,0"
            expect = "OK"
            require_tcp_ack = true
            "#,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("invalid Sequence `bad-tcp-ack` step `ack`"));
        assert!(error.contains("+QISEND:"));
    }

    #[test]
    fn loads_drop_in_sequences_in_lexicographic_order() {
        let dir = unique_temp_dir("sequences-dir");
        fs::write(
            dir.join("20-second.toml"),
            r#"
            title = "Second Sequences"

            [[sequences]]
            name = "second"
            summary = "Second."
            risk = "safe"

            [[sequences.steps]]
            id = "at"
            send = "AT"
            expect = "OK"
            "#,
        )
        .unwrap();
        fs::write(
            dir.join("10-first.toml"),
            r#"
            title = "First Sequences"

            [[sequences]]
            name = "first"
            summary = "First."
            risk = "safe"

            [[sequences.steps]]
            id = "at"
            send = "AT"
            expect = "OK"
            "#,
        )
        .unwrap();

        let sequences = load_sequences_dir_if_exists(&dir).unwrap();

        assert_eq!(
            sequences
                .iter()
                .map(|sequence| sequence.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
    }

    #[test]
    fn rejects_duplicate_sequence_names() {
        let sequences = vec![
            Sequence::built_in(
                "sms-send-check",
                "A.",
                RiskLevel::Write,
                Vec::new(),
                None,
                Vec::new(),
                vec![SequenceStep {
                    id: "at".to_owned(),
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
            ),
            Sequence::new(
                "sms-send-check",
                "B.",
                RiskLevel::Safe,
                Vec::new(),
                SequenceOrigin::file("Custom Sequences", "custom.toml", None),
                None,
                Vec::new(),
                vec![SequenceStep {
                    id: "at".to_owned(),
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
            ),
        ];

        assert!(matches!(
            validate_unique_sequence_names(&sequences),
            Err(AtctlError::DuplicateSequence { name, .. }) if name == "sms-send-check"
        ));
    }

    #[test]
    fn repository_example_sequences_load_through_drop_in_loader() {
        let dir = unique_temp_dir("example-sequences");
        fs::write(
            dir.join("10-quectel.toml"),
            include_str!("../../examples/sequences/quectel.toml"),
        )
        .unwrap();
        fs::write(
            dir.join("20-soracom.toml"),
            include_str!("../../examples/sequences/soracom.toml"),
        )
        .unwrap();

        let sequences = load_sequences_dir_if_exists(&dir).unwrap();

        assert!(sequences.iter().any(|sequence| {
            sequence.name == "quectel-tcp-send-check"
                && sequence.origin.label() == "Quectel Sequences"
                && sequence.risk == RiskLevel::Write
        }));
        assert!(
            sequences
                .iter()
                .any(|sequence| sequence.name == "quectel-ping-check")
        );
        assert!(
            sequences
                .iter()
                .any(|sequence| sequence.name == "soracom-ping-check")
        );
        assert!(sequences.iter().any(|sequence| {
            sequence.name == "soracom-ping-check"
                && sequence.steps.iter().any(|step| {
                    step.require_ping_success && step.expect_urc.as_deref() == Some("+QPING:")
                })
        }));
        assert!(sequences.iter().any(|sequence| {
            sequence.name == "soracom-unified-endpoint-tcp-send-check"
                && sequence.origin.label() == "SORACOM Sequences"
                && sequence.risk == RiskLevel::Write
        }));
        assert!(
            !sequences
                .iter()
                .any(|sequence| sequence.name == "soracom-beam-tcp-test-echo-check")
        );
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("atctl-sequences-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
