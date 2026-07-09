use crate::at::risk::RiskLevel;
use crate::presets::model::{Preset, PresetOrigin};

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetDefinition {
    pub name: String,
    pub command: String,
    pub risk: RiskLevel,
    #[serde(default)]
    pub categories: Vec<String>,
    pub timeout_secs: Option<u64>,
}

impl PresetDefinition {
    pub fn new<I, S>(
        name: impl Into<String>,
        command: impl Into<String>,
        risk: RiskLevel,
        categories: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.into(),
            command: command.into(),
            risk,
            categories: categories.into_iter().map(Into::into).collect(),
            timeout_secs: None,
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }

    pub fn into_preset(self, origin: PresetOrigin) -> Preset {
        Preset::new_with_timeout(
            self.name,
            self.command,
            self.risk,
            self.categories,
            origin,
            self.timeout_secs,
        )
    }
}

pub fn definitions_into_presets(
    definitions: impl IntoIterator<Item = PresetDefinition>,
    origin: PresetOrigin,
) -> Vec<Preset> {
    definitions
        .into_iter()
        .map(|definition| definition.into_preset(origin.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_definition_into_origin_aware_preset() {
        let preset = PresetDefinition::new(
            "available-operators",
            "AT+COPS=?",
            RiskLevel::Safe,
            ["network"],
        )
        .with_timeout(180)
        .into_preset(PresetOrigin::BuiltIn);

        assert_eq!(preset.name, "available-operators");
        assert_eq!(preset.command, "AT+COPS=?");
        assert_eq!(preset.declared_risk, RiskLevel::Safe);
        assert_eq!(preset.origin, PresetOrigin::BuiltIn);
        assert_eq!(preset.categories, vec!["network"]);
        assert_eq!(preset.timeout_secs, Some(180));
    }

    #[test]
    fn converts_multiple_definitions_with_the_same_file_origin() {
        let presets = definitions_into_presets(
            vec![
                PresetDefinition::new("one", "AT", RiskLevel::Safe, ["basic"]),
                PresetDefinition::new("two", "ATI", RiskLevel::Safe, ["identity"]),
            ],
            PresetOrigin::file("Custom commands", "custom.toml", None),
        );

        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].origin.label(), "Custom commands");
        assert_eq!(presets[1].origin.label(), "Custom commands");
    }
}
