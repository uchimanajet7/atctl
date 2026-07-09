use std::path::PathBuf;

pub fn default_config_path() -> PathBuf {
    config_home_dir().join("atctl/config.toml")
}

pub fn default_config_path_display() -> PathBuf {
    if std::env::var_os("XDG_CONFIG_HOME").is_some() {
        default_config_path()
    } else {
        PathBuf::from("~/.config/atctl/config.toml")
    }
}

pub fn default_state_dir() -> PathBuf {
    state_home_dir().join("atctl")
}

pub fn expand_tilde_path(value: &str) -> PathBuf {
    if value == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from("~"));
    }

    if let Some(rest) = value.strip_prefix("~/") {
        return home_dir().unwrap_or_else(|| PathBuf::from("~")).join(rest);
    }

    PathBuf::from(value)
}

fn config_home_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(".config")
        })
}

fn state_home_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(".local/state")
        })
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_display_keeps_user_facing_tilde_without_xdg() {
        if std::env::var_os("XDG_CONFIG_HOME").is_none() {
            assert_eq!(
                default_config_path_display(),
                PathBuf::from("~/.config/atctl/config.toml")
            );
        }
    }

    #[test]
    fn expands_tilde_paths() {
        let expanded = expand_tilde_path("~/example");

        if let Some(home) = home_dir() {
            assert_eq!(expanded, home.join("example"));
        } else {
            assert_eq!(expanded, PathBuf::from("~/example"));
        }
    }
}
