use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::at::mask::mask_sensitive_values;
use crate::log::session::{CommandLogRecord, create_private_dir_all};
use crate::{AtctlError, Result};

#[derive(Debug, Clone)]
pub struct LogListing {
    pub kind: LogListingKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogListingKind {
    History,
    Session,
}

#[derive(Debug, Serialize)]
struct HistoryLine<'a> {
    timestamp: &'a str,
    source: &'a str,
    command: String,
    risk: crate::at::risk::RiskLevel,
    status: &'a crate::at::response::AtStatus,
    duration_ms: u128,
    masked: bool,
    raw_log: bool,
    device: &'a crate::log::session::LogDeviceSelection,
}

pub fn append_command_history(state_dir: &Path, record: &CommandLogRecord) -> Result<PathBuf> {
    create_private_dir_all(state_dir)?;
    let path = history_path(state_dir);
    let line = HistoryLine {
        timestamp: record.timestamp.display(),
        source: &record.source,
        command: mask_sensitive_values(&record.command),
        risk: record.risk,
        status: &record.status,
        duration_ms: record.duration.as_millis(),
        masked: true,
        raw_log: false,
        device: &record.device,
    };
    let json = serde_json::to_string(&line)?;

    append_private_line(&path, &json)?;
    Ok(path)
}

pub fn history_path(state_dir: &Path) -> PathBuf {
    state_dir.join("history.jsonl")
}

pub fn list_logs(state_dir: &Path, session_dir: &Path) -> Result<Vec<LogListing>> {
    let mut listings = Vec::new();
    let history = history_path(state_dir);
    if history.is_file() {
        listings.push(LogListing {
            kind: LogListingKind::History,
            path: history,
        });
    }

    if session_dir.is_dir() {
        let mut session_logs = fs::read_dir(session_dir)
            .map_err(|source| AtctlError::ReadFile {
                path: session_dir.display().to_string(),
                source,
            })?
            .map(|entry| {
                entry
                    .map_err(|source| AtctlError::ReadFile {
                        path: session_dir.display().to_string(),
                        source,
                    })
                    .map(|entry| entry.path())
            })
            .collect::<Result<Vec<_>>>()?;

        session_logs.retain(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".session.log"))
        });
        session_logs.sort_by(|left, right| right.cmp(left));

        listings.extend(session_logs.into_iter().map(|path| LogListing {
            kind: LogListingKind::Session,
            path,
        }));
    }

    Ok(listings)
}

fn append_private_line(path: &Path, line: &str) -> Result<()> {
    let mut options = OpenOptions::new();
    options.append(true).create(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).map_err(|source| AtctlError::WriteFile {
        path: path.display().to_string(),
        source,
    })?;
    writeln!(file, "{line}").map_err(|source| AtctlError::WriteFile {
        path: path.display().to_string(),
        source,
    })
}

impl std::fmt::Display for LogListingKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::History => formatter.write_str("history"),
            Self::Session => formatter.write_str("session"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::at::response::AtStatus;
    use crate::at::risk::RiskLevel;
    use crate::log::session::{
        CommandLogRecord, LogDeviceSelection, now_timestamp, write_masked_session_log,
    };

    use super::*;

    #[test]
    fn appends_masked_history_without_response_body() {
        let dir = unique_temp_dir("history");
        let record = CommandLogRecord {
            timestamp: now_timestamp(),
            source: "send".to_owned(),
            command: "AT+CIMI".to_owned(),
            risk: RiskLevel::Sensitive,
            status: AtStatus::Ok,
            duration: Duration::from_millis(3),
            response: "\r\n898110001234567\r\nOK\r\n".to_owned(),
            device: LogDeviceSelection {
                requested_vendor_id: None,
                requested_product_id: None,
                requested_bus: None,
                requested_address: None,
                interface: None,
                bulk_in: None,
                bulk_out: None,
            },
        };

        let path = append_command_history(&dir, &record).unwrap();
        let contents = fs::read_to_string(path).unwrap();

        assert!(contents.contains("\"command\":\"AT+CIMI\""));
        assert!(!contents.contains("898110001234567"));
        assert!(!contents.contains("response"));
        assert!(contents.contains("\"raw_log\":false"));
    }

    #[test]
    fn lists_history_and_session_logs_without_creating_missing_files() {
        let state_dir = unique_temp_dir("list-state");
        let session_dir = state_dir.join("logs");
        let record = CommandLogRecord {
            timestamp: now_timestamp(),
            source: "send".to_owned(),
            command: "AT".to_owned(),
            risk: RiskLevel::Safe,
            status: AtStatus::Ok,
            duration: Duration::from_millis(1),
            response: "\r\nOK\r\n".to_owned(),
            device: LogDeviceSelection {
                requested_vendor_id: None,
                requested_product_id: None,
                requested_bus: None,
                requested_address: None,
                interface: None,
                bulk_in: None,
                bulk_out: None,
            },
        };

        append_command_history(&state_dir, &record).unwrap();
        write_masked_session_log(&session_dir, &record).unwrap();

        let listings = list_logs(&state_dir, &session_dir).unwrap();

        assert_eq!(listings.len(), 2);
        assert_eq!(listings[0].kind, LogListingKind::History);
        assert_eq!(listings[1].kind, LogListingKind::Session);
    }

    #[test]
    fn lists_session_logs_newest_first_after_history() {
        let state_dir = unique_temp_dir("list-newest");
        let session_dir = state_dir.join("logs");
        let record = CommandLogRecord {
            timestamp: now_timestamp(),
            source: "send".to_owned(),
            command: "AT".to_owned(),
            risk: RiskLevel::Safe,
            status: AtStatus::Ok,
            duration: Duration::from_millis(1),
            response: "\r\nOK\r\n".to_owned(),
            device: LogDeviceSelection {
                requested_vendor_id: None,
                requested_product_id: None,
                requested_bus: None,
                requested_address: None,
                interface: None,
                bulk_in: None,
                bulk_out: None,
            },
        };

        append_command_history(&state_dir, &record).unwrap();
        fs::create_dir_all(&session_dir).unwrap();
        fs::write(
            session_dir.join("2026-06-18T01-27-14-928781000Z.session.log"),
            "old",
        )
        .unwrap();
        fs::write(
            session_dir.join("2026-06-18T04-02-06-343794000Z.session.log"),
            "new",
        )
        .unwrap();
        fs::write(session_dir.join("ignored.txt"), "ignored").unwrap();

        let listings = list_logs(&state_dir, &session_dir).unwrap();
        let file_names = listings
            .iter()
            .map(|listing| {
                listing
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            file_names,
            vec![
                "history.jsonl",
                "2026-06-18T04-02-06-343794000Z.session.log",
                "2026-06-18T01-27-14-928781000Z.session.log",
            ]
        );
        assert_eq!(listings[0].kind, LogListingKind::History);
        assert_eq!(listings[1].kind, LogListingKind::Session);
        assert_eq!(listings[2].kind, LogListingKind::Session);
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
