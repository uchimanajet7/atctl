use crate::at::response::AtStatus;
use crate::at::risk::RiskLevel;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AtctlError>;

#[derive(Debug, Error)]
pub enum AtctlError {
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),

    #[error("invalid value for {name}: {value}")]
    InvalidValue { name: &'static str, value: String },

    #[error(
        "cannot resolve the atctl state directory; set XDG_STATE_HOME to an absolute path or HOME to an absolute home directory"
    )]
    StateDirectoryUnavailable,

    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML {path}: {source}")]
    TomlFile {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error(
        "preset not found: {name}\nhelp: run `atctl preset list` with the same `--preset-file` and `--preset-dir` options, then use an exact listed name"
    )]
    PresetNotFound { name: String },

    #[error(
        "sequence not found: {name}\nhelp: run `atctl sequence list` with the same `--sequence-file` and `--sequence-dir` options, then use an exact listed name"
    )]
    SequenceNotFound { name: String },

    #[error(
        "duplicate preset name `{name}` from {duplicate_source}; first definition was from {first_source}\nhelp: rename one duplicate definition, or stop loading one of the conflicting file sources"
    )]
    DuplicatePreset {
        name: String,
        first_source: String,
        duplicate_source: String,
    },

    #[error(
        "duplicate sequence name `{name}` from {duplicate_source}; first definition was from {first_source}\nhelp: rename one duplicate definition, or stop loading one of the conflicting file sources"
    )]
    DuplicateSequence {
        name: String,
        first_source: String,
        duplicate_source: String,
    },

    #[error(
        "sequence `{sequence}` requires parameter `{param}`{hint}\nhelp: rerun with `--param {param}=<VALUE>`"
    )]
    MissingSequenceParam {
        sequence: String,
        param: String,
        hint: String,
    },

    #[error("invalid sequence parameter `{param}`: {reason}")]
    InvalidSequenceParam { param: String, reason: String },

    #[error("sequence `{sequence}` step `{step}` did not produce expected marker `{expected}`")]
    SequenceExpectationFailed {
        sequence: String,
        step: String,
        expected: String,
    },

    #[error("no matching USB device found; run `atctl devices` to inspect visible devices")]
    DeviceNotFound,

    #[error(
        "multiple matching USB devices found; specify --bus and --address or a narrower --vid/--pid selection:\n{devices}"
    )]
    MultipleDevices { devices: String },

    #[error(
        "endpoint auto-detection failed; run `atctl inspect` and retry with --interface, --bulk-in, and --bulk-out"
    )]
    EndpointDetectionFailed,

    #[error(
        "failed to claim USB interface {interface}: {source}. Another process may be using it, the device may have disconnected, permissions may deny access, or a kernel driver may own the interface"
    )]
    InterfaceClaim {
        interface: u8,
        #[source]
        source: rusb::Error,
    },

    #[error("command requires confirmation: risk={risk}")]
    ConfirmationRequired { risk: RiskLevel },

    #[error("direct send requires --risk-ack {risk} when --yes is used")]
    MissingRiskAck { risk: RiskLevel },

    #[error("risk acknowledgement mismatch: classified={classified}, acknowledged={acknowledged}")]
    RiskAckMismatch {
        classified: RiskLevel,
        acknowledged: RiskLevel,
    },

    #[error(
        "raw diagnostic export requires --raw-log-ack raw-log when --raw-log-file is used non-interactively or with --yes"
    )]
    RawLogAckRequired,

    #[error(
        "raw diagnostic export acknowledgement mismatch: expected=raw-log, acknowledged={acknowledged}"
    )]
    RawLogAckMismatch { acknowledged: String },

    #[error(
        "raw diagnostic export file already exists: {path}\nhelp: choose a new file path; existing files are not overwritten"
    )]
    RawLogFileExists { path: String },

    #[error(
        "Response export file already exists: {path}\nhelp: choose a new file path; existing files are not overwritten"
    )]
    ResponseExportFileExists { path: String },

    #[error(
        "Response export parent directory does not exist: {path}\nhelp: choose a path in an existing directory, or create the parent directory first"
    )]
    ResponseExportParentUnavailable { path: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("AT command failed with status {status}")]
    AtCommandFailed { status: AtStatus },

    #[error("AT response timed out")]
    Timeout,
}
