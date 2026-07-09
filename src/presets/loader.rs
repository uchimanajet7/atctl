use std::fs;
use std::io;
use std::path::Path;

use crate::presets::definition::{PresetDefinition, definitions_into_presets};
use crate::presets::model::{Preset, PresetOrigin};
use crate::{AtctlError, Result};

#[cfg(test)]
pub(crate) fn parse_presets(input: &str) -> std::result::Result<Vec<Preset>, toml::de::Error> {
    parse_presets_with_source(input, "<inline>")
}

pub fn parse_presets_with_source(
    input: &str,
    origin_path: &str,
) -> std::result::Result<Vec<Preset>, toml::de::Error> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PresetFile {
        title: String,
        description: Option<String>,
        presets: Vec<PresetDefinition>,
    }

    let file = toml::from_str::<PresetFile>(input)?;
    let origin = PresetOrigin::file(file.title, origin_path, file.description);
    Ok(definitions_into_presets(file.presets, origin))
}

pub fn load_presets_if_exists(path: &Path) -> Result<Vec<Preset>> {
    load_presets_file(path, MissingPathBehavior::Empty)
}

pub fn load_presets_file_required(path: &Path) -> Result<Vec<Preset>> {
    load_presets_file(path, MissingPathBehavior::Error)
}

fn load_presets_file(path: &Path, missing: MissingPathBehavior) -> Result<Vec<Preset>> {
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

    parse_presets_with_source(&input, &origin_path_for_file(path)).map_err(|source| {
        AtctlError::TomlFile {
            path: path.display().to_string(),
            source,
        }
    })
}

#[cfg(test)]
pub fn load_presets_dir_if_exists(path: &Path) -> Result<Vec<Preset>> {
    load_presets_dir(path, MissingPathBehavior::Empty)
}

pub fn load_presets_dir_required(path: &Path) -> Result<Vec<Preset>> {
    load_presets_dir(path, MissingPathBehavior::Error)
}

fn load_presets_dir(path: &Path, missing: MissingPathBehavior) -> Result<Vec<Preset>> {
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
    let mut presets = Vec::new();
    for path in paths {
        presets.extend(load_presets_if_exists(&path)?);
    }
    Ok(presets)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MissingPathBehavior {
    Empty,
    Error,
}

pub fn validate_unique_preset_names(presets: &[Preset]) -> Result<()> {
    let mut seen: std::collections::BTreeMap<&str, String> = std::collections::BTreeMap::new();
    for preset in presets {
        let origin_id = preset.origin.id();
        if let Some(first_source) = seen.insert(&preset.name, origin_id.clone()) {
            return Err(AtctlError::DuplicatePreset {
                name: preset.name.clone(),
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

    use crate::at::risk::RiskLevel;

    use super::*;

    #[test]
    fn parses_user_presets() {
        let presets = parse_presets(
            r#"
            title = "Custom commands"

            [[presets]]
            name = "custom-modem-response"
            command = "AT"
            risk = "safe"
            categories = ["custom"]
            "#,
        )
        .unwrap();

        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "custom-modem-response");
        assert_eq!(presets[0].risk, RiskLevel::Safe);
        assert_eq!(presets[0].declared_risk, RiskLevel::Safe);
        assert_eq!(presets[0].origin.label(), "Custom commands");
        assert_eq!(presets[0].categories, vec!["custom"]);
        assert_eq!(presets[0].timeout_secs, None);
    }

    #[test]
    fn parses_optional_timeout_hint() {
        let presets = parse_presets(
            r#"
            title = "Long running commands"

            [[presets]]
            name = "long-scan"
            command = "AT+COPS=?"
            risk = "safe"
            timeout_secs = 180
            "#,
        )
        .unwrap();

        assert_eq!(presets[0].timeout_secs, Some(180));
    }

    #[test]
    fn parses_file_preset_title_as_display_label() {
        let presets = parse_presets(
            r#"
            title = "Quectel commands"
            description = "Quectel-specific commands."

            [[presets]]
            name = "signal-quectel"
            command = "AT+QCSQ"
            risk = "safe"
            categories = ["signal"]
            "#,
        )
        .unwrap();

        assert_eq!(presets[0].origin.label(), "Quectel commands");
        assert_eq!(presets[0].risk, RiskLevel::Safe);
    }

    #[test]
    fn rejects_legacy_tags_field() {
        let error = parse_presets(
            r#"
            title = "Legacy commands"

            [[presets]]
            name = "legacy"
            command = "AT"
            risk = "safe"
            tags = ["basic"]
            "#,
        )
        .unwrap_err();

        assert!(error.message().contains("unknown field `tags`"));
    }

    #[test]
    fn rejects_legacy_source_field() {
        let error = parse_presets(
            r#"
            title = "Legacy commands"
            source = "pack:legacy"

            [[presets]]
            name = "legacy"
            command = "AT"
            risk = "safe"
            categories = ["basic"]
            "#,
        )
        .unwrap_err();

        assert!(error.message().contains("unknown field `source`"));
    }

    #[test]
    fn loads_drop_in_presets_in_lexicographic_order() {
        let dir = unique_temp_dir("presets-dir");
        fs::write(
            dir.join("20-second.toml"),
            r#"
            title = "Second commands"

            [[presets]]
            name = "second"
            command = "AT+CSQ"
            risk = "safe"
            "#,
        )
        .unwrap();
        fs::write(
            dir.join("10-first.toml"),
            r#"
            title = "First commands"

            [[presets]]
            name = "first"
            command = "AT"
            risk = "safe"
            "#,
        )
        .unwrap();

        let presets = load_presets_dir_if_exists(&dir).unwrap();

        assert_eq!(
            presets
                .iter()
                .map(|preset| preset.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(presets[0].origin.label(), "First commands");
    }

    #[test]
    fn rejects_duplicate_preset_names() {
        let presets = vec![
            Preset::built_in(
                "modem-response",
                "AT",
                RiskLevel::Safe,
                Vec::<String>::new(),
            ),
            Preset::new(
                "modem-response",
                "AT+CSQ",
                RiskLevel::Safe,
                Vec::new(),
                PresetOrigin::file("Custom commands", "custom.toml", None),
            ),
        ];

        assert!(matches!(
            validate_unique_preset_names(&presets),
            Err(AtctlError::DuplicatePreset { name, .. }) if name == "modem-response"
        ));
    }

    #[test]
    fn repository_file_presets_load_through_drop_in_loader() {
        let dir = unique_temp_dir("example-presets");
        fs::write(
            dir.join("10-quectel.toml"),
            include_str!("../../examples/presets/quectel.toml"),
        )
        .unwrap();
        fs::write(
            dir.join("20-soracom.toml"),
            include_str!("../../examples/presets/soracom.toml"),
        )
        .unwrap();

        let presets = load_presets_dir_if_exists(&dir).unwrap();

        assert!(presets.iter().any(|preset| {
            preset.name == "signal-quectel" && preset.origin.label() == "Quectel commands"
        }));
        assert!(presets.iter().any(|preset| {
            preset.name == "mbn-list-quectel"
                && preset.origin.label() == "Quectel commands"
                && preset.risk == RiskLevel::Sensitive
        }));
        assert!(presets.iter().any(|preset| {
            preset.name == "network-scan-auto-quectel"
                && preset.origin.label() == "Quectel commands"
                && preset.risk == RiskLevel::Persistent
        }));
        assert!(presets.iter().any(|preset| {
            preset.name == "set-soracom-apn-cid1"
                && preset.origin.label() == "SORACOM commands"
                && preset.risk == RiskLevel::Write
        }));
        assert!(presets.iter().any(|preset| {
            preset.name == "set-soracom-auth-chap-cid1"
                && preset.origin.label() == "SORACOM commands"
                && preset.risk == RiskLevel::Write
        }));
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("atctl-loader-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
