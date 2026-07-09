use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::at::mask::mask_sensitive_values;
use crate::at::response::AtStatus;
use crate::at::risk::RiskLevel;
use crate::{AtctlError, Result};

#[derive(Debug, Clone, Serialize)]
pub struct LogDeviceSelection {
    pub requested_vendor_id: Option<String>,
    pub requested_product_id: Option<String>,
    pub requested_bus: Option<u8>,
    pub requested_address: Option<u8>,
    pub interface: Option<u8>,
    pub bulk_in: Option<String>,
    pub bulk_out: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommandLogRecord {
    pub timestamp: LogTimestamp,
    pub source: String,
    pub command: String,
    pub risk: RiskLevel,
    pub status: AtStatus,
    pub duration: Duration,
    pub response: String,
    pub device: LogDeviceSelection,
}

#[derive(Debug, Clone)]
pub struct LogTimestamp {
    display: String,
    file_stem: String,
}

#[derive(Debug, Serialize)]
struct SessionLogFile<'a> {
    timestamp: &'a str,
    source: &'a str,
    command: String,
    risk: RiskLevel,
    status: &'a AtStatus,
    duration_ms: u128,
    masked: bool,
    raw_log: bool,
    device: &'a LogDeviceSelection,
    response: String,
}

pub fn now_timestamp() -> LogTimestamp {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    timestamp_from_unix(duration.as_secs() as i64, duration.subsec_nanos())
}

impl LogTimestamp {
    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn file_stem(&self) -> &str {
        &self.file_stem
    }
}

pub fn write_masked_session_log(log_dir: &Path, record: &CommandLogRecord) -> Result<PathBuf> {
    create_private_dir_all(log_dir)?;
    let path = log_dir.join(format!("{}.session.log", record.timestamp.file_stem));
    let file = SessionLogFile {
        timestamp: &record.timestamp.display,
        source: &record.source,
        command: mask_sensitive_values(&record.command),
        risk: record.risk,
        status: &record.status,
        duration_ms: record.duration.as_millis(),
        masked: true,
        raw_log: false,
        device: &record.device,
        response: mask_sensitive_values(&record.response),
    };
    let line = serde_json::to_string_pretty(&file)?;

    write_private_file(&path, format!("{line}\n").as_bytes())?;
    Ok(path)
}

pub fn create_private_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| AtctlError::CreateDir {
        path: path.display().to_string(),
        source,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
            AtctlError::WriteFile {
                path: path.display().to_string(),
                source,
            }
        })?;
    }

    Ok(())
}

pub fn write_private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).map_err(|source| AtctlError::WriteFile {
        path: path.display().to_string(),
        source,
    })?;
    file.write_all(bytes)
        .map_err(|source| AtctlError::WriteFile {
            path: path.display().to_string(),
            source,
        })
}

fn timestamp_from_unix(seconds: i64, nanos: u32) -> LogTimestamp {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    let display = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z");
    let file_stem =
        format!("{year:04}-{month:02}-{day:02}T{hour:02}-{minute:02}-{second:02}-{nanos:09}Z");

    LogTimestamp { display, file_stem }
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };

    (year, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn masks_response_in_session_log() {
        let dir = unique_temp_dir("session-log");
        let record = CommandLogRecord {
            timestamp: timestamp_from_unix(1_735_689_600, 1),
            source: "send".to_owned(),
            command: "AT+CIMI".to_owned(),
            risk: RiskLevel::Sensitive,
            status: AtStatus::Ok,
            duration: Duration::from_millis(12),
            response: "\r\n898110001234567\r\nOK\r\n".to_owned(),
            device: LogDeviceSelection {
                requested_vendor_id: Some("0x2c7c".to_owned()),
                requested_product_id: Some("0x0125".to_owned()),
                requested_bus: None,
                requested_address: None,
                interface: None,
                bulk_in: None,
                bulk_out: None,
            },
        };

        let path = write_masked_session_log(&dir, &record).unwrap();
        let contents = fs::read_to_string(path).unwrap();

        assert!(contents.contains("89811000*******"));
        assert!(!contents.contains("898110001234567"));
        assert!(contents.contains("\"raw_log\": false"));
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "atctl-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
