use serde::Serialize;

use crate::at::mask::mask_sensitive_values;
use crate::presets::model::{Preset, PresetOrigin};
use crate::sequences::model::{Sequence, SequenceOrigin};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionSurface {
    Cli,
    Tui,
    Pty,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Command,
    Preset,
    Sequence,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DefinitionOrigin {
    Product,
    File,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefinitionIdentity {
    pub origin: DefinitionOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationIdentity {
    pub kind: OperationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<DefinitionIdentity>,
}

impl OperationIdentity {
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            kind: OperationKind::Command,
            name: None,
            command: Some(command.into()),
            purpose: None,
            definition: None,
        }
    }

    pub fn preset(preset: &Preset) -> Self {
        Self {
            kind: OperationKind::Preset,
            name: Some(preset.name.clone()),
            command: Some(preset.command.clone()),
            purpose: None,
            definition: Some(preset_definition(&preset.origin)),
        }
    }

    pub fn sequence(sequence: &Sequence) -> Self {
        Self {
            kind: OperationKind::Sequence,
            name: Some(sequence.name.clone()),
            command: None,
            purpose: Some(sequence.summary.clone()),
            definition: Some(sequence_definition(&sequence.origin)),
        }
    }

    pub fn masked(&self) -> Self {
        let mut masked = self.clone();
        masked.command = masked.command.map(|value| mask_sensitive_values(&value));
        masked
    }
}

fn preset_definition(origin: &PresetOrigin) -> DefinitionIdentity {
    match origin {
        PresetOrigin::BuiltIn => DefinitionIdentity {
            origin: DefinitionOrigin::Product,
            title: None,
            path: None,
        },
        PresetOrigin::File { title, path, .. } => DefinitionIdentity {
            origin: DefinitionOrigin::File,
            title: Some(title.clone()),
            path: Some(path.clone()),
        },
        PresetOrigin::Runtime { label } => DefinitionIdentity {
            origin: DefinitionOrigin::Runtime,
            title: Some(label.clone()),
            path: None,
        },
    }
}

fn sequence_definition(origin: &SequenceOrigin) -> DefinitionIdentity {
    match origin {
        SequenceOrigin::BuiltIn => DefinitionIdentity {
            origin: DefinitionOrigin::Product,
            title: None,
            path: None,
        },
        SequenceOrigin::File { title, path, .. } => DefinitionIdentity {
            origin: DefinitionOrigin::File,
            title: Some(title.clone()),
            path: Some(path.clone()),
        },
    }
}
