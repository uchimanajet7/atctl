use std::time::{Duration, Instant};

use crate::Result;
use crate::at::parser::parse_response;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseMatcher {
    Terminal,
    Contains(String),
    ContainsOrErrorTerminal(String),
    TerminalOrContains(String),
}

impl ResponseMatcher {
    pub fn is_match(&self, raw: &[u8]) -> bool {
        match self {
            Self::Terminal => parse_response(raw).status.is_terminal(),
            Self::Contains(needle) => raw_contains(raw, needle.as_bytes()),
            Self::ContainsOrErrorTerminal(needle) => {
                raw_contains(raw, needle.as_bytes()) || {
                    let status = parse_response(raw).status;
                    status.is_terminal() && !status.is_success()
                }
            }
            Self::TerminalOrContains(needle) => {
                parse_response(raw).status.is_terminal() || raw_contains(raw, needle.as_bytes())
            }
        }
    }
}

pub trait AtTransport {
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn write_command(&mut self, command: &str) -> Result<()>;
    fn read_response(&mut self, timeout: Duration) -> Result<Vec<u8>>;

    fn read_until(&mut self, timeout: Duration, matcher: ResponseMatcher) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        let mut raw = Vec::new();

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(crate::AtctlError::Timeout);
            }

            let chunk = self.read_response(remaining)?;
            if chunk.is_empty() {
                return Err(crate::AtctlError::Timeout);
            }
            raw.extend_from_slice(&chunk);
            if matcher.is_match(&raw) {
                return Ok(raw);
            }
        }
    }
}

fn raw_contains(raw: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && raw
            .windows(needle.len())
            .any(|candidate| candidate == needle)
}
