use clap::{CommandFactory, Parser};

use crate::presets::model::PresetOrigin;
use crate::transport::test_support::MockTransport;

use super::*;

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
    assert_eq!(args.usb.timeout, 180);
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
    assert!(root_help.contains("List or run product and loaded multi-step Sequences"));
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
fn accepts_risk_ack_values() {
    let cli = Cli::try_parse_from([
        "atctl",
        "send",
        "AT+CFUN=0",
        "--yes",
        "--risk-ack",
        "dangerous",
    ])
    .unwrap();

    let Command::Send(args) = cli.command else {
        panic!("expected send command");
    };

    assert_eq!(args.risk_ack, Some(RiskLevel::Dangerous));
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

    assert_eq!(args.usb.timeout, DEFAULT_COMMAND_TIMEOUT_SECS);
}

#[test]
fn parses_preset_run_transport_and_ack_options() {
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
        "--risk-ack",
        "write",
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
    assert_eq!(args.risk_ack, Some(RiskLevel::Write));
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
fn parses_sequence_run_transport_ack_params_and_locations() {
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
        "--risk-ack",
        "write",
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
    assert_eq!(args.usb.timeout, 180);
    assert!(args.yes);
    assert_eq!(args.risk_ack, Some(RiskLevel::Write));
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
    let output = format_sequence_list(&sequence_builtins());

    assert!(output.contains(
            "name\tsequence-set\tdeclared-risk\teffective-risk\ttimeout-secs\tcategories\trequired-params\tsummary\tsource-path"
        ));
    assert!(output.contains("sms-send-check\tProduct Sequences\twrite\twrite\t180\tsms\trecipient(sensitive),message(sensitive)"));
    assert!(output.contains("Send a standard SMS and report modem submit evidence.\t-"));
    assert!(output.contains("sms-receive-check\tProduct Sequences\twrite\twrite\t120\tsms"));
    assert!(output.contains(
        "sms-read-message\tProduct Sequences\twrite\twrite\t120\tsms\tindex(select,sms-message)"
    ));
    assert!(output.contains(
            "sms-reply-check\tProduct Sequences\twrite\twrite\t180\tsms\tindex(select,sms-message),message(sensitive)"
        ));
}

#[test]
fn sequence_json_output_includes_structured_steps_and_notes() {
    let execution = SequenceExecution {
        name: "sms-send-check".to_owned(),
        risk: RiskLevel::Write,
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

    let output = format_sequence_output(&execution, true, true).unwrap();
    let json: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(json["name"], "sms-send-check");
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
fn sequence_text_export_includes_identity_and_selected_transcript() {
    let execution = SequenceExecution {
        name: "sms-send-check".to_owned(),
        risk: RiskLevel::Write,
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
        format_sequence_export(&execution, true, false).unwrap(),
        "Sequence: sms-send-check\n\nmasked transcript\n"
    );
    assert_eq!(
        format_sequence_export(&execution, false, false).unwrap(),
        "Sequence: sms-send-check\n\nraw transcript\n"
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
            risk = "safe"
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
            risk = "safe"
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

    let output = format_sequence_list(&sequences);
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
            risk = "safe"
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
            risk = "safe"
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

    let output = format_preset_list(&presets);
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
fn formats_preset_list_with_risk_categories_and_command() {
    let presets = vec![Preset::new(
        "custom-modem-response",
        "AT",
        RiskLevel::Safe,
        vec!["custom".to_owned()],
        PresetOrigin::file("Custom commands", "presets.toml", None),
    )];

    let output = format_preset_list(&presets);

    assert!(output.contains(
        "name\tpreset-set\tdeclared-risk\teffective-risk\ttimeout-secs\tcategories\tcommand\tsource-path"
    ));
    assert!(output.contains(
        "custom-modem-response\tCustom commands\tsafe\tsafe\t-\tcustom\tAT\tpresets.toml"
    ));
}

#[test]
fn formats_product_preset_list_label() {
    let presets = vec![Preset::new(
        "modem-response",
        "AT",
        RiskLevel::Safe,
        vec!["basic".to_owned()],
        PresetOrigin::BuiltIn,
    )];

    let output = format_preset_list(&presets);

    assert!(output.contains("modem-response\tProduct presets\tsafe\tsafe\t-\tbasic\tAT"));
    assert!(output.contains("modem-response\tProduct presets\tsafe\tsafe\t-\tbasic\tAT\t-"));
    assert!(!output.contains("Built-in presets"));
}

#[test]
fn preset_lookup_uses_exact_name_instead_of_category() {
    let presets = vec![Preset::new(
        "current-operator",
        "AT+COPS?",
        RiskLevel::Safe,
        vec!["network".to_owned()],
        PresetOrigin::BuiltIn,
    )];

    assert_eq!(
        find_preset(&presets, "current-operator").unwrap().name,
        "current-operator"
    );
    assert!(matches!(
        find_preset(&presets, "network"),
        Err(AtctlError::PresetNotFound { name }) if name == "network"
    ));
}

#[test]
fn preset_execution_uses_explicit_preset_risk() {
    let preset = Preset::new(
        "write-labelled-at",
        "AT",
        RiskLevel::Write,
        Vec::new(),
        PresetOrigin::file("Custom commands", "presets.toml", None),
    );
    let run_args = preset_run_args("write-labelled-at");
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
        preset_classification(&preset),
        "preset:write-labelled-at",
        external_preset_source(&preset),
    )
    .unwrap();

    assert_eq!(execution.risk, RiskLevel::Write);
    assert_eq!(prompt.calls, 1);
    assert_eq!(prompt.notices, 0);
}

#[test]
fn file_preset_yes_risk_ack_still_reports_external_notice_before_transport() {
    let preset = Preset::new(
        "write-labelled-at",
        "AT",
        RiskLevel::Write,
        Vec::new(),
        PresetOrigin::file("Custom commands", "presets.toml", None),
    );
    let mut run_args = preset_run_args("write-labelled-at");
    run_args.yes = true;
    run_args.risk_ack = Some(RiskLevel::Write);
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
        preset_classification(&preset),
        "preset:write-labelled-at",
        external_preset_source(&preset),
    )
    .unwrap();

    assert_eq!(execution.risk, RiskLevel::Write);
    assert_eq!(prompt.calls, 0);
    assert_eq!(prompt.notices, 1);
}

#[test]
fn external_notice_helper_reports_only_when_confirmation_is_not_interactive() {
    let source = Some(ExternalDefinitionSource {
        label: "Custom commands",
        path: "presets.toml",
    });
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    notice_external_definition_without_interactive_confirmation(
        &mut prompt,
        DirectSendConfirmation::InteractiveRequired {
            risk: RiskLevel::Write,
        },
        source,
    )
    .unwrap();
    assert_eq!(prompt.notices, 0);

    notice_external_definition_without_interactive_confirmation(
        &mut prompt,
        DirectSendConfirmation::AutomationBypassApproved,
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
        RiskLevel::Safe,
        Vec::new(),
        PresetOrigin::BuiltIn,
        Some(180),
    );
    let mut run_args = preset_run_args("long-scan");
    run_args.usb.timeout = DEFAULT_COMMAND_TIMEOUT_SECS;

    let send_args = send_args_from_preset(&preset, &run_args);

    assert_eq!(send_args.usb.timeout, 180);
}

#[test]
fn explicit_preset_timeout_overrides_timeout_hint() {
    let preset = Preset::new_with_timeout(
        "long-scan",
        "AT+COPS=?",
        RiskLevel::Safe,
        Vec::new(),
        PresetOrigin::BuiltIn,
        Some(180),
    );
    let mut run_args = preset_run_args("long-scan");
    run_args.usb.timeout = 240;

    let send_args = send_args_from_preset(&preset, &run_args);

    assert_eq!(send_args.usb.timeout, 240);
}

#[test]
fn preset_no_log_option_is_forwarded_to_shared_send_execution() {
    let preset = Preset::new(
        "modem-info",
        "ATI",
        RiskLevel::Safe,
        Vec::new(),
        PresetOrigin::BuiltIn,
    );
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
    assert_eq!(execution.risk, RiskLevel::Safe);
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
fn send_no_mask_preserves_sensitive_response_for_display() {
    let mut args = send_args("AT+CIMI");
    args.no_mask = true;
    let transport = MockTransport::with_response(b"\r\n898110001234567\r\nOK\r\n".to_vec());

    let execution = execute_send_with_transport(&args, transport).unwrap();

    assert!(execution.text.contains("898110001234567"));
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

    let text = format_send_export(&args.command, &execution, false).unwrap();
    assert!(text.starts_with("AT+CIMI\n"));
    assert!(text.contains("89811000*******"));
    assert!(!text.contains("898110001234567"));

    let json = format_send_export(&args.command, &execution, true).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["command"], "AT+CIMI");
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
    args.risk_ack = Some(RiskLevel::Write);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let execution = execute_send_with_transport(&args, transport).unwrap();

    let text = format_send_export(command, &execution, false).unwrap();
    let json = format_send_export(command, &execution, true).unwrap();

    assert!(!text.contains("user"));
    assert!(!text.contains("password"));
    assert!(!json.contains("user"));
    assert!(!json.contains("password"));
    assert!(text.contains("AT+CGAUTH=1,1"));

    let mut unmasked_args = send_args(command);
    unmasked_args.no_mask = true;
    unmasked_args.yes = true;
    unmasked_args.risk_ack = Some(RiskLevel::Write);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let unmasked_execution = execute_send_with_transport(&unmasked_args, transport).unwrap();
    let unmasked = format_send_export(command, &unmasked_execution, false).unwrap();
    assert!(unmasked.contains("user"));
    assert!(unmasked.contains("password"));
}

#[test]
fn send_at_error_is_failure_unless_ignored() {
    let args = send_args("AT+EXAMPLE?");
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

    assert!(matches!(error, AtctlError::Timeout));
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains("\"event\":\"header\""));
    assert!(contents.contains("\"event\":\"transport_error\""));
    assert!(contents.contains("\"stage\":\"read_response\""));
    assert!(contents.contains("\"command\":\"AT+COPS?\""));
    assert!(contents.contains("\"tx_base64\":\"QVQrQ09QUz8N\""));
}

#[test]
fn send_write_command_requires_confirmation_without_yes() {
    let args = send_args("ATE0");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(
        error,
        AtctlError::ConfirmationRequired {
            risk: RiskLevel::Write
        }
    ));
}

#[test]
fn send_write_command_runs_after_interactive_confirmation() {
    let args = send_args("ATE0");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: true,
        calls: 0,
        notices: 0,
    };

    let execution = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert_eq!(execution.risk, RiskLevel::Write);
    assert_eq!(prompt.calls, 1);
}

#[test]
fn send_write_command_stops_when_interactive_confirmation_is_rejected() {
    let args = send_args("ATE0");
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    let error = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap_err();

    assert_eq!(prompt.calls, 1);
    assert!(matches!(
        error,
        AtctlError::ConfirmationRequired {
            risk: RiskLevel::Write
        }
    ));
}

#[test]
fn send_yes_and_matching_risk_ack_bypass_interactive_confirmation() {
    let mut args = send_args("AT+CFUN=0");
    args.yes = true;
    args.risk_ack = Some(RiskLevel::Dangerous);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: false,
        calls: 0,
        notices: 0,
    };

    let execution = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert_eq!(execution.risk, RiskLevel::Dangerous);
    assert_eq!(prompt.calls, 0);
}

#[test]
fn send_risk_ack_without_yes_does_not_bypass_interactive_confirmation() {
    let mut args = send_args("ATE0");
    args.risk_ack = Some(RiskLevel::Write);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());
    let mut prompt = TestConfirmationPrompt {
        allow: true,
        calls: 0,
        notices: 0,
    };

    let execution = execute_send_with_confirmation(&args, transport, &mut prompt).unwrap();

    assert_eq!(execution.status, AtStatus::Ok);
    assert_eq!(prompt.calls, 1);
}

#[test]
fn send_mismatched_risk_ack_fails_before_transport_access() {
    let mut args = send_args("AT");
    args.yes = true;
    args.risk_ack = Some(RiskLevel::Dangerous);
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(
        error,
        AtctlError::RiskAckMismatch {
            classified: RiskLevel::Safe,
            acknowledged: RiskLevel::Dangerous
        }
    ));
}

#[test]
fn send_validates_risk_before_raw_log_option() {
    let mut args = send_args("AT+CFUN=0");
    args.raw_log_file = Some(unique_temp_dir("raw-log-risk").join("case.rawlog"));
    args.yes = true;
    let transport = MockTransport::with_response(b"\r\nOK\r\n".to_vec());

    let error = execute_send_with_transport(&args, transport).unwrap_err();

    assert!(matches!(
        error,
        AtctlError::MissingRiskAck {
            risk: RiskLevel::Dangerous
        }
    ));
}

#[test]
fn send_rejects_zero_timeout_before_transport_access() {
    let mut args = send_args("AT");
    args.usb.timeout = 0;
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
        classification: &RiskClassification,
        _external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        self.calls += 1;
        if self.allow {
            Ok(())
        } else {
            Err(AtctlError::ConfirmationRequired {
                risk: classification.risk,
            })
        }
    }
}

fn send_args(command: &str) -> SendArgs {
    SendArgs {
        command: command.to_owned(),
        usb: UsbOptions {
            timeout: 3,
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
        risk_ack: None,
    }
}

fn preset_run_args(name: &str) -> PresetRunArgs {
    PresetRunArgs {
        name: name.to_owned(),
        usb: UsbOptions {
            timeout: 3,
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
        risk_ack: None,
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
