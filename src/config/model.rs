use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub device: Option<DeviceConfig>,
    pub profile: Option<ProfileConfig>,
    pub ui: Option<UiConfig>,
    pub log: Option<LogConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceConfig {
    pub default_vendor_id: Option<String>,
    pub default_product_id: Option<String>,
    pub default_interface: Option<String>,
    pub default_bulk_in: Option<String>,
    pub default_bulk_out: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileConfig {
    pub soracom: Option<SoracomProfileConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoracomProfileConfig {
    pub apn: Option<String>,
    pub pdp_type: Option<String>,
    pub context_id: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    pub mask_sensitive_values: Option<bool>,
    pub show_vendor_commands: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    pub enabled: Option<bool>,
    pub raw_log_enabled: Option<bool>,
    pub log_dir: Option<String>,
}
