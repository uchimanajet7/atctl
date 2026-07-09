use std::fs;
use std::io;
use std::path::Path;

use crate::config::model::Config;
use crate::{AtctlError, Result};

pub fn parse_config(input: &str) -> std::result::Result<Config, toml::de::Error> {
    toml::from_str(input)
}

pub fn load_config_if_exists(path: &Path) -> Result<Option<Config>> {
    let input = match fs::read_to_string(path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(AtctlError::ReadFile {
                path: path.display().to_string(),
                source: error,
            });
        }
    };

    parse_config(&input)
        .map(Some)
        .map_err(|source| AtctlError::TomlFile {
            path: path.display().to_string(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_device_config() {
        let config = parse_config(
            r#"
            [device]
            default_vendor_id = "0x2c7c"
            default_product_id = "0x0125"
            default_interface = "auto"
            default_bulk_in = "auto"
            default_bulk_out = "auto"
            "#,
        )
        .unwrap();

        let device = config.device.unwrap();
        assert_eq!(device.default_vendor_id.as_deref(), Some("0x2c7c"));
        assert_eq!(device.default_bulk_in.as_deref(), Some("auto"));
    }

    #[test]
    fn parses_profile_ui_and_log_config() {
        let config = parse_config(
            r#"
            [profile.soracom]
            apn = "soracom.io"
            pdp_type = "IP"
            context_id = 1

            [ui]
            mask_sensitive_values = true
            show_vendor_commands = true

            [log]
            enabled = true
            raw_log_enabled = false
            log_dir = "~/.local/state/atctl/logs"
            "#,
        )
        .unwrap();

        let soracom = config.profile.unwrap().soracom.unwrap();
        assert_eq!(soracom.apn.as_deref(), Some("soracom.io"));
        assert_eq!(soracom.context_id, Some(1));
        assert_eq!(config.ui.unwrap().show_vendor_commands, Some(true));
        assert_eq!(config.log.unwrap().raw_log_enabled, Some(false));
    }

    #[test]
    fn rejects_unknown_config_fields() {
        let error = parse_config(
            r#"
            [ui]
            mask_sensitive_values = true
            unexpected = true
            "#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown field"));
        assert!(error.to_string().contains("unexpected"));
    }
}
