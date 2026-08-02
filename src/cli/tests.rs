use clap::{CommandFactory, Parser};

use crate::presets::model::PresetOrigin;
use crate::transport::test_support::MockTransport;

use super::*;

#[test]
fn cli_confirmation_output_uses_consequence_hierarchy() {
    let mut known = operation_identity_output_lines(&OperationIdentity::command("AT+CMEE=2"));
    known.extend(execution_review_output_lines(&review_direct_command(
        "AT+CMEE=2",
    )));
    assert_eq!(known.first().map(String::as_str), Some("Type: Command"));
    assert!(known.iter().any(|line| line == "AT command: AT+CMEE=2"));
    assert!(known.iter().any(|line| line == "What will happen:"));
    assert!(!known.iter().any(|line| line.starts_with("Assessment:")));
    assert!(!known.iter().any(|line| line.starts_with("Confirmation:")));

    let mut unknown = operation_identity_output_lines(&OperationIdentity::command("AT+VENDOR=1"));
    unknown.extend(execution_review_output_lines(&review_direct_command(
        "AT+VENDOR=1",
    )));
    let positions = [
        "AT command: AT+VENDOR=1",
        "Effects are not fully known.",
        "Known:",
        "Possible consequences:",
        "Not established:",
    ]
    .map(|expected| {
        unknown
            .iter()
            .position(|line| line == expected)
            .unwrap_or_else(|| panic!("missing {expected:?} in {unknown:?}"))
    });
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn parses_hex_vid_pid_with_or_without_prefix() {
    let cli =
        Cli::try_parse_from(["atctl", "send", "AT", "--vid", "0x2c7c", "--pid", "0125"]).unwrap();

    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };

    assert_eq!(args.usb.vid, Some(0x2c7c));
    assert_eq!(args.usb.pid, Some(0x0125));
}

#[test]
fn parses_endpoint_overrides() {
    let cli = Cli::try_parse_from([
        "atctl",
        "send",
        "AT",
        "--interface",
        "2",
        "--bulk-in",
        "0x85",
        "--bulk-out",
        "04",
    ])
    .unwrap();

    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };

    assert_eq!(args.usb.interface_number, Some(2));
    assert_eq!(args.usb.bulk_in, Some(0x85));
    assert_eq!(args.usb.bulk_out, Some(0x04));
}

#[test]
fn parses_no_log_for_every_normal_logging_surface() {
    let cli = Cli::try_parse_from([
        "atctl",
        "send",
        "AT",
        "--no-log",
        "--raw-log-file",
        "/tmp/case.rawlog",
        "--raw-log-ack",
        "raw-log",
    ])
    .unwrap();
    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };
    assert!(args.no_log);
    assert_eq!(args.raw_log_file, Some(PathBuf::from("/tmp/case.rawlog")));

    let cli = Cli::try_parse_from(["atctl", "preset", "run", "modem-info", "--no-log"]).unwrap();
    let Command::Preset(PresetArgs {
        command: PresetCommand::Run(args),
    }) = cli.command
    else {
        panic!("expected preset run command");
    };
    assert!(args.no_log);

    let cli =
        Cli::try_parse_from(["atctl", "sequence", "run", "sms-receive-check", "--no-log"]).unwrap();
    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::Run(args),
    }) = cli.command
    else {
        panic!("expected sequence run command");
    };
    assert!(args.no_log);

    let cli = Cli::try_parse_from(["atctl", "tui", "--no-log"]).unwrap();
    let Command::Tui(args) = cli.command else {
        panic!("expected tui command");
    };
    assert!(args.no_log);
}

#[test]
fn parses_response_export_for_every_bounded_execution_surface() {
    let target = "/tmp/atctl-response.txt";
    let cli = Cli::try_parse_from(["atctl", "send", "AT", "--export-response", target]).unwrap();
    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };
    assert_eq!(args.export_response, Some(PathBuf::from(target)));

    let cli = Cli::try_parse_from([
        "atctl",
        "preset",
        "run",
        "modem-info",
        "--export-response",
        target,
    ])
    .unwrap();
    let Command::Preset(PresetArgs {
        command: PresetCommand::Run(args),
    }) = cli.command
    else {
        panic!("expected preset run command");
    };
    assert_eq!(args.export_response, Some(PathBuf::from(target)));

    let cli = Cli::try_parse_from([
        "atctl",
        "sequence",
        "run",
        "sms-receive-check",
        "--export-response",
        target,
    ])
    .unwrap();
    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::Run(args),
    }) = cli.command
    else {
        panic!("expected sequence run command");
    };
    assert_eq!(args.export_response, Some(PathBuf::from(target)));

    assert!(
        Cli::try_parse_from([
            "atctl",
            "bridge",
            "--symlink",
            "/tmp/atctl",
            "--export-response",
            target,
        ])
        .is_err()
    );
}

#[test]
fn config_subcommand_is_not_part_of_the_cli() {
    assert!(Cli::try_parse_from(["atctl", "config", "path"]).is_err());
}

#[test]
fn parses_bridge_options() {
    let cli = Cli::try_parse_from([
        "atctl",
        "bridge",
        "--symlink",
        "/tmp/atctl",
        "--replace-symlink",
        "--vid",
        "0x2c7c",
        "--pid",
        "0x0125",
        "--bus",
        "1",
        "--address",
        "4",
        "--interface",
        "2",
        "--bulk-in",
        "0x84",
        "--bulk-out",
        "0x03",
        "--timeout",
        "180",
    ])
    .unwrap();

    let Command::Bridge(args) = cli.command else {
        panic!("expected bridge command");
    };

    assert_eq!(args.symlink, PathBuf::from("/tmp/atctl"));
    assert!(args.replace_symlink);
    assert_eq!(args.usb.vid, Some(0x2c7c));
    assert_eq!(args.usb.pid, Some(0x0125));
    assert_eq!(args.usb.bus, Some(1));
    assert_eq!(args.usb.address, Some(4));
    assert_eq!(args.usb.interface_number, Some(2));
    assert_eq!(args.usb.bulk_in, Some(0x84));
    assert_eq!(args.usb.bulk_out, Some(0x03));
    assert_eq!(args.usb.timeout, Some(180));
}

#[test]
fn bridge_help_describes_runtime_device_discovery() {
    let mut command = Cli::command();
    let bridge = command
        .find_subcommand_mut("bridge")
        .expect("bridge subcommand");
    let mut help = Vec::new();
    bridge.write_long_help(&mut help).unwrap();
    let help = String::from_utf8(help).unwrap();

    assert!(help.contains("Run `atctl devices`"));
    assert!(help.contains("current AT operation target output"));
    assert!(help.contains("--bus <BUS> --address <ADDRESS>"));
    assert!(help.contains("only when that pair is unique"));
    assert!(help.contains("atctl devices --all-usb"));
    assert!(help.contains("115200 is a serial-tool compatibility value"));
    assert!(!help.contains("--export-response"));
}

#[test]
fn help_describes_primary_product_options() {
    let mut command = Cli::command();
    let mut root_help = Vec::new();
    command.write_long_help(&mut root_help).unwrap();
    let root_help = String::from_utf8(root_help).unwrap();
    assert!(root_help.contains("Send one AT command"));
    assert!(root_help.contains("List, inspect, or run product and loaded multi-step Sequences"));
    assert!(root_help.contains("Examples:\n  atctl devices\n  atctl send AT\n  atctl tui"));
    assert!(!root_help.contains("Show configuration paths"));

    let mut command = Cli::command();
    let send = command
        .find_subcommand_mut("send")
        .expect("send subcommand");
    let mut send_help = Vec::new();
    send.write_long_help(&mut send_help).unwrap();
    let send_help = String::from_utf8(send_help).unwrap();
    assert!(send_help.contains("AT command line to send"));
    assert!(send_help.contains("Print unmasked foreground output"));
    assert!(send_help.contains("Do not write masked history or session logs"));
    assert!(send_help.contains("--export-response <PATH>"));
    assert!(send_help.contains("follows --no-mask"));
    assert!(send_help.contains("does not replace stdout"));
    assert!(send_help.contains("Write an acknowledged raw diagnostic export"));

    let mut command = Cli::command();
    let preset = command
        .find_subcommand_mut("preset")
        .expect("preset subcommand");
    let preset_run = preset
        .find_subcommand_mut("run")
        .expect("preset run subcommand");
    let mut preset_run_help = Vec::new();
    preset_run.write_long_help(&mut preset_run_help).unwrap();
    let preset_run_help = String::from_utf8(preset_run_help).unwrap();
    assert!(preset_run_help.contains("Run one loaded preset by name"));
    assert!(preset_run_help.contains("<NAME>"));
    assert!(preset_run_help.contains("--export-response <PATH>"));
    assert!(preset_run_help.contains("follows --no-mask"));
    assert!(!preset_run_help.contains("--continue-on-error"));

    let mut command = Cli::command();
    let sequence = command
        .find_subcommand_mut("sequence")
        .expect("sequence subcommand");
    let sequence_run = sequence
        .find_subcommand_mut("run")
        .expect("sequence run subcommand");
    let mut sequence_run_help = Vec::new();
    sequence_run
        .write_long_help(&mut sequence_run_help)
        .unwrap();
    let sequence_run_help = String::from_utf8(sequence_run_help).unwrap();
    assert!(sequence_run_help.contains("--export-response <PATH>"));
    assert!(sequence_run_help.contains("follows --no-mask"));

    let mut command = Cli::command();
    let tui = command.find_subcommand_mut("tui").expect("tui subcommand");
    let mut tui_help = Vec::new();
    tui.write_long_help(&mut tui_help).unwrap();
    let tui_help = String::from_utf8(tui_help).unwrap();
    assert!(tui_help.contains("Start the TUI session with output masking off"));
    assert!(tui_help.contains("Do not write masked history or session logs"));
    assert!(tui_help.contains("Load Sequence definition TOML files"));
}

#[test]
fn parses_devices_filter_options() {
    let cli = Cli::try_parse_from([
        "atctl",
        "devices",
        "--all-usb",
        "--vid",
        "2c7c",
        "--pid",
        "0125",
        "--bus",
        "20",
        "--address",
        "7",
    ])
    .unwrap();

    let Command::Devices(args) = cli.command else {
        panic!("expected devices command");
    };

    assert!(args.all_usb);
    assert_eq!(args.filter.vid, Some(0x2c7c));
    assert_eq!(args.filter.pid, Some(0x0125));
    assert_eq!(args.filter.bus, Some(20));
    assert_eq!(args.filter.address, Some(7));
}

#[test]
fn rejects_devices_all_compatibility_flag() {
    let error = Cli::try_parse_from(["atctl", "devices", "--all"]).unwrap_err();

    assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
}

#[test]
fn builds_manual_endpoint_override_when_all_values_are_present() {
    let cli = Cli::try_parse_from([
        "atctl",
        "inspect",
        "--interface",
        "2",
        "--bulk-in",
        "0x85",
        "--bulk-out",
        "04",
    ])
    .unwrap();

    let Command::Inspect(args) = cli.command else {
        panic!("expected inspect command");
    };

    let pair = args.manual_endpoint_pair().unwrap().unwrap();
    assert_eq!(pair.interface_number, 2);
    assert_eq!(pair.bulk_in, 0x85);
    assert_eq!(pair.bulk_out, 0x04);
}

#[test]
fn rejects_partial_manual_endpoint_override() {
    let cli = Cli::try_parse_from(["atctl", "inspect", "--bulk-in", "0x85"]).unwrap();

    let Command::Inspect(args) = cli.command else {
        panic!("expected inspect command");
    };

    let error = args.manual_endpoint_pair().unwrap_err();
    assert!(error.to_string().contains("--interface"));
}

#[test]
fn accepts_allow_unknown_flag() {
    let cli =
        Cli::try_parse_from(["atctl", "send", "AT+VENDOR=1", "--yes", "--allow-unknown"]).unwrap();

    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };

    assert!(args.yes);
    assert!(args.allow_unknown);
}

#[test]
fn parses_tui_theme_option() {
    let cli = Cli::try_parse_from(["atctl", "tui", "--theme", "light"]).unwrap();

    let Command::Tui(args) = cli.command else {
        panic!("expected tui command");
    };

    assert_eq!(args.theme, Some(TuiThemeChoice::Light));
}

#[test]
fn parses_tui_no_mask_option() {
    let cli = Cli::try_parse_from(["atctl", "tui", "--no-mask"]).unwrap();

    let Command::Tui(args) = cli.command else {
        panic!("expected tui command");
    };

    assert!(args.no_mask);
}

#[test]
fn user_command_timeout_defaults_to_thirty_seconds() {
    let cli = Cli::try_parse_from(["atctl", "send", "AT"]).unwrap();

    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };

    assert_eq!(args.usb.timeout, None);
    assert_eq!(
        args.usb.resolved_timeout_secs(None),
        DEFAULT_COMMAND_TIMEOUT_SECS
    );
}

#[test]
fn timeout_resolution_uses_explicit_then_item_then_product_default() {
    let omitted = UsbOptions::default();
    assert_eq!(
        omitted.resolved_timeout_secs(None),
        DEFAULT_COMMAND_TIMEOUT_SECS
    );
    assert_eq!(omitted.resolved_timeout_secs(Some(240)), 240);

    let explicit_default = UsbOptions {
        timeout: Some(DEFAULT_COMMAND_TIMEOUT_SECS),
        ..UsbOptions::default()
    };
    assert_eq!(
        explicit_default.resolved_timeout_secs(Some(240)),
        DEFAULT_COMMAND_TIMEOUT_SECS
    );

    assert_eq!(USB_WRITE_TIMEOUT_SECS, 3);
    assert_eq!(ENDPOINT_PROBE_TIMEOUT_SECS, 3);
}

#[test]
fn parses_preset_run_transport_and_confirmation_options() {
    let cli = Cli::try_parse_from([
        "atctl",
        "preset",
        "run",
        "set-soracom-apn-cid1",
        "--vid",
        "0x2c7c",
        "--pid",
        "0x0125",
        "--yes",
    ])
    .unwrap();

    let Command::Preset(PresetArgs {
        command: PresetCommand::Run(args),
    }) = cli.command
    else {
        panic!("expected preset run command");
    };

    assert_eq!(args.name, "set-soracom-apn-cid1");
    assert_eq!(args.usb.vid, Some(0x2c7c));
    assert_eq!(args.usb.pid, Some(0x0125));
    assert!(args.yes);
    assert!(!args.allow_unknown);
}

#[test]
fn rejects_removed_preset_batch_option() {
    let error = Cli::try_parse_from([
        "atctl",
        "preset",
        "run",
        "modem-info",
        "--continue-on-error",
    ])
    .unwrap_err();

    assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
}

#[test]
fn parses_preset_file_locations_for_list_run_and_tui() {
    let list_cli = Cli::try_parse_from([
        "atctl",
        "preset",
        "list",
        "--preset-file",
        "custom.toml",
        "--preset-dir",
        "presets.d",
    ])
    .unwrap();
    let Command::Preset(PresetArgs {
        command: PresetCommand::List(list_args),
    }) = list_cli.command
    else {
        panic!("expected preset list command");
    };
    assert_eq!(
        list_args.preset_locations.preset_files,
        vec![PathBuf::from("custom.toml")]
    );
    assert_eq!(
        list_args.preset_locations.preset_dirs,
        vec![PathBuf::from("presets.d")]
    );
    assert!(!list_args.json);

    let show_cli = Cli::try_parse_from([
        "atctl",
        "preset",
        "show",
        "custom-modem-response",
        "--json",
        "--preset-file",
        "custom.toml",
    ])
    .unwrap();
    let Command::Preset(PresetArgs {
        command: PresetCommand::Show(show_args),
    }) = show_cli.command
    else {
        panic!("expected preset show command");
    };
    assert_eq!(show_args.name, "custom-modem-response");
    assert!(show_args.json);
    assert_eq!(
        show_args.preset_locations.preset_files,
        vec![PathBuf::from("custom.toml")]
    );

    let run_cli = Cli::try_parse_from([
        "atctl",
        "preset",
        "run",
        "custom-modem-response",
        "--preset-file",
        "custom.toml",
    ])
    .unwrap();
    let Command::Preset(PresetArgs {
        command: PresetCommand::Run(run_args),
    }) = run_cli.command
    else {
        panic!("expected preset run command");
    };
    assert_eq!(run_args.name, "custom-modem-response");
    assert_eq!(
        run_args.preset_locations.preset_files,
        vec![PathBuf::from("custom.toml")]
    );

    let tui_cli = Cli::try_parse_from(["atctl", "tui", "--preset-dir", "presets.d"]).unwrap();
    let Command::Tui(tui_args) = tui_cli.command else {
        panic!("expected tui command");
    };
    assert_eq!(
        tui_args.preset_locations.preset_dirs,
        vec![PathBuf::from("presets.d")]
    );
}

#[test]
fn parses_sequence_run_transport_confirmation_params_and_locations() {
    let cli = Cli::try_parse_from([
        "atctl",
        "sequence",
        "run",
        "sms-send-check",
        "--vid",
        "0x2c7c",
        "--pid",
        "0x0125",
        "--timeout",
        "180",
        "--param",
        "recipient=+819012345678",
        "--param",
        "message=hello",
        "--yes",
        "--sequence-file",
        "examples/sequences/quectel.toml",
    ])
    .unwrap();

    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::Run(args),
    }) = cli.command
    else {
        panic!("expected sequence run command");
    };

    assert_eq!(args.name, "sms-send-check");
    assert_eq!(args.usb.vid, Some(0x2c7c));
    assert_eq!(args.usb.pid, Some(0x0125));
    assert_eq!(args.usb.timeout, Some(180));
    assert!(args.yes);
    assert!(!args.allow_unknown);
    assert_eq!(
        args.params,
        vec![
            SequenceParamValue {
                name: "recipient".to_owned(),
                value: "+819012345678".to_owned(),
            },
            SequenceParamValue {
                name: "message".to_owned(),
                value: "hello".to_owned(),
            },
        ]
    );
    assert_eq!(
        args.sequence_locations.sequence_files,
        vec![PathBuf::from("examples/sequences/quectel.toml")]
    );
}

#[test]
fn parses_sequence_locations_for_list_run_and_tui() {
    let list_cli = Cli::try_parse_from([
        "atctl",
        "sequence",
        "list",
        "--sequence-file",
        "custom-sequences.toml",
        "--sequence-dir",
        "sequences.d",
    ])
    .unwrap();
    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::List(list_args),
    }) = list_cli.command
    else {
        panic!("expected sequence list command");
    };
    assert_eq!(
        list_args.sequence_locations.sequence_files,
        vec![PathBuf::from("custom-sequences.toml")]
    );
    assert_eq!(
        list_args.sequence_locations.sequence_dirs,
        vec![PathBuf::from("sequences.d")]
    );
    assert!(!list_args.json);

    let show_cli = Cli::try_parse_from([
        "atctl",
        "sequence",
        "show",
        "custom-sequence",
        "--json",
        "--sequence-file",
        "custom-sequences.toml",
    ])
    .unwrap();
    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::Show(show_args),
    }) = show_cli.command
    else {
        panic!("expected sequence show command");
    };
    assert_eq!(show_args.name, "custom-sequence");
    assert!(show_args.json);
    assert_eq!(
        show_args.sequence_locations.sequence_files,
        vec![PathBuf::from("custom-sequences.toml")]
    );

    let run_cli = Cli::try_parse_from([
        "atctl",
        "sequence",
        "run",
        "custom-sequence",
        "--sequence-dir",
        "sequences.d",
    ])
    .unwrap();
    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::Run(run_args),
    }) = run_cli.command
    else {
        panic!("expected sequence run command");
    };
    assert_eq!(run_args.name, "custom-sequence");
    assert_eq!(
        run_args.sequence_locations.sequence_dirs,
        vec![PathBuf::from("sequences.d")]
    );

    let tui_cli = Cli::try_parse_from(["atctl", "tui", "--sequence-dir", "sequences.d"]).unwrap();
    let Command::Tui(tui_args) = tui_cli.command else {
        panic!("expected tui command");
    };
    assert_eq!(
        tui_args.sequence_locations.sequence_dirs,
        vec![PathBuf::from("sequences.d")]
    );
}

#[test]
fn formats_sequence_list_with_sms_product_sequences() {
    let sequences = sequence_builtins();
    let output = format_sequence_list(&sequences, false).unwrap();

    assert!(output.starts_with("NAME"));
    assert!(output.contains("PURPOSE"));
    assert!(!output.contains("ATTENTION"));
    assert!(output.contains("sms-send-check"));
    assert!(output.contains("Send a standard SMS"));
    assert!(output.contains("sms-receive-check"));
    assert!(output.contains("sms-read-message"));
    assert!(output.contains("sms-reply-check"));
    assert!(output.contains("[confirm]"));
    assert!(!output.contains("! Confirmation required"));
    assert!(!output.contains("Product Sequences"));
    assert!(!output.contains("not required"));

    let json = format_sequence_list(&sequences, true).unwrap();
    assert!(!json.contains("[confirm]"));
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value[0]["operation"]["kind"], "sequence");
    assert!(value.as_array().unwrap().iter().any(|row| {
        row["operation"]["name"] == "sms-read-message"
            && row["parameters"][0]["candidate"] == "sms-message"
            && row["parameters"][0]["candidate_providers"][0]["kind"] == "sequence"
            && row["parameters"][0]["candidate_providers"][0]["name"] == "sms-receive-check"
    }));
}

#[derive(Default)]
struct TestSequenceValuePrompt {
    provider_choices: Vec<CandidateProviderChoice>,
    candidate_choices: Vec<CandidateValueChoice>,
    manual_values: Vec<String>,
    failures: Vec<String>,
    empty_results: Vec<String>,
}

impl SequenceValuePrompt for TestSequenceValuePrompt {
    fn choose_provider(
        &mut self,
        _sequence: &Sequence,
        _param: &SequenceParam,
        _candidate: SequenceCandidateSource,
        _providers: &[CandidateProviderOption],
    ) -> Result<CandidateProviderChoice> {
        Ok(self.provider_choices.remove(0))
    }

    fn enter_manual_value(&mut self, _param: &SequenceParam) -> Result<String> {
        Ok(self.manual_values.remove(0))
    }

    fn choose_candidate(
        &mut self,
        _provider: SequenceCandidateProvider,
        _candidates: &[SequenceValueCandidate],
        _masked: bool,
    ) -> Result<CandidateValueChoice> {
        Ok(self.candidate_choices.remove(0))
    }

    fn provider_failed(&mut self, provider: SequenceCandidateProvider, error: &AtctlError) {
        self.failures.push(format!("{}: {error}", provider.name));
    }

    fn provider_returned_no_candidates(&mut self, provider: SequenceCandidateProvider) {
        self.empty_results.push(provider.name.to_owned());
    }
}

#[test]
fn interactive_sequence_resolution_runs_existing_sms_sequence_provider() {
    let sequences = sequence_builtins();
    let sequence = sequences
        .iter()
        .find(|sequence| sequence.name == "sms-read-message")
        .unwrap();
    let provider = SequenceCandidateSource::SmsMessage.providers()[0];
    let mut prompt = TestSequenceValuePrompt {
        provider_choices: vec![CandidateProviderChoice::Provider(provider)],
        candidate_choices: vec![CandidateValueChoice::Candidate("8".to_owned())],
        ..TestSequenceValuePrompt::default()
    };
    let presets = builtins();
    let mut executed = Vec::new();

    let params = resolve_sequence_params_interactively(
        sequence,
        Vec::new(),
        true,
        &mut prompt,
        |candidate| candidate_provider_options(candidate, &presets, &sequences),
        |candidate, provider| {
            executed.push(provider);
            Ok(SequenceValueCandidateSet {
                candidate,
                candidates: vec![SequenceValueCandidate {
                    value: "8".to_owned(),
                    raw_label: "storage=8  REC UNREAD  901001".to_owned(),
                    masked_label: "storage=8  REC UNREAD  ******".to_owned(),
                }],
            })
        },
    )
    .unwrap();

    assert_eq!(executed, vec![provider]);
    assert_eq!(provider.kind, SequenceCandidateProviderKind::Sequence);
    assert_eq!(provider.name, "sms-receive-check");
    assert_eq!(
        params,
        vec![SequenceParamValue {
            name: "index".to_owned(),
            value: "8".to_owned(),
        }]
    );
}

#[test]
fn interactive_sequence_resolution_preserves_manual_entry_without_provider_execution() {
    let sequences = sequence_builtins();
    let sequence = sequences
        .iter()
        .find(|sequence| sequence.name == "sms-read-message")
        .unwrap();
    let presets = builtins();
    let mut prompt = TestSequenceValuePrompt {
        provider_choices: vec![CandidateProviderChoice::Manual],
        manual_values: vec!["12".to_owned()],
        ..TestSequenceValuePrompt::default()
    };
    let mut provider_ran = false;

    let params = resolve_sequence_params_interactively(
        sequence,
        Vec::new(),
        true,
        &mut prompt,
        |candidate| candidate_provider_options(candidate, &presets, &sequences),
        |candidate, _provider| {
            provider_ran = true;
            Ok(SequenceValueCandidateSet {
                candidate,
                candidates: Vec::new(),
            })
        },
    )
    .unwrap();

    assert!(!provider_ran);
    assert_eq!(params[0].name, "index");
    assert_eq!(params[0].value, "12");
}

#[test]
fn sequence_json_output_includes_structured_steps_and_notes() {
    let review = review_direct_command("AT+CMGS=1");
    let execution = SequenceExecution {
        name: "sms-send-check".to_owned(),
        masked_review: review.masked(),
        review,
        status: AtStatus::Ok,
        steps: vec![SequenceStepResult {
            id: "write-message".to_owned(),
            label: Some("Write message body".to_owned()),
            status: AtStatus::Ok,
            masked_analysis: Some("masked analysis".to_owned()),
            raw_analysis: Some("raw payload analysis".to_owned()),
        }],
        masked_notes: vec!["masked note".to_owned()],
        raw_notes: vec!["raw payload note".to_owned()],
        value_candidate_sets: Vec::new(),
        masked_transcript: "masked transcript".to_owned(),
        raw_transcript: "raw payload transcript".to_owned(),
        duration: Duration::from_millis(1),
    };

    let sequence = sequence_builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-send-check")
        .unwrap();
    let output = format_sequence_output(&sequence, &execution, true, true).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(json["surface"], "cli");
    assert_eq!(json["operation"]["kind"], "sequence");
    assert_eq!(json["operation"]["name"], "sms-send-check");
    assert_eq!(json["operation"]["purpose"], sequence.summary);
    assert_eq!(json["execution_review"]["assessment"], "known");
    assert_eq!(json["execution_review"]["confirmation_required"], true);
    assert!(json.get("risk").is_none());
    assert_eq!(json["masked"], true);
    assert_eq!(json["transcript"], "masked transcript");
    assert_eq!(json["steps"][0]["id"], "write-message");
    assert_eq!(json["steps"][0]["label"], "Write message body");
    assert_eq!(json["steps"][0]["analysis"], "masked analysis");
    assert!(json["steps"][0].get("evidence").is_none());
    assert_eq!(json["notes"][0], "masked note");
    assert!(!output.contains("raw payload"));
}

#[test]
fn noninteractive_sequence_review_does_not_repeat_resolved_values() {
    let sequence = sequence_builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-send-check")
        .unwrap();
    let params = vec![
        SequenceParamValue {
            name: "recipient".to_owned(),
            value: "+819012345678".to_owned(),
        },
        SequenceParamValue {
            name: "message".to_owned(),
            value: "private message body".to_owned(),
        },
    ];
    let review = review_sequence_execution(&sequence, &params).unwrap();
    let mut lines = operation_identity_output_lines(&OperationIdentity::sequence(&sequence));
    lines.extend(execution_review_output_lines(&review));
    let notice = lines.join("\n");

    assert!(notice.contains("Name: sms-send-check"));
    assert!(notice.contains("Purpose:"));
    assert!(!notice.contains("+819012345678"));
    assert!(!notice.contains("private message body"));
    assert!(!notice.contains("Payload:"));
}

#[test]
fn sequence_text_export_includes_identity_and_selected_transcript() {
    let sequence = sequence_builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-send-check")
        .unwrap();
    let review = review_direct_command("AT+CMGS=1");
    let execution = SequenceExecution {
        name: "sms-send-check".to_owned(),
        masked_review: review.masked(),
        review,
        status: AtStatus::Ok,
        steps: Vec::new(),
        masked_notes: Vec::new(),
        raw_notes: Vec::new(),
        value_candidate_sets: Vec::new(),
        masked_transcript: "masked transcript".to_owned(),
        raw_transcript: "raw transcript".to_owned(),
        duration: Duration::from_millis(1),
    };

    assert_eq!(
        format_sequence_export(&sequence, &execution, true, false).unwrap(),
        format!(
            "Sequence: sms-send-check\nPurpose: {}\n\nmasked transcript\n",
            sequence.summary
        )
    );
    assert_eq!(
        format_sequence_export(&sequence, &execution, false, false).unwrap(),
        format!(
            "Sequence: sms-send-check\nPurpose: {}\n\nraw transcript\n",
            sequence.summary
        )
    );
}

#[test]
fn default_sequence_locations_load_only_product_sequences() {
    let sequences = load_sequences(&SequenceFileLocationOptions::default()).unwrap();
    let names = sequences
        .iter()
        .map(|sequence| sequence.name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"sms-send-check"));
    assert!(names.contains(&"sms-receive-check"));
    assert!(!names.contains(&"file-sequence"));
    assert!(!names.contains(&"dir-sequence"));
}

#[test]
fn explicit_sequence_locations_load_add_on_sequences() {
    let dir = unique_temp_dir("explicit-sequence-locations");
    let file = dir.join("custom-sequences.toml");
    let drop_in_dir = dir.join("sequences.d");
    std::fs::create_dir_all(&drop_in_dir).unwrap();
    std::fs::write(
        &file,
        r#"
            title = "File Sequences"

            [[sequences]]
            name = "file-sequence"
            summary = "File sequence"
            categories = ["diagnostics"]

            [[sequences.steps]]
            id = "at"
            send = "AT"
            expect = "OK"
            "#,
    )
    .unwrap();
    std::fs::write(
        drop_in_dir.join("10-dir.toml"),
        r#"
            title = "Directory Sequences"

            [[sequences]]
            name = "dir-sequence"
            summary = "Directory sequence"
            categories = ["diagnostics"]

            [[sequences.steps]]
            id = "at"
            send = "AT"
            expect = "OK"
            "#,
    )
    .unwrap();
    let locations = SequenceFileLocationOptions {
        sequence_files: vec![file],
        sequence_dirs: vec![drop_in_dir],
    };

    let sequences = load_sequences(&locations).unwrap();
    let names = sequences
        .iter()
        .map(|sequence| sequence.name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"sms-send-check"));
    assert!(names.contains(&"sms-receive-check"));
    assert!(names.contains(&"file-sequence"));
    assert!(names.contains(&"dir-sequence"));

    let output = format_sequence_list(&sequences, false).unwrap();
    assert!(output.contains(locations.sequence_files[0].to_string_lossy().as_ref()));
    assert!(
        output.contains(
            locations.sequence_dirs[0]
                .join("10-dir.toml")
                .to_string_lossy()
                .as_ref()
        )
    );
}

#[test]
fn default_preset_locations_load_only_product_presets() {
    let presets = load_presets(&PresetFileLocationOptions::default()).unwrap();
    let names = presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"modem-response"));
    assert!(!names.contains(&"file-modem-response"));
    assert!(!names.contains(&"dir-signal"));
}

#[test]
fn explicit_preset_locations_load_add_on_presets() {
    let dir = unique_temp_dir("explicit-preset-locations");
    let file = dir.join("custom.toml");
    let drop_in_dir = dir.join("presets.d");
    std::fs::create_dir_all(&drop_in_dir).unwrap();
    std::fs::write(
        &file,
        r#"
            title = "File commands"

            [[presets]]
            name = "file-modem-response"
            command = "AT"
            categories = ["file"]
            "#,
    )
    .unwrap();
    std::fs::write(
        drop_in_dir.join("10-dir.toml"),
        r#"
            title = "Directory commands"

            [[presets]]
            name = "dir-signal"
            command = "AT+CSQ"
            categories = ["dir"]
            "#,
    )
    .unwrap();
    let locations = PresetFileLocationOptions {
        preset_files: vec![file],
        preset_dirs: vec![drop_in_dir],
    };

    let presets = load_presets(&locations).unwrap();
    let names = presets
        .iter()
        .map(|preset| preset.name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"modem-response"));
    assert!(names.contains(&"file-modem-response"));
    assert!(names.contains(&"dir-signal"));

    let output = format_preset_list(&presets, false).unwrap();
    assert!(output.contains(locations.preset_files[0].to_string_lossy().as_ref()));
    assert!(
        output.contains(
            locations.preset_dirs[0]
                .join("10-dir.toml")
                .to_string_lossy()
                .as_ref()
        )
    );
}

#[test]
fn explicit_missing_preset_location_is_an_error() {
    let locations = PresetFileLocationOptions {
        preset_files: vec![PathBuf::from("/definitely/missing/atctl-presets.toml")],
        preset_dirs: Vec::new(),
    };

    assert!(matches!(
        load_presets(&locations),
        Err(AtctlError::ReadFile { .. })
    ));
}

#[test]
fn formats_preset_list_with_review_categories_and_command() {
    let presets = vec![Preset::new(
        "custom-modem-response",
        "AT",
        vec!["custom".to_owned()],
        PresetOrigin::file("Custom commands", "presets.toml", None),
    )];

    let output = format_preset_list(&presets, false).unwrap();

    assert!(output.starts_with("NAME"));
    assert!(output.contains("AT COMMAND"));
    assert!(!output.contains("ATTENTION"));
    assert!(output.contains("Loaded from: Custom commands"));
    assert!(output.contains("File: presets.toml"));
    assert!(output.contains("custom-modem-response"));
    assert!(!output.contains("[confirm]"));
    assert!(!output.contains("not required"));
}

#[test]
fn human_preset_list_uses_one_confirmation_tag_for_known_and_unknown_rows() {
    let presets = vec![
        Preset::built_in("known-change", "AT+CMEE=2", ["review"]),
        Preset::built_in("unknown-change", "AT+VENDOR=1", ["review"]),
        Preset::built_in("observation", "AT", ["review"]),
    ];

    let output = format_preset_list(&presets, false).unwrap();
    let lines = output.lines().collect::<Vec<_>>();

    for name in ["known-change", "unknown-change"] {
        let row = lines
            .iter()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("missing {name} row in {output}"));
        assert!(row.ends_with("[confirm]"), "{row}");
    }
    let known_tag_column = lines
        .iter()
        .find(|line| line.contains("known-change"))
        .and_then(|line| line.find("[confirm]"))
        .expect("known confirmation tag");
    let unknown_tag_column = lines
        .iter()
        .find(|line| line.contains("unknown-change"))
        .and_then(|line| line.find("[confirm]"))
        .expect("unknown confirmation tag");
    assert_eq!(known_tag_column, unknown_tag_column);
    let observation = lines
        .iter()
        .find(|line| line.contains("observation"))
        .unwrap_or_else(|| panic!("missing observation row in {output}"));
    assert!(!observation.contains("[confirm]"));
    assert!(!output.contains("ATTENTION"));
    assert!(!output.contains("Confirmation required"));

    let json = format_preset_list(&presets, true).unwrap();
    assert!(!json.contains("[confirm]"));
}

#[test]
fn human_discovery_lists_reclaim_confirmation_lane_when_unused() {
    let long_preview = "X".repeat(200);
    let mixed = format_discovery_table(
        [
            DiscoveryRow {
                name: "plain-record",
                preview: &long_preview,
                confirmation_tag: None,
                source_title: None,
                source_path: None,
            },
            DiscoveryRow {
                name: "tagged-row-x",
                preview: "AT+CMEE=2",
                confirmation_tag: Some(crate::at::review::CONFIRMATION_FLOW_TAG),
                source_title: None,
                source_path: None,
            },
        ]
        .into_iter(),
        "AT COMMAND",
    );
    let confirmation_free = format_discovery_table(
        [DiscoveryRow {
            name: "plain-record",
            preview: &long_preview,
            confirmation_tag: None,
            source_title: None,
            source_path: None,
        }]
        .into_iter(),
        "AT COMMAND",
    );

    let mixed_plain = mixed
        .lines()
        .find(|line| line.contains("plain-record"))
        .expect("mixed plain row");
    let mixed_tagged = mixed
        .lines()
        .find(|line| line.contains("tagged-row-x"))
        .expect("mixed tagged row");
    let reclaimed_plain = confirmation_free
        .lines()
        .find(|line| line.contains("plain-record"))
        .expect("confirmation-free row");
    let tag_column = mixed_tagged.find("[confirm]").expect("confirmation tag");
    assert!(display_width(mixed_plain) <= tag_column);
    assert!(display_width(reclaimed_plain) > display_width(mixed_plain));
}

#[test]
fn formats_product_preset_list_label() {
    let presets = vec![Preset::new(
        "modem-response",
        "AT",
        vec!["basic".to_owned()],
        PresetOrigin::BuiltIn,
    )];

    let output = format_preset_list(&presets, false).unwrap();

    assert!(output.contains("modem-response"));
    assert!(output.contains("AT"));
    assert!(!output.contains("Product presets"));
    assert!(!output.contains("Built-in presets"));
    assert!(!output.contains("not required"));
}

#[test]
fn show_outputs_are_complete_structured_and_read_only_formatters() {
    let preset = Preset::new_with_timeout(
        "external-command",
        "AT+CMEE=2",
        vec!["diagnostics".to_owned()],
        PresetOrigin::file("External commands", "/tmp/external.toml", None),
        Some(45),
    );
    let preset_text = format_preset_show(&preset, false).unwrap();
    assert!(preset_text.contains("Name: external-command"));
    assert!(preset_text.contains("AT command: AT+CMEE=2"));
    assert!(preset_text.contains("Definition origin: file"));
    assert!(preset_text.contains("Definition file: /tmp/external.toml"));
    assert!(preset_text.contains("Categories: diagnostics"));
    assert!(preset_text.contains("Timeout: 45 seconds"));
    assert!(!preset_text.contains("(default)"));
    assert!(preset_text.starts_with("Overview:\nType: Preset"));
    assert!(preset_text.contains("\n\nDefinition:\n"));
    assert!(preset_text.contains("\n\nExecution:\n"));
    assert!(!preset_text.contains("Before running:"));
    assert!(!preset_text.contains("Inputs:"));
    assert!(
        preset_text.find("Overview:") < preset_text.find("Definition:")
            && preset_text.find("Definition:") < preset_text.find("Execution:")
    );

    let default_preset = Preset::new("default-timeout", "AT", Vec::new(), PresetOrigin::BuiltIn);
    let default_preset_text = format_preset_show(&default_preset, false).unwrap();
    assert!(default_preset_text.contains("Timeout: 30 seconds"));
    assert!(!default_preset_text.contains("(default)"));

    let sequence = sequence_builtins()
        .into_iter()
        .find(|sequence| sequence.name == "sms-read-message")
        .unwrap();
    let sequence_json = format_sequence_show(&sequence, true).unwrap();
    let value: serde_json::Value = serde_json::from_str(&sequence_json).unwrap();
    assert_eq!(value["operation"]["kind"], "sequence");
    assert_eq!(value["operation"]["name"], "sms-read-message");
    assert_eq!(value["parameters"][0]["source"], "select");
    assert_eq!(value["parameters"][0]["candidate"], "sms-message");
    assert_eq!(
        value["parameters"][0]["candidate_providers"][0]["kind"],
        "sequence"
    );
    assert_eq!(
        value["parameters"][0]["candidate_providers"][0]["name"],
        "sms-receive-check"
    );
    assert!(value.get("execution_review").is_some());

    let mut default_sequence = sequence.clone();
    default_sequence.timeout_secs = None;
    default_sequence.before_running =
        vec!["Inspect the target SMS storage before execution.".to_owned()];
    let default_sequence_text = format_sequence_show(&default_sequence, false).unwrap();
    assert!(default_sequence_text.contains("Timeout: 30 seconds"));
    assert!(default_sequence_text.contains("Provider: Sequence sms-receive-check"));
    assert!(!default_sequence_text.contains("(default)"));
    assert!(default_sequence_text.starts_with("Overview:\nType: Sequence"));
    for heading in [
        "Overview:",
        "Definition:",
        "Before running:",
        "Inputs:",
        "Execution:",
    ] {
        assert!(default_sequence_text.contains(heading));
    }
    assert!(
        default_sequence_text.find("Overview:") < default_sequence_text.find("Definition:")
            && default_sequence_text.find("Definition:")
                < default_sequence_text.find("Before running:")
            && default_sequence_text.find("Before running:")
                < default_sequence_text.find("Inputs:")
            && default_sequence_text.find("Inputs:") < default_sequence_text.find("Execution:")
    );
}

#[test]
fn preset_lookup_uses_exact_name_instead_of_category() {
    let presets = vec![Preset::new(
        "current-operator",
        "AT+COPS?",
        vec!["network".to_owned()],
        PresetOrigin::BuiltIn,
    )];

    assert_eq!(
        find_preset(&presets, "current-operator").unwrap().name,
        "current-operator"
    );
    let error = find_preset(&presets, "network").unwrap_err();
    assert!(matches!(
        &error,
        AtctlError::PresetNotFound { name } if name == "network"
    ));
    assert!(error.to_string().contains(
        "help: run `atctl preset list` with the same `--preset-file` and `--preset-dir` options"
    ));
}

#[test]
fn sequence_lookup_error_points_to_the_exact_loaded_name_list() {
    let sequences = crate::sequences::builtin::builtins();

    let error = find_sequence(&sequences, "missing-sequence").unwrap_err();

    assert!(matches!(
        &error,
        AtctlError::SequenceNotFound { name } if name == "missing-sequence"
    ));
    assert!(error.to_string().contains(
        "help: run `atctl sequence list` with the same `--sequence-file` and `--sequence-dir` options"
    ));
}

#[test]
fn preset_execution_uses_product_derived_review() {
    let preset = Preset::new(
        "verbose-errors",
        "AT+CMEE=2",
        Vec::new(),
        PresetOrigin::file("Custom commands", "presets.toml", None),
    );
    let run_args = preset_run_args("verbose-errors");
    let send_args = send_args_from_preset(&preset, &run_args);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: true,
        calls: 0,
        notices: 0,
    };

    let execution = execute_command_with_confirmation(
        &send_args,
        transport,
        &mut prompt,
        preset.review.clone(),
        ExecutionSurface::Cli,
        OperationIdentity::preset(&preset),
        external_preset_source(&preset),
    )
    .unwrap();

    assert_eq!(execution.review.assessment, Assessment::Known);
    assert!(execution.review.confirmation_required);
    assert_eq!(prompt.calls, 1);
    assert_eq!(prompt.notices, 0);
}

#[test]
fn file_preset_yes_still_reports_external_notice_before_transport() {
    let preset = Preset::new(
        "verbose-errors",
        "AT+CMEE=2",
        Vec::new(),
        PresetOrigin::file("Custom commands", "presets.toml", None),
    );
    let mut run_args = preset_run_args("verbose-errors");
    run_args.yes = true;
    let send_args = send_args_from_preset(&preset, &run_args);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    let execution = execute_command_with_confirmation(
        &send_args,
        transport,
        &mut prompt,
        preset.review.clone(),
        ExecutionSurface::Cli,
        OperationIdentity::preset(&preset),
        external_preset_source(&preset),
    )
    .unwrap();

    assert!(execution.review.confirmation_required);
    assert_eq!(prompt.calls, 0);
    assert_eq!(prompt.notices, 1);
}

#[test]
fn authorized_notice_helper_reports_only_for_noninteractive_approval() {
    let source = Some(ExternalDefinitionSource {
        label: "Custom commands",
        path: "presets.toml",
    });
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    let operation = OperationIdentity::command("AT+CMEE=2");
    let review = review_direct_command("AT+CMEE=2");
    notice_authorized_execution_without_interactive_confirmation(
        &mut prompt,
        ExecutionAuthorization::InteractiveRequired,
        &operation,
        &review,
        source,
    )
    .unwrap();
    assert_eq!(prompt.notices, 0);

    notice_authorized_execution_without_interactive_confirmation(
        &mut prompt,
        ExecutionAuthorization::NonInteractiveApproved,
        &operation,
        &review,
        source,
    )
    .unwrap();
    assert_eq!(prompt.notices, 1);
}

#[test]
fn preset_timeout_hint_replaces_default_timeout() {
    let preset = Preset::new_with_timeout(
        "long-scan",
        "AT+COPS=?",
        Vec::new(),
        PresetOrigin::BuiltIn,
        Some(180),
    );
    let mut run_args = preset_run_args("long-scan");
    run_args.usb.timeout = None;

    let send_args = send_args_from_preset(&preset, &run_args);

    assert_eq!(send_args.usb.timeout, Some(180));
}

#[test]
fn explicit_default_preset_timeout_overrides_timeout_hint() {
    let preset = Preset::new_with_timeout(
        "long-scan",
        "AT+COPS=?",
        Vec::new(),
        PresetOrigin::BuiltIn,
        Some(180),
    );
    let mut run_args = preset_run_args("long-scan");
    run_args.usb.timeout = Some(DEFAULT_COMMAND_TIMEOUT_SECS);

    let send_args = send_args_from_preset(&preset, &run_args);

    assert_eq!(send_args.usb.timeout, Some(DEFAULT_COMMAND_TIMEOUT_SECS));
}

#[test]
fn explicit_preset_timeout_overrides_timeout_hint() {
    let preset = Preset::new_with_timeout(
        "long-scan",
        "AT+COPS=?",
        Vec::new(),
        PresetOrigin::BuiltIn,
        Some(180),
    );
    let mut run_args = preset_run_args("long-scan");
    run_args.usb.timeout = Some(240);

    let send_args = send_args_from_preset(&preset, &run_args);

    assert_eq!(send_args.usb.timeout, Some(240));
}

#[test]
fn preset_no_log_option_is_forwarded_to_shared_send_execution() {
    let preset = Preset::new("modem-info", "ATI", Vec::new(), PresetOrigin::BuiltIn);
    let mut run_args = preset_run_args("modem-info");
    run_args.no_log = true;

    let send_args = send_args_from_preset(&preset, &run_args);

    assert!(send_args.no_log);
}

#[test]
fn send_at_uses_transport_and_returns_masked_text_output() {
    let args = send_args("AT");
    let transport = MockTransport::with_response(b"AT\r\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert_eq!(execution.review.assessment, Assessment::Known);
    assert!(!execution.review.confirmation_required);
    assert_eq!(execution.text, "AT\r\r\nOK\r\n");
    assert!(send_status_result(&execution, false).is_ok());
}

#[test]
fn send_accepts_successful_mock_response_without_hardware() {
    let args = send_args("AT");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
}

#[test]
fn send_masks_sensitive_response_by_default() {
    let args = send_args("AT+CIMI");
    let transport = MockTransport::with_response(b"\r\n898110001234567\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert!(execution.text.contains("89811000*******"));
    assert!(!execution.text.contains("898110001234567"));
}

#[test]
fn send_pairs_short_phone_masking_with_visible_operational_fields() {
    let mut args = send_args("AT+CNUM");
    args.yes = true;
    args.allow_unknown = true;
    let transport = MockTransport::with_response(
        b"\r\n+CNUM: \"line\",\"901001\",129\r\n+CMGS: 7\r\nOK\r\n".to_vec(),
    );

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert!(execution.raw_text.contains("\"901001\",129"));
    assert!(execution.masked_text.contains("\"******\",129"));
    assert!(execution.masked_text.contains("+CMGS: 7"));
    assert_eq!(execution.text, execution.masked_text);
    assert!(
        !format_send_output(&execution, true)
            .unwrap()
            .contains("901001")
    );
}

#[test]
fn send_no_mask_preserves_sensitive_response_for_display() {
    let mut args = send_args("AT+CIMI");
    args.no_mask = true;
    let transport = MockTransport::with_response(b"\r\n898110001234567\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert!(execution.text.contains("898110001234567"));
    assert!(execution.masked_text.contains("89811000*******"));
    assert!(!execution.masked);
}

#[test]
fn send_json_uses_masked_response_by_default() {
    let args = send_args("AT+CIMI");
    let transport = MockTransport::with_response(b"\r\n898110001234567\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();
    let output = format_send_output(&execution, true).unwrap();

    assert!(output.contains("89811000*******"));
    assert!(!output.contains("898110001234567"));
    assert!(output.contains("\"masked\":true"));
}

#[test]
fn send_export_includes_command_and_selected_masking_state() {
    let args = send_args("AT+CIMI");
    let transport = MockTransport::with_response(b"\r\n898110001234567\r\nOK\r\n".to_vec());
    let execution = execute_send_with_transport(&args, transport).unwrap();

    let text = format_send_export(&execution, false).unwrap();
    assert!(text.starts_with("AT+CIMI\n"));
    assert!(text.contains("89811000*******"));
    assert!(!text.contains("898110001234567"));

    let json = format_send_export(&execution, true).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["surface"], "cli");
    assert_eq!(value["operation"]["kind"], "command");
    assert_eq!(value["operation"]["command"], "AT+CIMI");
    assert_eq!(value["masked"], true);
    assert!(
        value["response"]
            .as_str()
            .unwrap()
            .contains("89811000*******")
    );
}

#[test]
fn send_export_masks_sensitive_command_values_by_default() {
    let command = "AT+CGAUTH=1,1,\"user\",\"password\"";
    let mut args = send_args(command);
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let execution = execute_send_with_transport(&args, transport).unwrap();

    let text = format_send_export(&execution, false).unwrap();
    let json = format_send_export(&execution, true).unwrap();

    assert!(!text.contains("user"));
    assert!(!text.contains("password"));
    assert!(!json.to_ascii_lowercase().contains("user"));
    assert!(!json.to_ascii_lowercase().contains("password"));
    assert!(!json.contains("\"risk\""));
    assert!(json.contains("\"execution_review\""));
    assert!(json.contains("\"assessment\":\"known\""));
    assert!(json.contains("\"effects\""));
    assert!(json.contains("\"data_handling\""));
    assert!(json.contains("\"reversibility\""));
    assert!(json.contains("\"consequences\""));
    assert!(json.contains("\"confirmation_required\""));
    assert!(json.contains("\"reasons\""));
    assert!(text.contains("AT+CGAUTH=1,1"));

    let mut unmasked_args = send_args(command);
    unmasked_args.no_mask = true;
    unmasked_args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let unmasked_execution = execute_send_with_transport(&unmasked_args, transport).unwrap();
    let unmasked = format_send_export(&unmasked_execution, false).unwrap();
    assert!(unmasked.contains("user"));
    assert!(unmasked.contains("password"));
}

#[test]
fn send_at_error_is_failure_unless_ignored() {
    let args = send_args("AT+CSQ");
    let transport = MockTransport::with_response(b"\r\nERROR\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert_eq!(execution.status, AtStatus::Error);
    assert!(matches!(
        send_status_result(&execution, false),
        Err(AtctlError::AtCommandFailed {
            status: AtStatus::Error
        })
    ));
    assert!(send_status_result(&execution, true).is_ok());
}

#[test]
fn send_raw_log_file_writes_raw_exchange_without_changing_masked_display() {
    let dir = unique_temp_dir("raw-log-send");
    let path = dir.join("case.rawlog");
    let mut args = send_args("AT+CIMI");
    args.raw_log_file = Some(path.clone());
    args.raw_log_ack = Some(RAW_LOG_ACK.to_owned());
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\n898110001234567\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert!(execution.text.contains("89811000*******"));
    assert!(!execution.text.contains("898110001234567"));
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains("\"event\":\"header\""));
    assert!(contents.contains("\"event\":\"exchange\""));
    assert!(contents.contains("\"tx_base64\":\"QVQrQ0lNSQ0=\""));
    assert!(contents.contains("898110001234567"));
}

#[test]
fn raw_log_preflight_accepts_canonical_ack_for_cli_sequence_and_pty() {
    let dir = unique_temp_dir("raw-log-preflight-surfaces");

    let mut send = send_args("AT");
    send.raw_log_file = Some(dir.join("send.rawlog"));
    send.raw_log_ack = Some(RAW_LOG_ACK.to_owned());
    let mut prompt = NonInteractiveConfirmationPrompt;
    assert!(
        prepare_raw_log_config(&send, &mut prompt, ExecutionSurface::Cli)
            .unwrap()
            .is_some()
    );

    let sequence_path = dir.join("sequence.rawlog");
    let sequence_path = sequence_path.to_string_lossy().into_owned();
    let cli = Cli::try_parse_from([
        "atctl",
        "sequence",
        "run",
        "sms-receive-check",
        "--raw-log-file",
        sequence_path.as_str(),
        "--raw-log-ack",
        RAW_LOG_ACK,
    ])
    .unwrap();
    let Command::Sequence(SequenceArgs {
        command: SequenceCommand::Run(sequence),
    }) = cli.command
    else {
        panic!("expected sequence run command");
    };
    assert!(
        prepare_sequence_raw_log_config(&sequence)
            .unwrap()
            .is_some()
    );

    let bridge_path = dir.join("bridge.rawlog");
    let bridge_path = bridge_path.to_string_lossy().into_owned();
    let symlink_path = dir.join("bridge-pty");
    let symlink_path = symlink_path.to_string_lossy().into_owned();
    let cli = Cli::try_parse_from([
        "atctl",
        "bridge",
        "--symlink",
        symlink_path.as_str(),
        "--raw-log-file",
        bridge_path.as_str(),
        "--raw-log-ack",
        RAW_LOG_ACK,
    ])
    .unwrap();
    let Command::Bridge(bridge) = cli.command else {
        panic!("expected bridge command");
    };
    assert!(bridge_raw_log_config(&bridge).unwrap().is_some());
}

#[test]
fn send_raw_log_file_requires_acknowledgement() {
    let dir = unique_temp_dir("raw-log-ack");
    let mut args = send_args("AT");
    args.raw_log_file = Some(dir.join("case.rawlog"));
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::RawLogAckRequired));
}

#[test]
fn send_raw_log_file_refuses_overwrite() {
    let dir = unique_temp_dir("raw-log-overwrite");
    let path = dir.join("case.rawlog");
    std::fs::write(&path, "existing").unwrap();
    let mut args = send_args("AT");
    args.raw_log_file = Some(path);
    args.raw_log_ack = Some(RAW_LOG_ACK.to_owned());
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::RawLogFileExists { .. }));
}

#[test]
fn send_raw_log_file_is_not_created_when_transport_open_fails() {
    let dir = unique_temp_dir("raw-log-open-error");
    let path = dir.join("case.rawlog");
    let mut args = send_args("AT+COPS?");
    args.raw_log_file = Some(path.clone());
    args.raw_log_ack = Some(RAW_LOG_ACK.to_owned());
    args.yes = true;

    let error = execute_send_with_transport(&args, FailingOpenTransport).unwrap_err();

    assert!(error.to_string().contains("target selection failed"));
    assert!(!path.exists());
}

#[test]
fn send_raw_log_file_records_transport_error_when_response_is_missing() {
    let dir = unique_temp_dir("raw-log-timeout");
    let path = dir.join("case.rawlog");
    let mut args = send_args("AT+COPS?");
    args.raw_log_file = Some(path.clone());
    args.raw_log_ack = Some(RAW_LOG_ACK.to_owned());
    args.yes = true;
    let transport = MockTransport::default();

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::Timeout { timeout_secs: 3 }));
    assert_eq!(error.to_string(), "AT operation timed out after 3 seconds.");
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains("\"event\":\"header\""));
    assert!(contents.contains("\"event\":\"transport_error\""));
    assert!(contents.contains("\"stage\":\"read_response\""));
    assert!(contents.contains("\"command\":\"AT+COPS?\""));
    assert!(contents.contains("\"tx_base64\":\"QVQrQ09QUz8N\""));
}

#[test]
fn known_material_command_requires_confirmation_without_yes() {
    let args = send_args("ATE0");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::ConfirmationRequired));
}

#[test]
fn one_shot_send_rejects_prompt_required_data_entry_before_transport_access() {
    let mut args = send_args("AT+QISEND=0,4");
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\n> ".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::PromptRequiredOneShot));
}

#[test]
fn known_material_command_runs_after_interactive_confirmation() {
    let args = send_args("ATE0");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: true,
        calls: 0,
        notices: 0,
    };

    let execution = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert_eq!(execution.review.assessment, Assessment::Known);
    assert!(execution.review.confirmation_required);
    assert_eq!(prompt.calls, 1);
}

#[test]
fn known_material_command_stops_when_interactive_confirmation_is_rejected() {
    let args = send_args("ATE0");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    let error = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap_err();

    assert_eq!(prompt.calls, 1);
    assert!(matches!(error, AtctlError::ConfirmationRequired));
}

#[test]
fn send_yes_bypasses_known_interactive_confirmation() {
    let mut args = send_args("AT+CFUN=0");
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    let execution = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert_eq!(execution.review.assessment, Assessment::Known);
    assert_eq!(prompt.calls, 0);
}

#[test]
fn allow_unknown_without_yes_is_rejected_before_transport_access() {
    let mut args = send_args("AT+VENDOR=1");
    args.allow_unknown = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::AllowUnknownRequiresYes));
}

#[test]
fn allow_unknown_is_rejected_for_known_operation() {
    let mut args = send_args("AT");
    args.yes = true;
    args.allow_unknown = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(error, AtctlError::AllowUnknownNotApplicable));
}

#[test]
fn unknown_noninteractive_send_requires_allow_unknown_before_raw_log_option() {
    let mut args = send_args("AT+VENDOR=1");
    args.raw_log_file = Some(unique_temp_dir("raw-log-review").join("case.rawlog"));
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(
        error,
        AtctlError::UnknownOperationRequiresAllowance
    ));
}

#[test]
fn unknown_noninteractive_send_runs_with_yes_and_allow_unknown() {
    let mut args = send_args("AT+VENDOR=1");
    args.yes = true;
    args.allow_unknown = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert_eq!(execution.review.assessment, Assessment::Unknown);
    assert!(execution.review.confirmation_required);
}

#[test]
fn send_rejects_zero_timeout_before_transport_access() {
    let mut args = send_args("AT");
    args.usb.timeout = Some(0);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(error.to_string().contains("--timeout"));
}

struct TestConfirmationPrompt {
    allow: bool,
    calls: usize,
    notices: usize,
}

impl ConfirmationPrompt for TestConfirmationPrompt {
    fn notice_external_definition(
        &mut self,
        _external_source: ExternalDefinitionSource<'_>,
    ) -> Result<()> {
        self.notices += 1;
        Ok(())
    }

    fn confirm(
        &mut self,
        _operation: &OperationIdentity,
        _review: &ExecutionReview,
        _external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        self.calls += 1;
        if self.allow {
            Ok(())
        } else {
            Err(AtctlError::ConfirmationRequired)
        }
    }
}

fn send_args(command: &str) -> SendArgs {
    SendArgs {
        command: command.to_owned(),
        usb: UsbOptions {
            timeout: Some(3),
            ..UsbOptions::default()
        },
        no_mask: false,
        no_log: false,
        export_response: None,
        raw_log_file: None,
        raw_log_ack: None,
        json: false,
        ignore_at_error: false,
        yes: false,
        allow_unknown: false,
    }
}

fn preset_run_args(name: &str) -> PresetRunArgs {
    PresetRunArgs {
        name: name.to_owned(),
        usb: UsbOptions {
            timeout: Some(3),
            ..UsbOptions::default()
        },
        no_mask: false,
        no_log: false,
        export_response: None,
        raw_log_file: None,
        raw_log_ack: None,
        json: false,
        ignore_at_error: false,
        yes: false,
        allow_unknown: false,
        preset_locations: PresetFileLocationOptions::default(),
    }
}

#[test]
fn no_log_skips_normal_logging_path_resolution() {
    assert!(normal_logging_paths(true).unwrap().is_none());
}

#[test]
fn response_export_target_is_rejected_before_usb_access() {
    let directory = unique_temp_dir("response-export-before-usb");
    let path = directory.join("existing.response.txt");
    std::fs::write(&path, "existing").unwrap();
    let mut args = send_args("AT");
    args.export_response = Some(path.clone());

    let error = run_send(args).unwrap_err();

    assert!(matches!(
        error,
        AtctlError::ResponseExportFileExists { path: error_path }
            if error_path == path.display().to_string()
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "existing");
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("atctl-cli-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct FailingOpenTransport;

impl AtTransport for FailingOpenTransport {
    fn open(&mut self) -> Result<()> {
        Err(AtctlError::Transport("target selection failed".to_owned()))
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_command(&mut self, _command: &str) -> Result<()> {
        unreachable!("open failure must stop before write")
    }

    fn read_response(&mut self, _timeout: Duration) -> Result<Vec<u8>> {
        unreachable!("open failure must stop before read")
    }
}
