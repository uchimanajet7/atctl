use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::at::response::AtStatus;
use crate::at::risk::RiskLevel;
use crate::log::session::now_timestamp;
use crate::{AtctlError, Result};

pub const RAW_LOG_ACK: &str = "raw-log";
pub const RAW_LOG_WARNING: &str = "This raw diagnostic export may contain sensitive modem, subscriber, network, APN, or PDP authentication values.";

#[derive(Debug, Clone)]
pub struct RawLogConfig {
    path: PathBuf,
    surface: String,
    source: String,
}

impl RawLogConfig {
    pub fn new(
        path: impl Into<PathBuf>,
        surface: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            surface: surface.into(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawLogSink {
    path: PathBuf,
    surface: String,
    source: String,
    next_sequence: u64,
}

impl RawLogSink {
    pub fn create(config: RawLogConfig) -> Result<Self> {
        validate_raw_log_target(&config.path)?;
        let sink = Self {
            path: config.path,
            surface: config.surface,
            source: config.source,
            next_sequence: 1,
        };
        sink.create_header()?;
        Ok(sink)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append_exchange(&mut self, exchange: RawLogExchange<'_>) -> Result<()> {
        let event = RawLogLine::Exchange {
            schema_version: 1,
            sequence: self.next_sequence,
            timestamp: now_timestamp().display().to_owned(),
            surface: &self.surface,
            source: &self.source,
            command_name: exchange.command_name,
            command: exchange.command,
            risk: exchange.risk,
            status: exchange.status,
            duration_ms: exchange.duration.as_millis(),
            tx_base64: base64_encode(exchange.tx_bytes),
            tx_preview: bytes_preview(exchange.tx_bytes),
            rx_base64: base64_encode(exchange.rx_bytes),
            rx_preview: bytes_preview(exchange.rx_bytes),
        };
        append_json_line(&self.path, &event)?;
        self.next_sequence += 1;
        Ok(())
    }

    pub fn append_transport_error(&mut self, error: RawLogTransportError<'_>) -> Result<()> {
        let event = RawLogLine::TransportError {
            schema_version: 1,
            sequence: self.next_sequence,
            timestamp: now_timestamp().display().to_owned(),
            surface: &self.surface,
            source: &self.source,
            command_name: error.command_name,
            command: error.command,
            risk: error.risk,
            duration_ms: error.duration.as_millis(),
            stage: error.stage,
            error: error.error,
            tx_base64: base64_encode(error.tx_bytes),
            tx_preview: bytes_preview(error.tx_bytes),
            rx_base64: base64_encode(error.rx_bytes),
            rx_preview: bytes_preview(error.rx_bytes),
        };
        append_json_line(&self.path, &event)?;
        self.next_sequence += 1;
        Ok(())
    }

    fn create_header(&self) -> Result<()> {
        let header = RawLogLine::Header {
            schema_version: 1,
            created_at: now_timestamp().display().to_owned(),
            surface: &self.surface,
            source: &self.source,
            warning: RAW_LOG_WARNING,
        };
        create_json_line(&self.path, &header)
    }
}

pub fn validate_raw_log_target(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(AtctlError::RawLogFileExists {
            path: path.display().to_string(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AtctlError::WriteFile {
            path: path.display().to_string(),
            source: error,
        }),
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RawLogExchange<'a> {
    pub command_name: Option<&'a str>,
    pub command: &'a str,
    pub risk: RiskLevel,
    pub status: &'a AtStatus,
    pub duration: Duration,
    pub tx_bytes: &'a [u8],
    pub rx_bytes: &'a [u8],
}

#[derive(Debug, Copy, Clone)]
pub struct RawLogTransportError<'a> {
    pub command_name: Option<&'a str>,
    pub command: &'a str,
    pub risk: RiskLevel,
    pub duration: Duration,
    pub stage: &'a str,
    pub error: &'a str,
    pub tx_bytes: &'a [u8],
    pub rx_bytes: &'a [u8],
}

#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum RawLogLine<'a> {
    Header {
        schema_version: u8,
        created_at: String,
        surface: &'a str,
        source: &'a str,
        warning: &'static str,
    },
    Exchange {
        schema_version: u8,
        sequence: u64,
        timestamp: String,
        surface: &'a str,
        source: &'a str,
        command_name: Option<&'a str>,
        command: &'a str,
        risk: RiskLevel,
        status: &'a AtStatus,
        duration_ms: u128,
        tx_base64: String,
        tx_preview: String,
        rx_base64: String,
        rx_preview: String,
    },
    TransportError {
        schema_version: u8,
        sequence: u64,
        timestamp: String,
        surface: &'a str,
        source: &'a str,
        command_name: Option<&'a str>,
        command: &'a str,
        risk: RiskLevel,
        duration_ms: u128,
        stage: &'a str,
        error: &'a str,
        tx_base64: String,
        tx_preview: String,
        rx_base64: String,
        rx_preview: String,
    },
}

pub fn require_raw_log_ack(ack: Option<&str>) -> Result<()> {
    match ack {
        Some(RAW_LOG_ACK) => Ok(()),
        Some(value) => Err(AtctlError::RawLogAckMismatch {
            acknowledged: value.to_owned(),
        }),
        None => Err(AtctlError::RawLogAckRequired),
    }
}

fn create_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let line = serde_json::to_string(value)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::AlreadyExists {
            AtctlError::RawLogFileExists {
                path: path.display().to_string(),
            }
        } else {
            AtctlError::WriteFile {
                path: path.display().to_string(),
                source,
            }
        }
    })?;
    write_line(&mut file, path, &line)
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let line = serde_json::to_string(value)?;
    let mut options = OpenOptions::new();
    options.append(true);
    let mut file = options.open(path).map_err(|source| AtctlError::WriteFile {
        path: path.display().to_string(),
        source,
    })?;
    write_line(&mut file, path, &line)
}

fn write_line(file: &mut std::fs::File, path: &Path, line: &str) -> Result<()> {
    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|source| AtctlError::WriteFile {
            path: path.display().to_string(),
            source,
        })
}

fn bytes_preview(bytes: &[u8]) -> String {
    let mut output = String::new();
    for byte in bytes {
        match byte {
            b'\r' => output.push_str("\\r"),
            b'\n' => output.push_str("\\n"),
            b'\t' => output.push_str("\\t"),
            b'\\' => output.push_str("\\\\"),
            b'"' => output.push_str("\\\""),
            0x20..=0x7e => output.push(*byte as char),
            value => output.push_str(&format!("\\x{value:02X}")),
        }
    }
    output
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::at::response::AtStatus;
    use crate::at::risk::RiskLevel;

    use super::*;

    #[test]
    fn raw_log_sink_writes_header_and_exchange_without_overwrite() {
        let dir = unique_temp_dir("raw-log");
        let path = dir.join("case.rawlog");
        let mut sink = RawLogSink::create(RawLogConfig::new(&path, "send", "send")).unwrap();

        sink.append_exchange(RawLogExchange {
            command_name: None,
            command: "AT+CIMI",
            risk: RiskLevel::Sensitive,
            status: &AtStatus::Ok,
            duration: Duration::from_millis(12),
            tx_bytes: b"AT+CIMI\r",
            rx_bytes: b"\r\n898110001234567\r\nOK\r\n",
        })
        .unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"event\":\"header\""));
        assert!(contents.contains("\"event\":\"exchange\""));
        assert!(contents.contains("\"tx_base64\":\"QVQrQ0lNSQ0=\""));
        assert!(contents.contains("898110001234567"));

        let error = RawLogSink::create(RawLogConfig::new(&path, "send", "send")).unwrap_err();
        assert!(matches!(error, AtctlError::RawLogFileExists { .. }));
    }

    #[test]
    fn raw_log_sink_writes_transport_error_event() {
        let dir = unique_temp_dir("raw-log-error");
        let path = dir.join("case.rawlog");
        let mut sink = RawLogSink::create(RawLogConfig::new(&path, "send", "send")).unwrap();

        sink.append_transport_error(RawLogTransportError {
            command_name: None,
            command: "AT+COPS?",
            risk: RiskLevel::Safe,
            duration: Duration::from_millis(30),
            stage: "read_response",
            error: "AT response timed out",
            tx_bytes: b"AT+COPS?\r",
            rx_bytes: b"",
        })
        .unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"event\":\"transport_error\""));
        assert!(contents.contains("\"stage\":\"read_response\""));
        assert!(contents.contains("\"tx_base64\":\"QVQrQ09QUz8N\""));
    }

    #[test]
    fn raw_log_ack_requires_exact_value() {
        assert!(require_raw_log_ack(Some(RAW_LOG_ACK)).is_ok());
        assert!(matches!(
            require_raw_log_ack(None),
            Err(AtctlError::RawLogAckRequired)
        ));
        assert!(matches!(
            require_raw_log_ack(Some("yes")),
            Err(AtctlError::RawLogAckMismatch { .. })
        ));
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("atctl-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
