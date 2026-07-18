use crate::at::review::{ExecutionReview, review_direct_command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    pub name: String,
    pub command: String,
    pub review: ExecutionReview,
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
        categories: Vec<String>,
        origin: PresetOrigin,
    ) -> Self {
        Self::new_with_timeout(name, command, categories, origin, None)
    }

    pub fn new_with_timeout(
        name: impl Into<String>,
        command: impl Into<String>,
        categories: Vec<String>,
        origin: PresetOrigin,
        timeout_secs: Option<u64>,
    ) -> Self {
        let command = command.into();
        let review = review_direct_command(&command);
        Self {
            name: name.into(),
            command,
            review,
            origin,
            categories,
            timeout_secs,
        }
    }

    #[cfg(test)]
    pub(crate) fn built_in<I, S>(
        name: impl Into<String>,
        command: impl Into<String>,
        categories: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            name,
            command,
            categories.into_iter().map(Into::into).collect(),
            PresetOrigin::BuiltIn,
        )
    }

    pub fn ad_hoc(command: impl Into<String>) -> Self {
        let command = command.into();
        Self::new(
            "ad-hoc",
            command,
            vec!["ad-hoc".to_owned()],
            PresetOrigin::runtime("ad-hoc"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::at::review::{Assessment, Effect};

    #[test]
    fn definition_metadata_cannot_override_product_review() {
        let preset = Preset::new(
            "file-preset",
            "AT+CGDCONT=1,\"IP\",\"example\"",
            Vec::new(),
            PresetOrigin::file("Test presets", "presets.toml", None),
        );

        assert_eq!(preset.review.assessment, Assessment::Known);
        assert!(
            preset
                .review
                .effects
                .contains(&Effect::ChangePersistentState)
        );
        assert!(preset.review.confirmation_required);
    }

    #[test]
    fn ad_hoc_preset_uses_the_same_unknown_review_contract() {
        let preset = Preset::new(
            "unknown-file-command",
            "AT+VENDOR=1",
            Vec::new(),
            PresetOrigin::file("Test presets", "presets.toml", None),
        );

        assert_eq!(preset.review.assessment, Assessment::Unknown);
        assert!(preset.review.confirmation_required);
    }
}
