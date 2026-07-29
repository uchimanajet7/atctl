use crate::presets::definition::{PresetDefinition, definitions_into_presets};
use crate::presets::model::{Preset, PresetOrigin};

pub fn builtins() -> Vec<Preset> {
    definitions_into_presets(builtin_definitions(), PresetOrigin::BuiltIn)
}

fn builtin_definitions() -> Vec<PresetDefinition> {
    vec![
        preset_definition("modem-response", "AT", ["basic"]),
        preset_definition("modem-info", "ATI", ["identity"]),
        preset_definition("manufacturer", "AT+CGMI", ["identity"]),
        preset_definition("model", "AT+CGMM", ["identity"]),
        preset_definition("firmware-revision", "AT+CGMR", ["identity"]),
        preset_definition("imei", "AT+CGSN", ["identity"]),
        preset_definition("sim-pin-status", "AT+CPIN?", ["sim"]),
        preset_definition("imsi", "AT+CIMI", ["sim"]),
        preset_definition("radio-stack", "AT+WS46?", ["network"]),
        preset_definition("radio-stack-capabilities", "AT+WS46=?", ["network"]),
        preset_definition("current-operator", "AT+COPS?", ["network"]),
        preset_definition_with_timeout("available-operators", "AT+COPS=?", ["network"], 240),
        preset_definition("operator-format-numeric", "AT+COPS=3,2", ["network"]),
        preset_definition("operator-auto-selection", "AT+COPS=0", ["network"]),
        preset_definition("circuit-registration", "AT+CREG?", ["network"]),
        preset_definition("gprs-registration", "AT+CGREG?", ["network"]),
        preset_definition("eps-registration", "AT+CEREG?", ["network"]),
        preset_definition(
            "enable-circuit-registration-detail",
            "AT+CREG=2",
            ["network"],
        ),
        preset_definition("enable-gprs-registration-detail", "AT+CGREG=2", ["network"]),
        preset_definition("enable-eps-registration-detail", "AT+CEREG=2", ["network"]),
        preset_definition("enable-eps-registration-cause", "AT+CEREG=3", ["network"]),
        preset_definition(
            "enable-eps-registration-extended",
            "AT+CEREG=5",
            ["network"],
        ),
        preset_definition("signal-quality", "AT+CSQ", ["signal"]),
        preset_definition("extended-signal-quality", "AT+CESQ", ["signal"]),
        preset_definition("extended-signal-capabilities", "AT+CESQ=?", ["signal"]),
        preset_definition("pdp-contexts", "AT+CGDCONT?", ["pdp", "apn"]),
        preset_definition("pdp-auth-settings", "AT+CGAUTH?", ["pdp", "apn"]),
        preset_definition("pdp-auth-capabilities", "AT+CGAUTH=?", ["pdp", "apn"]),
        preset_definition("packet-attach", "AT+CGATT?", ["network", "pdp"]),
        preset_definition("active-pdp-contexts", "AT+CGACT?", ["pdp"]),
        preset_definition("pdp-addresses", "AT+CGPADDR", ["pdp"]),
        preset_definition("pdp-address-capabilities", "AT+CGPADDR=?", ["pdp"]),
        preset_definition("pdp-connection-details", "AT+CGCONTRDP", ["pdp"]),
        preset_definition("extended-error-report", "AT+CEER", ["diagnostics"]),
        preset_definition("error-reporting-status", "AT+CMEE?", ["diagnostics"]),
        preset_definition("enable-verbose-errors", "AT+CMEE=2", ["diagnostics"]),
        preset_definition("modem-activity-status", "AT+CPAS", ["modem", "diagnostics"]),
        preset_definition("sms-service-support", "AT+CSMS?", ["sms"]),
        preset_definition("sms-format", "AT+CMGF?", ["sms"]),
        preset_definition("sms-storage", "AT+CPMS?", ["sms"]),
        preset_definition("modem-functionality", "AT+CFUN?", ["modem"]),
        preset_definition("set-modem-minimum-functionality", "AT+CFUN=0", ["modem"]),
        preset_definition("set-modem-full-functionality", "AT+CFUN=1", ["modem"]),
        preset_definition("restart-modem", "AT+CFUN=1,1", ["modem"]),
        preset_definition("disable-command-echo", "ATE0", ["basic"]),
    ]
}

fn preset_definition<I, S>(
    name: impl Into<String>,
    command: impl Into<String>,
    categories: I,
) -> PresetDefinition
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    PresetDefinition::new(name, command, categories)
}

fn preset_definition_with_timeout<I, S>(
    name: impl Into<String>,
    command: impl Into<String>,
    categories: I,
    timeout_secs: u64,
) -> PresetDefinition
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    PresetDefinition::new(name, command, categories).with_timeout(timeout_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_specified_builtin_presets() {
        let names = builtins()
            .into_iter()
            .map(|preset| preset.name)
            .collect::<Vec<_>>();

        for expected in [
            "modem-response",
            "disable-command-echo",
            "modem-info",
            "manufacturer",
            "model",
            "firmware-revision",
            "imei",
            "sim-pin-status",
            "imsi",
            "radio-stack",
            "radio-stack-capabilities",
            "current-operator",
            "available-operators",
            "operator-format-numeric",
            "operator-auto-selection",
            "circuit-registration",
            "eps-registration",
            "gprs-registration",
            "enable-circuit-registration-detail",
            "enable-gprs-registration-detail",
            "enable-eps-registration-detail",
            "enable-eps-registration-cause",
            "enable-eps-registration-extended",
            "signal-quality",
            "extended-signal-quality",
            "extended-signal-capabilities",
            "pdp-contexts",
            "pdp-auth-settings",
            "pdp-auth-capabilities",
            "packet-attach",
            "active-pdp-contexts",
            "pdp-addresses",
            "pdp-address-capabilities",
            "pdp-connection-details",
            "extended-error-report",
            "error-reporting-status",
            "enable-verbose-errors",
            "modem-activity-status",
            "sms-service-support",
            "sms-format",
            "sms-storage",
            "modem-functionality",
            "set-modem-minimum-functionality",
            "set-modem-full-functionality",
            "restart-modem",
        ] {
            assert!(names.iter().any(|name| name == expected), "{expected}");
        }
    }

    #[test]
    fn builtin_presets_follow_curated_workflow_order() {
        let names = builtins()
            .into_iter()
            .map(|preset| preset.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "modem-response",
                "modem-info",
                "manufacturer",
                "model",
                "firmware-revision",
                "imei",
                "sim-pin-status",
                "imsi",
                "radio-stack",
                "radio-stack-capabilities",
                "current-operator",
                "available-operators",
                "operator-format-numeric",
                "operator-auto-selection",
                "circuit-registration",
                "gprs-registration",
                "eps-registration",
                "enable-circuit-registration-detail",
                "enable-gprs-registration-detail",
                "enable-eps-registration-detail",
                "enable-eps-registration-cause",
                "enable-eps-registration-extended",
                "signal-quality",
                "extended-signal-quality",
                "extended-signal-capabilities",
                "pdp-contexts",
                "pdp-auth-settings",
                "pdp-auth-capabilities",
                "packet-attach",
                "active-pdp-contexts",
                "pdp-addresses",
                "pdp-address-capabilities",
                "pdp-connection-details",
                "extended-error-report",
                "error-reporting-status",
                "enable-verbose-errors",
                "modem-activity-status",
                "sms-service-support",
                "sms-format",
                "sms-storage",
                "modem-functionality",
                "set-modem-minimum-functionality",
                "set-modem-full-functionality",
                "restart-modem",
                "disable-command-echo",
            ]
        );
    }

    #[test]
    fn excludes_vendor_and_carrier_file_presets_from_built_ins() {
        let names = builtins()
            .into_iter()
            .map(|preset| preset.name)
            .collect::<Vec<_>>();

        for not_built_in in [
            "iccid",
            "signal-quectel",
            "network-info-quectel",
            "set-soracom-apn-cid1",
            "serving-cell-quectel",
            "neighbour-cell-quectel",
            "qcfg-list-quectel",
            "sim-init-status-quectel",
            "pin-retries-quectel",
            "network-name-quectel",
            "network-time-quectel",
            "mbn-list-quectel",
            "network-scan-mode-quectel",
            "network-scan-auto-quectel",
            "power-down-quectel",
            "set-soracom-du-apn-cid1",
            "set-soracom-mc-apn-cid1",
            "set-soracom-auth-chap-cid1",
            "set-soracom-auth-pap-cid1",
        ] {
            assert!(
                !names.iter().any(|name| name == not_built_in),
                "{not_built_in} must be loaded from file presets, not built-ins"
            );
        }
    }

    #[test]
    fn available_operators_has_long_timeout_hint() {
        let presets = builtins();
        let preset = presets
            .iter()
            .find(|preset| preset.name == "available-operators")
            .expect("available-operators preset");

        assert_eq!(preset.timeout_secs, Some(240));
    }
}
