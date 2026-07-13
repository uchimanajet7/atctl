use std::ffi::OsString;
use std::path::PathBuf;

use crate::{AtctlError, Result};

pub fn default_state_dir() -> Result<PathBuf> {
    resolve_state_dir(std::env::var_os("XDG_STATE_HOME"), std::env::var_os("HOME"))
}

fn resolve_state_dir(xdg_state_home: Option<OsString>, home: Option<OsString>) -> Result<PathBuf> {
    resolve_state_home(xdg_state_home, home).map(|path| path.join("atctl"))
}

fn resolve_state_home(xdg_state_home: Option<OsString>, home: Option<OsString>) -> Result<PathBuf> {
    if let Some(value) = xdg_state_home.filter(|value| !value.is_empty()) {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Ok(path);
        }
    }

    home.filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(".local/state"))
        .ok_or(AtctlError::StateDirectoryUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_absolute_xdg_state_home() {
        let path = resolve_state_dir(
            Some(OsString::from("/tmp/atctl-state")),
            Some(OsString::from("/Users/example")),
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/atctl-state/atctl"));
    }

    #[test]
    fn uses_home_fallback_when_xdg_state_home_is_unset_empty_or_relative() {
        let home = Some(OsString::from("/Users/example"));

        for xdg in [
            None,
            Some(OsString::new()),
            Some(OsString::from("relative/state")),
        ] {
            assert_eq!(
                resolve_state_dir(xdg, home.clone()).unwrap(),
                PathBuf::from("/Users/example/.local/state/atctl")
            );
        }
    }

    #[test]
    fn rejects_missing_or_relative_home_without_absolute_xdg_state_home() {
        for home in [
            None,
            Some(OsString::new()),
            Some(OsString::from("relative/home")),
        ] {
            assert!(matches!(
                resolve_state_dir(None, home),
                Err(AtctlError::StateDirectoryUnavailable)
            ));
        }
    }
}
