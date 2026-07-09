use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtResponse {
    pub raw: Vec<u8>,
    pub text: String,
    pub lines: Vec<String>,
    pub status: AtStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AtStatus {
    Ok,
    Error,
    CmeError(String),
    CmsError(String),
    NoCarrier,
    Timeout,
    Incomplete,
}

impl AtStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok)
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Timeout | Self::Incomplete)
    }
}

impl std::fmt::Display for AtStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => formatter.write_str("OK"),
            Self::Error => formatter.write_str("ERROR"),
            Self::CmeError(line) | Self::CmsError(line) => formatter.write_str(line),
            Self::NoCarrier => formatter.write_str("NO CARRIER"),
            Self::Timeout => formatter.write_str("timeout"),
            Self::Incomplete => formatter.write_str("incomplete"),
        }
    }
}
