use crate::at::risk::{RiskLevel, classify_direct_command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub name: String,
    pub command: String,
    pub declared_risk: RiskLevel,
    pub risk: RiskLevel,
    pub origin: PresetOrigin,
    pub categories: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetOrigin {
    BuiltIn,
    File {
        title: String,
        path: String,
        description: Option<String>,
    },
    Runtime {
        label: String,
    },
}

impl PresetOrigin {
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

    pub fn runtime(label: impl Into<String>) -> Self {
        Self::Runtime {
            label: label.into(),
        }
    }

    pub fn id(&self) -> String {
        match self {
            Self::BuiltIn => "built-in".to_owned(),
            Self::File { path, .. } => format!("file:{path}"),
            Self::Runtime { label } => format!("runtime:{label}"),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::BuiltIn => "Product presets",
            Self::File { title, .. } => title,
            Self::Runtime { label } => label,
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::File { title, .. } => Some(title),
            Self::BuiltIn | Self::Runtime { .. } => None,
        }
    }

    pub fn file_path(&self) -> Option<&str> {
        match self {
            Self::File { path, .. } => Some(path),
            Self::BuiltIn | Self::Runtime { .. } => None,
        }
    }

    pub fn is_built_in(&self) -> bool {
        matches!(self, Self::BuiltIn)
    }

    pub fn sort_key(&self) -> (u8, String) {
        let kind = match self {
            Self::BuiltIn => 0,
            Self::File { .. } => 1,
            Self::Runtime { .. } => 2,
        };
        (kind, self.label().to_ascii_lowercase())
    }
}

impl Preset {
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        declared_risk: RiskLevel,
        categories: Vec<String>,
        origin: PresetOrigin,
    ) -> Self {
        Self::new_with_timeout(name, command, declared_risk, categories, origin, None)
    }

    pub fn new_with_timeout(
        name: impl Into<String>,
        command: impl Into<String>,
        declared_risk: RiskLevel,
        categories: Vec<String>,
        origin: PresetOrigin,
        timeout_secs: Option<u64>,
    ) -> Self {
        let command = command.into();
        let classified_risk = classify_direct_command(&command).risk;
        Self {
            name: name.into(),
            command,
            declared_risk,
            risk: declared_risk.stricter(classified_risk),
            origin,
            categories,
            timeout_secs,
        }
    }

    #[cfg(test)]
    pub(crate) fn built_in<I, S>(
        name: impl Into<String>,
        command: impl Into<String>,
        declared_risk: RiskLevel,
        categories: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            name,
            command,
            declared_risk,
            categories.into_iter().map(Into::into).collect(),
            PresetOrigin::BuiltIn,
        )
    }

    pub fn ad_hoc(command: impl Into<String>) -> Self {
        let command = command.into();
        let declared_risk = classify_direct_command(&command).risk;
        Self::new(
            "ad-hoc",
            command,
            declared_risk,
            vec!["ad-hoc".to_owned()],
            PresetOrigin::runtime("ad-hoc"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_risk_cannot_downgrade_classifier() {
        let preset = Preset::new(
            "unsafe-toml",
            "AT+CGDCONT=1,\"IP\",\"example\"",
            RiskLevel::Safe,
            Vec::new(),
            PresetOrigin::file("Test presets", "presets.toml", None),
        );

        assert_eq!(preset.declared_risk, RiskLevel::Safe);
        assert_eq!(preset.risk, RiskLevel::Write);
    }

    #[test]
    fn declared_stricter_risk_is_preserved() {
        let preset = Preset::new(
            "write-labelled-at",
            "AT",
            RiskLevel::Write,
            Vec::new(),
            PresetOrigin::file("Test presets", "presets.toml", None),
        );

        assert_eq!(preset.risk, RiskLevel::Write);
    }
}
