use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::{AtctlError, Result};

pub(crate) fn validate_response_export_target(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(AtctlError::ResponseExportFileExists {
                path: path.display().to_string(),
            });
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(source) => {
            return Err(AtctlError::WriteFile {
                path: path.display().to_string(),
                source,
            });
        }
    }

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        return Err(AtctlError::ResponseExportParentUnavailable {
            path: parent.display().to_string(),
        });
    }

    Ok(())
}

pub(crate) fn write_response_export(path: &Path, contents: &str) -> Result<()> {
    validate_response_export_target(path)?;

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path).map_err(|source| {
        if source.kind() == ErrorKind::AlreadyExists {
            AtctlError::ResponseExportFileExists {
                path: path.display().to_string(),
            }
        } else {
            AtctlError::WriteFile {
                path: path.display().to_string(),
                source,
            }
        }
    })?;
    file.write_all(contents.as_bytes())
        .map_err(|source| AtctlError::WriteFile {
            path: path.display().to_string(),
            source,
        })
}

pub(crate) fn response_export_path(
    directory: &Path,
    response_label: &str,
    timestamp_file_stem: &str,
) -> PathBuf {
    directory.join(format!(
        "{}-{timestamp_file_stem}.response.txt",
        response_label_slug(response_label)
    ))
}

fn response_label_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut pending_separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            pending_separator = false;
        } else if !slug.is_empty() {
            pending_separator = true;
        }

        if slug.len() >= 64 {
            break;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "response".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn response_export_uses_meaningful_exclusive_private_file() {
        let directory = unique_temp_dir("exclusive");
        let path = response_export_path(
            &directory,
            "Command: modem-response",
            "2026-07-13T08-22-19-123456789Z",
        );

        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "command-modem-response-2026-07-13T08-22-19-123456789Z.response.txt"
        );
        write_response_export(&path, "AT\nOK\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "AT\nOK\n");

        let error = write_response_export(&path, "replacement").unwrap_err();
        assert!(matches!(error, AtctlError::ResponseExportFileExists { .. }));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn response_export_requires_existing_parent_directory() {
        let directory = unique_temp_dir("missing-parent");
        let path = directory.join("missing").join("response.txt");

        let error = validate_response_export_target(&path).unwrap_err();
        assert!(matches!(
            error,
            AtctlError::ResponseExportParentUnavailable { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("atctl-response-{name}-{nanos}"));
        fs::create_dir_all(&directory).unwrap();
        directory
    }
}
