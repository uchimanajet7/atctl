use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::at::command::{command_with_terminator, normalize_command};
use crate::at::mask::{mask_identifier, mask_sensitive_values};
use crate::at::parser::parse_response;
use crate::at::response::AtStatus;
use crate::at::risk::{
    DirectSendConfirmation, RiskClassification, RiskLevel, classify_direct_command,
    direct_send_confirmation,
};
use crate::log::history::{LogListingKind, append_command_history, list_logs};
use crate::log::raw::{
    RAW_LOG_ACK, RawLogConfig, RawLogExchange, RawLogSink, RawLogTransportError,
    require_raw_log_ack, validate_raw_log_target,
};
use crate::log::session::{
    CommandLogRecord, LogDeviceSelection, now_timestamp, write_masked_session_log,
};
use crate::paths::default_state_dir;
use crate::presets::builtin::builtins;
use crate::presets::loader::{
    load_presets_dir_required, load_presets_file_required, validate_unique_preset_names,
};
use crate::presets::model::Preset;
use crate::response_export::{validate_response_export_target, write_response_export};
use crate::sequences::builtin::builtins as sequence_builtins;
use crate::sequences::engine::{
    SequenceExecution, SequenceParamValue, SequenceReviewValue, SequenceStepResult,
    execute_sequence, render_sequence_review, required_param_summary, sequence_classification,
    validate_sequence_confirmation,
};
use crate::sequences::loader::{
    load_sequences_dir_required, load_sequences_file_required, validate_unique_sequence_names,
};
use crate::sequences::model::Sequence;
use crate::transport::pty::{PtyBridgeConfig, run_usb_bridge};
use crate::transport::traits::AtTransport;
use crate::transport::usb::{UsbAtTransport, UsbAtTransportConfig};
use crate::usb::descriptor::UsbInspection;
use crate::usb::device::{UsbDeviceFilter, UsbDeviceInfo};
use crate::usb::endpoint::{EndpointPair, manual_override_pair};
use crate::usb::transport::{UsbDeviceListMode, inspect_devices, list_devices};
use crate::{AtctlError, Result};

pub(crate) const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 30;
pub(crate) const ENDPOINT_PROBE_TIMEOUT_SECS: u64 = 3;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct TuiDeviceSelection {
    pub vendor_id: u16,
    pub product_id: u16,
    pub bus: u8,
    pub address: u8,
}

#[derive(Debug, Parser)]
#[command(
    name = "atctl",
    version,
    about = "AT command controller for USB cellular modems"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "List USB devices that can be used for AT operations")]
    Devices(DevicesArgs),
    #[command(about = "Inspect USB interfaces and bulk endpoints")]
    Inspect(UsbOptions),
    #[command(about = "Send one AT command")]
    Send(SendArgs),
    #[command(about = "List or run one-shot product and loaded file presets")]
    Preset(PresetArgs),
    #[command(about = "List or run product and loaded multi-step Sequences")]
    Sequence(SequenceArgs),
    #[command(about = "Open the interactive TUI")]
    Tui(TuiArgs),
    #[command(about = "Expose the AT USB path as a local PTY")]
    Bridge(BridgeArgs),
    #[command(about = "List masked history and session logs")]
    Logs(LogsArgs),
}

#[derive(Debug, Args)]
pub struct DevicesArgs {
    #[arg(
        long,
        help = "Show every USB device visible through libusb, including non-AT devices"
    )]
    pub all_usb: bool,

    #[command(flatten)]
    pub filter: DeviceFilterOptions,
}

#[derive(Debug, Args, Clone, Default)]
pub struct DeviceFilterOptions {
    #[arg(long, value_parser = parse_hex_u16, help = "Filter by USB vendor ID, for example 0x2c7c")]
    pub vid: Option<u16>,

    #[arg(long, value_parser = parse_hex_u16, help = "Filter by USB product ID, for example 0x0125")]
    pub pid: Option<u16>,

    #[arg(long, value_parser = parse_decimal_u8, help = "Select a USB bus number from atctl devices output")]
    pub bus: Option<u8>,

    #[arg(long, value_parser = parse_decimal_u8, help = "Select a USB device address from atctl devices output")]
    pub address: Option<u8>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct UsbOptions {
    #[arg(long, value_parser = parse_hex_u16, help = "Filter by USB vendor ID, for example 0x2c7c")]
    pub vid: Option<u16>,

    #[arg(long, value_parser = parse_hex_u16, help = "Filter by USB product ID, for example 0x0125")]
    pub pid: Option<u16>,

    #[arg(long, value_parser = parse_decimal_u8, help = "Select a USB bus number from atctl devices output")]
    pub bus: Option<u8>,

    #[arg(long, value_parser = parse_decimal_u8, help = "Select a USB device address from atctl devices output")]
    pub address: Option<u8>,

    #[arg(long = "interface", value_parser = parse_decimal_u8, help = "Use a specific USB interface number")]
    pub interface_number: Option<u8>,

    #[arg(long = "bulk-in", value_parser = parse_hex_u8, help = "Use a specific bulk IN endpoint address, for example 0x85")]
    pub bulk_in: Option<u8>,

    #[arg(long = "bulk-out", value_parser = parse_hex_u8, help = "Use a specific bulk OUT endpoint address, for example 0x04")]
    pub bulk_out: Option<u8>,

    #[arg(long, default_value_t = DEFAULT_COMMAND_TIMEOUT_SECS, help = "Set the AT operation timeout in seconds")]
    pub timeout: u64,
}

#[derive(Debug, Args)]
pub struct SendArgs {
    #[arg(help = "AT command line to send, for example AT or ATI")]
    pub command: String,

    #[command(flatten)]
    pub usb: UsbOptions,

    #[arg(
        long,
        help = "Print unmasked foreground output; logs and history remain masked"
    )]
    pub no_mask: bool,

    #[arg(
        long,
        help = "Do not write masked history or session logs; explicit raw diagnostic export is unaffected"
    )]
    pub no_log: bool,

    #[arg(
        long = "export-response",
        value_name = "PATH",
        help = "Export the normal Response to a new file at PATH; follows --no-mask and does not replace stdout"
    )]
    pub export_response: Option<PathBuf>,

    #[arg(
        long = "raw-log-file",
        value_name = "PATH",
        help = "Write an acknowledged raw diagnostic export to PATH"
    )]
    pub raw_log_file: Option<PathBuf>,

    #[arg(
        long = "raw-log-ack",
        value_name = "raw-log",
        help = "Acknowledge raw diagnostic export by passing raw-log"
    )]
    pub raw_log_ack: Option<String>,

    #[arg(long, help = "Print structured JSON output")]
    pub json: bool,

    #[arg(long, help = "Exit successfully even when the modem returns AT ERROR")]
    pub ignore_at_error: bool,

    #[arg(
        long,
        help = "Run without an interactive risk prompt when paired with --risk-ack"
    )]
    pub yes: bool,

    #[arg(long = "risk-ack", value_parser = parse_risk_level, help = "Acknowledge the classified risk level for --yes")]
    pub risk_ack: Option<RiskLevel>,
}

#[derive(Debug, Args)]
pub struct PresetArgs {
    #[command(subcommand)]
    pub command: PresetCommand,
}

#[derive(Debug, Subcommand)]
pub enum PresetCommand {
    #[command(about = "List product and loaded file presets")]
    List(PresetListArgs),
    #[command(about = "Run one loaded preset by name")]
    Run(PresetRunArgs),
}

#[derive(Debug, Args)]
pub struct PresetListArgs {
    #[command(flatten)]
    pub preset_locations: PresetFileLocationOptions,
}

#[derive(Debug, Args)]
pub struct PresetRunArgs {
    #[arg(help = "Preset name from atctl preset list")]
    pub name: String,

    #[command(flatten)]
    pub usb: UsbOptions,

    #[arg(
        long,
        help = "Print unmasked foreground output; logs and history remain masked"
    )]
    pub no_mask: bool,

    #[arg(
        long,
        help = "Do not write masked history or session logs; explicit raw diagnostic export is unaffected"
    )]
    pub no_log: bool,

    #[arg(
        long = "export-response",
        value_name = "PATH",
        help = "Export the normal Response to a new file at PATH; follows --no-mask and does not replace stdout"
    )]
    pub export_response: Option<PathBuf>,

    #[arg(
        long = "raw-log-file",
        value_name = "PATH",
        help = "Write an acknowledged raw diagnostic export to PATH"
    )]
    pub raw_log_file: Option<PathBuf>,

    #[arg(
        long = "raw-log-ack",
        value_name = "raw-log",
        help = "Acknowledge raw diagnostic export by passing raw-log"
    )]
    pub raw_log_ack: Option<String>,

    #[arg(long, help = "Print structured JSON output")]
    pub json: bool,

    #[arg(long, help = "Exit successfully even when the modem returns AT ERROR")]
    pub ignore_at_error: bool,

    #[arg(
        long,
        help = "Run without an interactive risk prompt when paired with --risk-ack"
    )]
    pub yes: bool,

    #[arg(long = "risk-ack", value_parser = parse_risk_level, help = "Acknowledge the classified risk level for --yes")]
    pub risk_ack: Option<RiskLevel>,

    #[command(flatten)]
    pub preset_locations: PresetFileLocationOptions,
}

#[derive(Debug, Args)]
pub struct SequenceArgs {
    #[command(subcommand)]
    pub command: SequenceCommand,
}

#[derive(Debug, Subcommand)]
pub enum SequenceCommand {
    #[command(about = "List product and loaded Sequence definitions")]
    List(SequenceListArgs),
    #[command(about = "Run one loaded Sequence by name")]
    Run(SequenceRunArgs),
}

#[derive(Debug, Args)]
pub struct SequenceListArgs {
    #[command(flatten)]
    pub sequence_locations: SequenceFileLocationOptions,
}

#[derive(Debug, Args)]
#[command(
    after_help = "Parameters are supplied with repeated --param NAME=VALUE options.
Examples:
  atctl sequence run sms-send-check --param recipient=+819012345678 --param message='hello' --yes --risk-ack write
  atctl sequence run soracom-ping-check --sequence-dir examples/sequences --yes --risk-ack write
  atctl sequence run soracom-unified-endpoint-tcp-send-check --sequence-dir examples/sequences --param payload='hello' --yes --risk-ack write"
)]
pub struct SequenceRunArgs {
    #[arg(help = "Sequence name from atctl sequence list")]
    pub name: String,

    #[command(flatten)]
    pub usb: UsbOptions,

    #[arg(
        long,
        help = "Print unmasked foreground output; logs and history remain masked"
    )]
    pub no_mask: bool,

    #[arg(
        long,
        help = "Do not write masked history or session logs; explicit raw diagnostic export is unaffected"
    )]
    pub no_log: bool,

    #[arg(
        long = "export-response",
        value_name = "PATH",
        help = "Export the normal Response to a new file at PATH; follows --no-mask and does not replace stdout"
    )]
    pub export_response: Option<PathBuf>,

    #[arg(
        long = "raw-log-file",
        value_name = "PATH",
        help = "Write an acknowledged raw diagnostic export to PATH"
    )]
    pub raw_log_file: Option<PathBuf>,

    #[arg(
        long = "raw-log-ack",
        value_name = "raw-log",
        help = "Acknowledge raw diagnostic export by passing raw-log"
    )]
    pub raw_log_ack: Option<String>,

    #[arg(long, help = "Print structured JSON output")]
    pub json: bool,

    #[arg(long, help = "Exit successfully even when the modem returns AT ERROR")]
    pub ignore_at_error: bool,

    #[arg(
        long,
        help = "Run without an interactive risk prompt when paired with --risk-ack"
    )]
    pub yes: bool,

    #[arg(long = "risk-ack", value_parser = parse_risk_level, help = "Acknowledge the classified risk level for --yes")]
    pub risk_ack: Option<RiskLevel>,

    #[arg(long = "param", value_parser = parse_sequence_param_value, help = "Set one Sequence parameter as NAME=VALUE; repeat for multiple parameters")]
    pub params: Vec<SequenceParamValue>,

    #[command(flatten)]
    pub sequence_locations: SequenceFileLocationOptions,
}

#[derive(Debug, Args)]
pub struct TuiArgs {
    #[arg(long, help = "Start the TUI session with output masking off")]
    pub no_mask: bool,

    #[arg(
        long,
        help = "Do not write masked history or session logs during this TUI session; explicit raw diagnostic export is unaffected"
    )]
    pub no_log: bool,

    #[arg(long, value_enum, help = "Set the TUI color theme")]
    pub theme: Option<TuiThemeChoice>,

    #[command(flatten)]
    pub preset_locations: PresetFileLocationOptions,

    #[command(flatten)]
    pub sequence_locations: SequenceFileLocationOptions,
}

#[derive(Debug, Args, Clone, Default, PartialEq, Eq)]
pub struct PresetFileLocationOptions {
    #[arg(
        long = "preset-file",
        value_name = "FILE",
        help = "Load preset definitions from one TOML file for this invocation"
    )]
    pub preset_files: Vec<PathBuf>,

    #[arg(
        long = "preset-dir",
        value_name = "DIR",
        help = "Load preset definition TOML files from one directory for this invocation"
    )]
    pub preset_dirs: Vec<PathBuf>,
}

impl PresetFileLocationOptions {
    fn has_explicit_locations(&self) -> bool {
        !self.preset_files.is_empty() || !self.preset_dirs.is_empty()
    }
}

#[derive(Debug, Args, Clone, Default, PartialEq, Eq)]
pub struct SequenceFileLocationOptions {
    #[arg(
        long = "sequence-file",
        value_name = "FILE",
        help = "Load Sequence definitions from one TOML file for this invocation"
    )]
    pub sequence_files: Vec<PathBuf>,

    #[arg(
        long = "sequence-dir",
        value_name = "DIR",
        help = "Load Sequence definition TOML files from one directory for this invocation"
    )]
    pub sequence_dirs: Vec<PathBuf>,
}

impl SequenceFileLocationOptions {
    fn has_explicit_locations(&self) -> bool {
        !self.sequence_files.is_empty() || !self.sequence_dirs.is_empty()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum TuiThemeChoice {
    Dark,
    Light,
    NoColor,
}

#[derive(Debug, Args)]
#[command(after_help = "First-time workflow:
  1. Run `atctl devices`.
  2. Choose the target from the current AT operation target output.
  3. Prefer exact runtime selection with `--bus <BUS> --address <ADDRESS>`.
  4. Use `--vid <VID> --pid <PID>` only when that pair is unique in the current output.
  5. Run `atctl devices --all-usb` only when troubleshooting full USB visibility.
  6. Connect with `screen <SYMLINK> 115200`; 115200 is a serial-tool compatibility value.")]
pub struct BridgeArgs {
    #[arg(long, help = "PTY symlink path to create for screen or cu")]
    pub symlink: PathBuf,

    #[arg(long, help = "Replace an existing symlink at --symlink")]
    pub replace_symlink: bool,

    #[arg(
        long = "raw-log-file",
        value_name = "PATH",
        help = "Write an acknowledged raw diagnostic export to PATH"
    )]
    pub raw_log_file: Option<PathBuf>,

    #[arg(
        long = "raw-log-ack",
        value_name = "raw-log",
        help = "Acknowledge raw diagnostic export by passing raw-log"
    )]
    pub raw_log_ack: Option<String>,

    #[command(flatten)]
    pub usb: UsbOptions,
}

#[derive(Debug, Args)]
pub struct LogsArgs {
    #[command(subcommand)]
    pub command: LogsCommand,
}

#[derive(Debug, Subcommand)]
pub enum LogsCommand {
    #[command(about = "List recent masked history and session logs")]
    List,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    run_cli(cli)
}

pub fn run_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Send(args) => run_send(args),
        Command::Devices(args) => {
            let mode = if args.all_usb {
                UsbDeviceListMode::AllUsb
            } else {
                UsbDeviceListMode::AtTargets
            };
            let devices = list_devices(&args.filter.to_usb_filter(), mode)?;
            print_devices(&devices, mode);
            Ok(())
        }
        Command::Inspect(args) => {
            let manual_pair = args.manual_endpoint_pair()?;
            let inspections = inspect_devices(&args.to_usb_filter())?;
            print_inspections(&inspections, args.interface_number, manual_pair.as_ref());
            Ok(())
        }
        Command::Preset(args) => run_preset(args),
        Command::Sequence(args) => run_sequence(args),
        Command::Tui(args) => crate::tui::run(
            args.theme,
            args.no_mask,
            args.no_log,
            args.preset_locations,
            args.sequence_locations,
        ),
        Command::Bridge(args) => run_bridge(args),
        Command::Logs(args) => run_logs(args),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SendExecution {
    pub(crate) risk: RiskLevel,
    pub(crate) status: AtStatus,
    pub(crate) text: String,
    pub(crate) lines: Vec<String>,
    pub(crate) raw_text: String,
    pub(crate) raw_response: Vec<u8>,
    pub(crate) masked: bool,
    pub(crate) duration: Duration,
}

#[derive(Debug, Serialize)]
struct SendJsonOutput<'a> {
    risk: RiskLevel,
    status: &'a AtStatus,
    masked: bool,
    response: &'a str,
    lines: &'a [String],
}

fn run_send(args: SendArgs) -> Result<()> {
    validate_optional_response_export(args.export_response.as_deref())?;
    let manual_pair = args.usb.manual_endpoint_pair()?;
    let transport = UsbAtTransport::new(UsbAtTransportConfig {
        filter: args.usb.to_usb_filter(),
        manual_pair,
        timeout: Duration::from_secs(args.usb.timeout),
        probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
    });
    let mut prompt = StdioConfirmationPrompt;
    let execution = execute_send_with_confirmation(&args, transport, &mut prompt)?;
    record_command_logs("send", &args, &execution)?;
    let output = format_send_output(&execution, args.json)?;
    print!("{output}");
    if let Some(path) = args.export_response.as_deref() {
        let export = format_send_export(&args.command, &execution, args.json)?;
        export_response(path, &export)?;
    }
    send_status_result(&execution, args.ignore_at_error)
}

fn run_preset(args: PresetArgs) -> Result<()> {
    match args.command {
        PresetCommand::List(list_args) => {
            let presets = load_presets(&list_args.preset_locations)?;
            print!("{}", format_preset_list(&presets));
            Ok(())
        }
        PresetCommand::Run(run_args) => {
            validate_optional_response_export(run_args.export_response.as_deref())?;
            let presets = load_presets(&run_args.preset_locations)?;
            let preset = find_preset(&presets, &run_args.name)?;
            let send_args = send_args_from_preset(preset, &run_args);
            let manual_pair = send_args.usb.manual_endpoint_pair()?;
            let transport = UsbAtTransport::new(UsbAtTransportConfig {
                filter: send_args.usb.to_usb_filter(),
                manual_pair,
                timeout: Duration::from_secs(send_args.usb.timeout),
                probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
            });
            let classification = preset_classification(preset);
            let mut prompt = StdioConfirmationPrompt;
            let execution = execute_command_with_confirmation(
                &send_args,
                transport,
                &mut prompt,
                classification,
                &format!("preset:{}", preset.name),
                external_preset_source(preset),
            )?;
            record_command_logs(&format!("preset:{}", preset.name), &send_args, &execution)?;
            let output = format_send_output(&execution, send_args.json)?;
            print!("{output}");
            if let Some(path) = send_args.export_response.as_deref() {
                let export = format_send_export(&send_args.command, &execution, send_args.json)?;
                export_response(path, &export)?;
            }
            send_status_result(&execution, send_args.ignore_at_error)
        }
    }
}

fn run_sequence(args: SequenceArgs) -> Result<()> {
    match args.command {
        SequenceCommand::List(list_args) => {
            let sequences = load_sequences(&list_args.sequence_locations)?;
            print!("{}", format_sequence_list(&sequences));
            Ok(())
        }
        SequenceCommand::Run(run_args) => {
            validate_optional_response_export(run_args.export_response.as_deref())?;
            let sequences = load_sequences(&run_args.sequence_locations)?;
            let sequence = find_sequence(&sequences, &run_args.name)?;
            let review = render_sequence_review(sequence, &run_args.params)?;
            let confirmation =
                validate_sequence_confirmation(sequence, run_args.yes, run_args.risk_ack)?;
            let external_source = external_sequence_source(sequence);
            let mut prompt = StdioConfirmationPrompt;
            if let DirectSendConfirmation::InteractiveRequired { .. } = confirmation {
                prompt.confirm_sequence(
                    &sequence_classification(sequence),
                    &sequence.name,
                    &sequence.before_running,
                    &review,
                    external_source,
                )?;
            } else {
                notice_external_definition_without_interactive_confirmation(
                    &mut prompt,
                    confirmation,
                    external_source,
                )?;
            }
            let raw_log = prepare_sequence_raw_log_config(&run_args, sequence)?;
            let manual_pair = run_args.usb.manual_endpoint_pair()?;
            let transport = UsbAtTransport::new(UsbAtTransportConfig {
                filter: run_args.usb.to_usb_filter(),
                manual_pair,
                timeout: Duration::from_secs(run_args.usb.timeout),
                probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
            });
            let mut raw_log = raw_log.map(RawLogSink::create).transpose()?;
            let execution = execute_sequence(
                sequence,
                &run_args.params,
                transport,
                Duration::from_secs(run_args.usb.timeout),
                !run_args.no_mask,
                raw_log.as_mut(),
            )?;
            record_sequence_logs(sequence, &run_args, &execution)?;
            let output = format_sequence_output(&execution, !run_args.no_mask, run_args.json)?;
            print!("{output}");
            if let Some(path) = run_args.export_response.as_deref() {
                let export = format_sequence_export(&execution, !run_args.no_mask, run_args.json)?;
                export_response(path, &export)?;
            }
            sequence_status_result(&execution, run_args.ignore_at_error)
        }
    }
}

pub(crate) fn tui_device_filter() -> Result<UsbDeviceFilter> {
    let usb = configured_tui_usb_options(DEFAULT_COMMAND_TIMEOUT_SECS)?;
    Ok(usb.to_usb_filter())
}

pub(crate) fn execute_tui_preset(
    preset: &Preset,
    confirmed: bool,
    timeout_secs: u64,
    device_selection: Option<TuiDeviceSelection>,
    normal_logging_enabled: bool,
) -> Result<SendExecution> {
    let mut usb = configured_tui_usb_options(timeout_secs)?;
    if let Some(selection) = device_selection {
        usb.vid = Some(selection.vendor_id);
        usb.pid = Some(selection.product_id);
        usb.bus = Some(selection.bus);
        usb.address = Some(selection.address);
    }

    execute_tui_preset_with_usb(preset, confirmed, usb, normal_logging_enabled)
}

fn configured_tui_usb_options(timeout_secs: u64) -> Result<UsbOptions> {
    Ok(UsbOptions {
        timeout: timeout_secs,
        ..UsbOptions::default()
    })
}

fn execute_tui_preset_with_usb(
    preset: &Preset,
    confirmed: bool,
    usb: UsbOptions,
    normal_logging_enabled: bool,
) -> Result<SendExecution> {
    let requires_confirmation = preset.risk.requires_confirmation();
    let send_args = SendArgs {
        command: preset.command.clone(),
        usb,
        no_mask: false,
        no_log: !normal_logging_enabled,
        export_response: None,
        raw_log_file: None,
        raw_log_ack: None,
        json: false,
        ignore_at_error: false,
        yes: confirmed,
        risk_ack: (confirmed && requires_confirmation).then_some(preset.risk),
    };
    let manual_pair = send_args.usb.manual_endpoint_pair()?;
    let transport = UsbAtTransport::new(UsbAtTransportConfig {
        filter: send_args.usb.to_usb_filter(),
        manual_pair,
        timeout: Duration::from_secs(send_args.usb.timeout),
        probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
    });
    let classification = preset_classification(preset);
    let mut prompt = RejectingConfirmationPrompt;
    let execution = execute_command_with_confirmation(
        &send_args,
        transport,
        &mut prompt,
        classification,
        &format!("tui:{}", preset.name),
        None,
    )?;
    record_command_logs(&format!("tui:{}", preset.name), &send_args, &execution)?;
    Ok(execution)
}

pub(crate) fn execute_tui_sequence(
    sequence: &Sequence,
    params: &[SequenceParamValue],
    confirmed: bool,
    timeout_secs: u64,
    device_selection: Option<TuiDeviceSelection>,
    normal_logging_enabled: bool,
    raw_log: Option<&mut RawLogSink>,
) -> Result<SequenceExecution> {
    let mut usb = configured_tui_usb_options(timeout_secs)?;
    if let Some(selection) = device_selection {
        usb.vid = Some(selection.vendor_id);
        usb.pid = Some(selection.product_id);
        usb.bus = Some(selection.bus);
        usb.address = Some(selection.address);
    }

    let confirmation = validate_sequence_confirmation(
        sequence,
        confirmed,
        (confirmed && sequence.risk.requires_confirmation()).then_some(sequence.risk),
    )?;
    if let DirectSendConfirmation::InteractiveRequired { risk } = confirmation {
        return Err(AtctlError::ConfirmationRequired { risk });
    }

    let manual_pair = usb.manual_endpoint_pair()?;
    let transport = UsbAtTransport::new(UsbAtTransportConfig {
        filter: usb.to_usb_filter(),
        manual_pair,
        timeout: Duration::from_secs(usb.timeout),
        probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
    });
    let execution = execute_sequence(
        sequence,
        params,
        transport,
        Duration::from_secs(usb.timeout),
        true,
        raw_log,
    )?;
    record_tui_sequence_logs(sequence, &usb, &execution, normal_logging_enabled)?;
    Ok(execution)
}

fn run_logs(args: LogsArgs) -> Result<()> {
    match args.command {
        LogsCommand::List => {
            let paths = logging_paths()?;
            let listings = list_logs(&paths.state_dir, &paths.session_dir)?;
            if listings.is_empty() {
                println!("No logs found.");
                return Ok(());
            }

            for listing in listings {
                let kind = match listing.kind {
                    LogListingKind::History => "history",
                    LogListingKind::Session => "session",
                };
                println!("{kind}\t{}", listing.path.display());
            }

            Ok(())
        }
    }
}

fn run_bridge(args: BridgeArgs) -> Result<()> {
    let raw_log = bridge_raw_log_config(&args)?;
    let manual_pair = args.usb.manual_endpoint_pair()?;
    let config = PtyBridgeConfig {
        symlink: args.symlink,
        replace_symlink: args.replace_symlink,
        raw_log,
        usb: UsbAtTransportConfig {
            filter: args.usb.to_usb_filter(),
            manual_pair,
            timeout: Duration::from_secs(args.usb.timeout),
            probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
        },
        command_timeout: Duration::from_secs(args.usb.timeout),
    };
    run_usb_bridge(config)
}

#[cfg(test)]
fn execute_send_with_transport<T>(args: &SendArgs, transport: T) -> Result<SendExecution>
where
    T: AtTransport,
{
    let mut prompt = NonInteractiveConfirmationPrompt;
    execute_send_with_confirmation(args, transport, &mut prompt)
}

fn execute_send_with_confirmation<T, P>(
    args: &SendArgs,
    transport: T,
    prompt: &mut P,
) -> Result<SendExecution>
where
    T: AtTransport,
    P: ConfirmationPrompt,
{
    let classification = classify_direct_command(&args.command);
    execute_command_with_confirmation(args, transport, prompt, classification, "send", None)
}

fn execute_command_with_confirmation<T, P>(
    args: &SendArgs,
    mut transport: T,
    prompt: &mut P,
    classification: RiskClassification,
    source: &str,
    external_source: Option<ExternalDefinitionSource<'_>>,
) -> Result<SendExecution>
where
    T: AtTransport,
    P: ConfirmationPrompt,
{
    let confirmation = direct_send_confirmation(&classification, args.yes, args.risk_ack)?;
    if args.usb.timeout == 0 {
        return Err(AtctlError::InvalidValue {
            name: "--timeout",
            value: "must be greater than zero".to_owned(),
        });
    }

    if let DirectSendConfirmation::InteractiveRequired { .. } = confirmation {
        prompt.confirm(&classification, external_source)?;
    } else {
        notice_external_definition_without_interactive_confirmation(
            prompt,
            confirmation,
            external_source,
        )?;
    }

    let raw_log = prepare_raw_log_config(args, prompt, source)?;
    let started = Instant::now();
    transport.open()?;
    let tx = command_with_terminator(&args.command);
    if let Err(error) = transport.write_command(&tx) {
        append_cli_raw_error(
            raw_log.as_ref(),
            &args.command,
            classification.risk,
            started.elapsed(),
            "write_command",
            &error,
            tx.as_bytes(),
        )?;
        let _ = transport.close();
        return Err(error);
    }
    let raw_response = match transport.read_response(Duration::from_secs(args.usb.timeout)) {
        Ok(raw_response) => raw_response,
        Err(error) => {
            append_cli_raw_error(
                raw_log.as_ref(),
                &args.command,
                classification.risk,
                started.elapsed(),
                "read_response",
                &error,
                tx.as_bytes(),
            )?;
            let _ = transport.close();
            return Err(error);
        }
    };
    let close_result = transport.close();
    let response = parse_response(&raw_response);
    let duration = started.elapsed();

    let masked = !args.no_mask;
    let raw_response = response.raw;
    let raw_text = response.text;
    let raw_lines = response.lines;
    let status = response.status;
    let text = mask_if_needed(&raw_text, masked);
    let lines = raw_lines
        .iter()
        .map(|line| mask_if_needed(line, masked))
        .collect();

    let execution = SendExecution {
        risk: classification.risk,
        status,
        text,
        lines,
        raw_text,
        raw_response,
        masked,
        duration,
    };

    if let Some(raw_log) = raw_log.as_ref() {
        let mut raw_log = RawLogSink::create(raw_log.clone())?;
        raw_log.append_exchange(RawLogExchange {
            command_name: None,
            command: &args.command,
            risk: execution.risk,
            status: &execution.status,
            duration: execution.duration,
            tx_bytes: tx.as_bytes(),
            rx_bytes: &execution.raw_response,
        })?;
    }

    close_result?;
    Ok(execution)
}

fn append_cli_raw_error(
    raw_log: Option<&RawLogConfig>,
    command: &str,
    risk: RiskLevel,
    duration: Duration,
    stage: &'static str,
    error: &AtctlError,
    tx_bytes: &[u8],
) -> Result<()> {
    let Some(raw_log) = raw_log else {
        return Ok(());
    };
    let error = error.to_string();
    let mut raw_log = RawLogSink::create(raw_log.clone())?;
    raw_log.append_transport_error(RawLogTransportError {
        command_name: None,
        command,
        risk,
        duration,
        stage,
        error: &error,
        tx_bytes,
        rx_bytes: b"",
    })
}

#[derive(Debug, Copy, Clone)]
struct ExternalDefinitionSource<'a> {
    label: &'a str,
    path: &'a str,
}

fn external_preset_source(preset: &Preset) -> Option<ExternalDefinitionSource<'_>> {
    preset
        .origin
        .file_path()
        .map(|path| ExternalDefinitionSource {
            label: preset.origin.label(),
            path,
        })
}

fn external_sequence_source(sequence: &Sequence) -> Option<ExternalDefinitionSource<'_>> {
    sequence
        .origin
        .file_path()
        .map(|path| ExternalDefinitionSource {
            label: sequence.origin.label(),
            path,
        })
}

fn print_external_definition_notice(source: ExternalDefinitionSource<'_>) {
    eprintln!("Source: {}", source.label);
    eprintln!("File: {}", source.path);
    eprintln!(
        "Review this external definition before running it; atctl validates format, duplicate names, masking, and effective risk, but does not certify that the loaded commands are appropriate for your device, SIM, network, or endpoint."
    );
}

trait ConfirmationPrompt {
    fn notice_external_definition(
        &mut self,
        external_source: ExternalDefinitionSource<'_>,
    ) -> Result<()> {
        let _ = external_source;
        Ok(())
    }

    fn confirm(
        &mut self,
        classification: &RiskClassification,
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()>;

    fn confirm_sequence(
        &mut self,
        classification: &RiskClassification,
        _sequence_name: &str,
        _before_running: &[String],
        _review: &[SequenceReviewValue],
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        self.confirm(classification, external_source)
    }

    fn confirm_raw_log(&mut self, _path: &std::path::Path) -> Result<()> {
        Err(AtctlError::RawLogAckRequired)
    }
}

struct StdioConfirmationPrompt;

impl ConfirmationPrompt for StdioConfirmationPrompt {
    fn notice_external_definition(
        &mut self,
        external_source: ExternalDefinitionSource<'_>,
    ) -> Result<()> {
        print_external_definition_notice(external_source);
        Ok(())
    }

    fn confirm(
        &mut self,
        classification: &RiskClassification,
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        if !io::stdin().is_terminal() {
            return Err(AtctlError::ConfirmationRequired {
                risk: classification.risk,
            });
        }

        eprintln!("Command requires confirmation before sending.");
        if let Some(source) = external_source {
            print_external_definition_notice(source);
        }
        eprintln!("Command: {}", classification.normalized_command);
        eprintln!("Risk: {}", classification.risk);
        eprintln!("Reason: {}", classification.reason);
        eprint!("Type `{}` to continue: ", classification.risk);
        io::stderr()
            .flush()
            .map_err(|error| AtctlError::Transport(format!("failed to flush prompt: {error}")))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|error| {
            AtctlError::Transport(format!("failed to read confirmation: {error}"))
        })?;

        if input.trim() == classification.risk.to_string() {
            Ok(())
        } else {
            Err(AtctlError::ConfirmationRequired {
                risk: classification.risk,
            })
        }
    }

    fn confirm_sequence(
        &mut self,
        classification: &RiskClassification,
        sequence_name: &str,
        before_running: &[String],
        review: &[SequenceReviewValue],
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        if !io::stdin().is_terminal() {
            return Err(AtctlError::ConfirmationRequired {
                risk: classification.risk,
            });
        }

        eprintln!("Sequence requires confirmation before sending.");
        eprintln!("Sequence: {sequence_name}");
        if let Some(source) = external_source {
            print_external_definition_notice(source);
        }
        if !before_running.is_empty() {
            eprintln!("Before running:");
            for item in before_running {
                eprintln!("  {item}");
            }
        }
        if !review.is_empty() {
            eprintln!("Review:");
            for item in review {
                let sensitive = if item.sensitive { " (sensitive)" } else { "" };
                eprintln!("  {}{}: {}", item.label, sensitive, item.value);
            }
        }
        eprintln!("Risk: {}", classification.risk);
        eprintln!("Reason: {}", classification.reason);
        eprint!("Type `{}` to continue: ", classification.risk);
        io::stderr()
            .flush()
            .map_err(|error| AtctlError::Transport(format!("failed to flush prompt: {error}")))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|error| {
            AtctlError::Transport(format!("failed to read confirmation: {error}"))
        })?;

        if input.trim() == classification.risk.to_string() {
            Ok(())
        } else {
            Err(AtctlError::ConfirmationRequired {
                risk: classification.risk,
            })
        }
    }

    fn confirm_raw_log(&mut self, path: &std::path::Path) -> Result<()> {
        if !io::stdin().is_terminal() {
            return Err(AtctlError::RawLogAckRequired);
        }

        eprintln!(
            "Raw diagnostic export may contain sensitive modem, subscriber, network, APN, or PDP authentication values."
        );
        eprintln!("Raw log file: {}", path.display());
        eprint!("Type `{RAW_LOG_ACK}` to create this raw export: ");
        io::stderr()
            .flush()
            .map_err(|error| AtctlError::Transport(format!("failed to flush prompt: {error}")))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|error| {
            AtctlError::Transport(format!("failed to read raw-log acknowledgement: {error}"))
        })?;

        if input.trim() == RAW_LOG_ACK {
            Ok(())
        } else {
            Err(AtctlError::RawLogAckRequired)
        }
    }
}

fn notice_external_definition_without_interactive_confirmation<P>(
    prompt: &mut P,
    confirmation: DirectSendConfirmation,
    external_source: Option<ExternalDefinitionSource<'_>>,
) -> Result<()>
where
    P: ConfirmationPrompt,
{
    if matches!(
        confirmation,
        DirectSendConfirmation::InteractiveRequired { .. }
    ) {
        return Ok(());
    }
    if let Some(source) = external_source {
        prompt.notice_external_definition(source)?;
    }
    Ok(())
}

struct RejectingConfirmationPrompt;

impl ConfirmationPrompt for RejectingConfirmationPrompt {
    fn confirm(
        &mut self,
        classification: &RiskClassification,
        _external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        Err(AtctlError::ConfirmationRequired {
            risk: classification.risk,
        })
    }
}

#[cfg(test)]
struct NonInteractiveConfirmationPrompt;

#[cfg(test)]
impl ConfirmationPrompt for NonInteractiveConfirmationPrompt {
    fn confirm(
        &mut self,
        classification: &RiskClassification,
        _external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        Err(AtctlError::ConfirmationRequired {
            risk: classification.risk,
        })
    }
}

fn prepare_raw_log_config<P>(
    args: &SendArgs,
    prompt: &mut P,
    source: &str,
) -> Result<Option<RawLogConfig>>
where
    P: ConfirmationPrompt,
{
    let Some(path) = &args.raw_log_file else {
        if args.raw_log_ack.is_some() {
            return Err(AtctlError::InvalidValue {
                name: "--raw-log-ack",
                value: "requires --raw-log-file".to_owned(),
            });
        }
        return Ok(None);
    };

    match args.raw_log_ack.as_deref() {
        Some(_) => require_raw_log_ack(args.raw_log_ack.as_deref())?,
        None if args.yes || !io::stdin().is_terminal() => {
            return Err(AtctlError::RawLogAckRequired);
        }
        None => prompt.confirm_raw_log(path)?,
    }

    validate_raw_log_target(path)?;
    Ok(Some(RawLogConfig::new(
        path.clone(),
        "cli",
        source.to_owned(),
    )))
}

fn prepare_sequence_raw_log_config(
    args: &SequenceRunArgs,
    sequence: &Sequence,
) -> Result<Option<RawLogConfig>> {
    let Some(path) = &args.raw_log_file else {
        if args.raw_log_ack.is_some() {
            return Err(AtctlError::InvalidValue {
                name: "--raw-log-ack",
                value: "requires --raw-log-file".to_owned(),
            });
        }
        return Ok(None);
    };

    require_raw_log_ack(args.raw_log_ack.as_deref())?;
    validate_raw_log_target(path)?;
    Ok(Some(RawLogConfig::new(
        path.clone(),
        "cli",
        format!("sequence:{}", sequence.name),
    )))
}

fn bridge_raw_log_config(args: &BridgeArgs) -> Result<Option<RawLogConfig>> {
    let Some(path) = &args.raw_log_file else {
        if args.raw_log_ack.is_some() {
            return Err(AtctlError::InvalidValue {
                name: "--raw-log-ack",
                value: "requires --raw-log-file".to_owned(),
            });
        }
        return Ok(None);
    };

    require_raw_log_ack(args.raw_log_ack.as_deref())?;
    validate_raw_log_target(path)?;
    Ok(Some(RawLogConfig::new(path.clone(), "bridge", "bridge")))
}

pub(crate) fn load_presets(locations: &PresetFileLocationOptions) -> Result<Vec<Preset>> {
    let mut presets = builtins();
    if locations.has_explicit_locations() {
        for path in &locations.preset_files {
            presets.extend(load_presets_file_required(path)?);
        }
        for path in &locations.preset_dirs {
            presets.extend(load_presets_dir_required(path)?);
        }
    }
    validate_unique_preset_names(&presets)?;
    Ok(presets)
}

pub(crate) fn load_sequences(locations: &SequenceFileLocationOptions) -> Result<Vec<Sequence>> {
    let mut sequences = sequence_builtins();
    if locations.has_explicit_locations() {
        for path in &locations.sequence_files {
            sequences.extend(load_sequences_file_required(path)?);
        }
        for path in &locations.sequence_dirs {
            sequences.extend(load_sequences_dir_required(path)?);
        }
    }
    validate_unique_sequence_names(&sequences)?;
    Ok(sequences)
}

fn find_preset<'a>(presets: &'a [Preset], name: &str) -> Result<&'a Preset> {
    presets
        .iter()
        .find(|preset| preset.name == name)
        .ok_or_else(|| AtctlError::PresetNotFound {
            name: name.to_owned(),
        })
}

fn find_sequence<'a>(sequences: &'a [Sequence], name: &str) -> Result<&'a Sequence> {
    sequences
        .iter()
        .find(|sequence| sequence.name == name)
        .ok_or_else(|| AtctlError::SequenceNotFound {
            name: name.to_owned(),
        })
}

fn format_preset_list(presets: &[Preset]) -> String {
    let mut output = String::from(
        "name\tpreset-set\tdeclared-risk\teffective-risk\ttimeout-secs\tcategories\tcommand\tsource-path\n",
    );
    for preset in presets {
        let categories = if preset.categories.is_empty() {
            "-".to_owned()
        } else {
            preset.categories.join(",")
        };
        let timeout = preset
            .timeout_secs
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_owned());
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            preset.name,
            preset.origin.label(),
            preset.declared_risk,
            preset.risk,
            timeout,
            categories,
            preset.command,
            preset.origin.file_path().unwrap_or("-")
        ));
    }
    output
}

fn format_sequence_list(sequences: &[Sequence]) -> String {
    let mut output = String::from(
        "name\tsequence-set\tdeclared-risk\teffective-risk\ttimeout-secs\tcategories\trequired-params\tsummary\tsource-path\n",
    );
    for sequence in sequences {
        let categories = if sequence.categories.is_empty() {
            "-".to_owned()
        } else {
            sequence.categories.join(",")
        };
        let timeout = sequence
            .timeout_secs
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_owned());
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            sequence.name,
            sequence.origin.label(),
            sequence.declared_risk,
            sequence.risk,
            timeout,
            categories,
            required_param_summary(sequence),
            sequence.summary,
            sequence.origin.file_path().unwrap_or("-")
        ));
    }
    output
}

fn send_args_from_preset(preset: &Preset, args: &PresetRunArgs) -> SendArgs {
    let mut usb = args.usb.clone();
    if usb.timeout == DEFAULT_COMMAND_TIMEOUT_SECS
        && let Some(timeout_secs) = preset.timeout_secs
    {
        usb.timeout = timeout_secs;
    }

    SendArgs {
        command: preset.command.clone(),
        usb,
        no_mask: args.no_mask,
        no_log: args.no_log,
        export_response: args.export_response.clone(),
        raw_log_file: args.raw_log_file.clone(),
        raw_log_ack: args.raw_log_ack.clone(),
        json: args.json,
        ignore_at_error: args.ignore_at_error,
        yes: args.yes,
        risk_ack: args.risk_ack,
    }
}

fn preset_classification(preset: &Preset) -> RiskClassification {
    RiskClassification {
        normalized_command: normalize_command(&preset.command),
        risk: preset.risk,
        reason: "preset effective risk level",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoggingPaths {
    pub(crate) state_dir: PathBuf,
    pub(crate) session_dir: PathBuf,
}

fn record_command_logs(source: &str, args: &SendArgs, execution: &SendExecution) -> Result<()> {
    let Some(paths) = normal_logging_paths(args.no_log)? else {
        return Ok(());
    };
    let record = CommandLogRecord {
        timestamp: now_timestamp(),
        source: source.to_owned(),
        command: args.command.clone(),
        risk: execution.risk,
        status: execution.status.clone(),
        duration: execution.duration,
        response: execution.text.clone(),
        device: LogDeviceSelection {
            requested_vendor_id: args.usb.vid.map(hex_u16),
            requested_product_id: args.usb.pid.map(hex_u16),
            requested_bus: args.usb.bus,
            requested_address: args.usb.address,
            interface: args.usb.interface_number,
            bulk_in: args.usb.bulk_in.map(hex_u8),
            bulk_out: args.usb.bulk_out.map(hex_u8),
        },
    };

    append_command_history(&paths.state_dir, &record)?;
    write_masked_session_log(&paths.session_dir, &record)?;
    Ok(())
}

fn record_sequence_logs(
    sequence: &Sequence,
    args: &SequenceRunArgs,
    execution: &SequenceExecution,
) -> Result<()> {
    let Some(paths) = normal_logging_paths(args.no_log)? else {
        return Ok(());
    };
    let record = CommandLogRecord {
        timestamp: now_timestamp(),
        source: format!("sequence:{}", sequence.name),
        command: format!("sequence {}", sequence.name),
        risk: execution.risk,
        status: execution.status.clone(),
        duration: execution.duration,
        response: execution.masked_transcript.clone(),
        device: LogDeviceSelection {
            requested_vendor_id: args.usb.vid.map(hex_u16),
            requested_product_id: args.usb.pid.map(hex_u16),
            requested_bus: args.usb.bus,
            requested_address: args.usb.address,
            interface: args.usb.interface_number,
            bulk_in: args.usb.bulk_in.map(hex_u8),
            bulk_out: args.usb.bulk_out.map(hex_u8),
        },
    };

    append_command_history(&paths.state_dir, &record)?;
    write_masked_session_log(&paths.session_dir, &record)?;
    Ok(())
}

fn record_tui_sequence_logs(
    sequence: &Sequence,
    usb: &UsbOptions,
    execution: &SequenceExecution,
    normal_logging_enabled: bool,
) -> Result<()> {
    let Some(paths) = normal_logging_paths(!normal_logging_enabled)? else {
        return Ok(());
    };
    let record = CommandLogRecord {
        timestamp: now_timestamp(),
        source: format!("tui-sequence:{}", sequence.name),
        command: format!("sequence {}", sequence.name),
        risk: execution.risk,
        status: execution.status.clone(),
        duration: execution.duration,
        response: execution.masked_transcript.clone(),
        device: LogDeviceSelection {
            requested_vendor_id: usb.vid.map(hex_u16),
            requested_product_id: usb.pid.map(hex_u16),
            requested_bus: usb.bus,
            requested_address: usb.address,
            interface: usb.interface_number,
            bulk_in: usb.bulk_in.map(hex_u8),
            bulk_out: usb.bulk_out.map(hex_u8),
        },
    };

    append_command_history(&paths.state_dir, &record)?;
    write_masked_session_log(&paths.session_dir, &record)?;
    Ok(())
}

fn normal_logging_paths(no_log: bool) -> Result<Option<LoggingPaths>> {
    if no_log {
        return Ok(None);
    }

    logging_paths().map(Some)
}

pub(crate) fn logging_paths() -> Result<LoggingPaths> {
    let state_dir = default_state_dir()?;
    let session_dir = state_dir.join("logs");

    Ok(LoggingPaths {
        state_dir,
        session_dir,
    })
}

fn send_status_result(execution: &SendExecution, ignore_at_error: bool) -> Result<()> {
    if execution.status.is_success() || ignore_at_error {
        Ok(())
    } else {
        Err(AtctlError::AtCommandFailed {
            status: execution.status.clone(),
        })
    }
}

fn sequence_status_result(execution: &SequenceExecution, ignore_at_error: bool) -> Result<()> {
    if execution.status.is_success() || ignore_at_error {
        Ok(())
    } else {
        Err(AtctlError::AtCommandFailed {
            status: execution.status.clone(),
        })
    }
}

fn format_send_output(execution: &SendExecution, json: bool) -> Result<String> {
    if json {
        let output = SendJsonOutput {
            risk: execution.risk,
            status: &execution.status,
            masked: execution.masked,
            response: &execution.text,
            lines: &execution.lines,
        };
        return Ok(format!("{}\n", serde_json::to_string(&output)?));
    }

    Ok(execution.text.clone())
}

#[derive(Debug, Serialize)]
struct SendExportJsonOutput<'a> {
    command: String,
    risk: RiskLevel,
    status: &'a AtStatus,
    masked: bool,
    response: &'a str,
    lines: &'a [String],
}

fn format_send_export(command: &str, execution: &SendExecution, json: bool) -> Result<String> {
    let command = if execution.masked {
        mask_sensitive_values(command)
    } else {
        command.to_owned()
    };
    if json {
        let output = SendExportJsonOutput {
            command: command.clone(),
            risk: execution.risk,
            status: &execution.status,
            masked: execution.masked,
            response: &execution.text,
            lines: &execution.lines,
        };
        return Ok(format!("{}\n", serde_json::to_string(&output)?));
    }

    let response = execution.text.trim_end_matches(['\r', '\n']);
    let starts_with_command = response
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| normalize_command(line) == normalize_command(&command));
    if starts_with_command {
        Ok(format!("{response}\n"))
    } else {
        Ok(format!("{command}\n{response}\n"))
    }
}

#[derive(Debug, Serialize)]
struct SequenceJsonOutput<'a> {
    name: &'a str,
    risk: RiskLevel,
    status: &'a AtStatus,
    masked: bool,
    transcript: &'a str,
    steps: Vec<SequenceStepJsonOutput<'a>>,
    notes: Vec<&'a str>,
}

#[derive(Debug, Serialize)]
struct SequenceStepJsonOutput<'a> {
    id: &'a str,
    label: Option<&'a str>,
    status: &'a AtStatus,
    analysis: Option<&'a str>,
}

fn format_sequence_output(
    execution: &SequenceExecution,
    masked: bool,
    json: bool,
) -> Result<String> {
    let transcript = if masked {
        &execution.masked_transcript
    } else {
        &execution.raw_transcript
    };
    if json {
        let steps = execution
            .steps
            .iter()
            .map(|step| sequence_step_json_output(step, masked))
            .collect();
        let notes = if masked {
            execution.masked_notes.iter().map(String::as_str).collect()
        } else {
            execution.raw_notes.iter().map(String::as_str).collect()
        };
        let output = SequenceJsonOutput {
            name: &execution.name,
            risk: execution.risk,
            status: &execution.status,
            masked,
            transcript,
            steps,
            notes,
        };
        return Ok(format!("{}\n", serde_json::to_string(&output)?));
    }

    Ok(format!("{transcript}\n"))
}

fn format_sequence_export(
    execution: &SequenceExecution,
    masked: bool,
    json: bool,
) -> Result<String> {
    if json {
        return format_sequence_output(execution, masked, true);
    }

    let transcript = if masked {
        &execution.masked_transcript
    } else {
        &execution.raw_transcript
    };
    Ok(format!(
        "Sequence: {}\n\n{}\n",
        execution.name,
        transcript.trim_end_matches(['\r', '\n'])
    ))
}

fn validate_optional_response_export(path: Option<&Path>) -> Result<()> {
    match path {
        Some(path) => validate_response_export_target(path),
        None => Ok(()),
    }
}

fn export_response(path: &Path, contents: &str) -> Result<()> {
    write_response_export(path, contents)?;
    eprintln!("Exported response: {}", path.display());
    Ok(())
}

fn sequence_step_json_output(
    step: &SequenceStepResult,
    masked: bool,
) -> SequenceStepJsonOutput<'_> {
    SequenceStepJsonOutput {
        id: &step.id,
        label: step.label.as_deref(),
        status: &step.status,
        analysis: if masked {
            step.masked_analysis.as_deref()
        } else {
            step.raw_analysis.as_deref()
        },
    }
}

fn mask_if_needed(value: &str, masked: bool) -> String {
    if masked {
        mask_sensitive_values(value)
    } else {
        value.to_owned()
    }
}

impl DeviceFilterOptions {
    fn to_usb_filter(&self) -> UsbDeviceFilter {
        UsbDeviceFilter {
            vendor_id: self.vid,
            product_id: self.pid,
            bus: self.bus,
            address: self.address,
        }
    }
}

impl UsbOptions {
    fn to_usb_filter(&self) -> UsbDeviceFilter {
        UsbDeviceFilter {
            vendor_id: self.vid,
            product_id: self.pid,
            bus: self.bus,
            address: self.address,
        }
    }

    fn manual_endpoint_pair(&self) -> Result<Option<EndpointPair>> {
        let endpoint_override_present = self.bulk_in.is_some() || self.bulk_out.is_some();
        if !endpoint_override_present {
            return Ok(None);
        }

        let Some(interface_number) = self.interface_number else {
            return Err(AtctlError::InvalidValue {
                name: "--interface",
                value: "required when --bulk-in or --bulk-out is specified".to_owned(),
            });
        };
        let Some(bulk_in) = self.bulk_in else {
            return Err(AtctlError::InvalidValue {
                name: "--bulk-in",
                value: "required when --bulk-out is specified".to_owned(),
            });
        };
        let Some(bulk_out) = self.bulk_out else {
            return Err(AtctlError::InvalidValue {
                name: "--bulk-out",
                value: "required when --bulk-in is specified".to_owned(),
            });
        };

        if bulk_in & 0x80 == 0 {
            return Err(AtctlError::InvalidValue {
                name: "--bulk-in",
                value: format!("endpoint address {} is not IN", hex_u8(bulk_in)),
            });
        }
        if bulk_out & 0x80 != 0 {
            return Err(AtctlError::InvalidValue {
                name: "--bulk-out",
                value: format!("endpoint address {} is not OUT", hex_u8(bulk_out)),
            });
        }

        Ok(Some(manual_override_pair(
            interface_number,
            bulk_in,
            bulk_out,
        )))
    }
}

fn print_devices(devices: &[UsbDeviceInfo], mode: UsbDeviceListMode) {
    if devices.is_empty() {
        match mode {
            UsbDeviceListMode::AtTargets => {
                println!("No USB modem / AT candidate devices found.");
                println!(
                    "Run `atctl devices --all-usb` to inspect all USB devices visible through libusb."
                );
            }
            UsbDeviceListMode::AllUsb => println!("No matching USB devices found."),
        }
        return;
    }

    for device in devices {
        println!(
            "{} {}:{} bus={} address={}",
            device_label(device),
            hex_u16(device.vendor_id),
            hex_u16(device.product_id),
            device.bus,
            device.address
        );
        println!(
            "  class={:02x}/{:02x}/{:02x} configurations={}",
            device.class_code,
            device.sub_class_code,
            device.protocol_code,
            device.num_configurations
        );
        print_optional("manufacturer", device.manufacturer.as_deref());
        print_optional("product", device.product.as_deref());
        print_optional_masked("serial", device.serial_number.as_deref());
    }
}

fn print_inspections(
    inspections: &[UsbInspection],
    interface_override: Option<u8>,
    manual_pair: Option<&EndpointPair>,
) {
    if let Some(interface_number) = interface_override {
        println!("Manual interface override requested: interface={interface_number}");
    }
    if let Some(pair) = manual_pair {
        println!(
            "Manual endpoint override requested: selection={} interface={} bulk-in={} bulk-out={}",
            pair.selection,
            pair.interface_number,
            hex_u8(pair.bulk_in),
            hex_u8(pair.bulk_out)
        );
    }

    if inspections.is_empty() {
        println!("No matching USB devices found.");
        return;
    }

    for inspection in inspections {
        let device = &inspection.device;
        println!(
            "Device: {} {}:{} bus={} address={}",
            device_label(device),
            hex_u16(device.vendor_id),
            hex_u16(device.product_id),
            device.bus,
            device.address
        );
        println!(
            "  class={:02x}/{:02x}/{:02x} configurations={}",
            device.class_code,
            device.sub_class_code,
            device.protocol_code,
            device.num_configurations
        );

        for config in &inspection.configurations {
            println!(
                "  Configuration {}: self-powered={} remote-wakeup={} max-power={}mA",
                config.configuration_value,
                config.self_powered,
                config.remote_wakeup,
                config.max_power_ma
            );

            for interface in &config.interfaces {
                let override_marker = match interface_override {
                    Some(number) if number == interface.interface_number => {
                        " manual-interface-override-target"
                    }
                    _ => "",
                };
                println!(
                    "    Interface {} alt {}{} class={:02x}/{:02x}/{:02x}",
                    interface.interface_number,
                    interface.alternate_setting,
                    override_marker,
                    interface.class_code,
                    interface.sub_class_code,
                    interface.protocol_code
                );

                if interface.endpoints.is_empty() {
                    println!("      Endpoints: none");
                } else {
                    for endpoint in &interface.endpoints {
                        println!(
                            "      Endpoint {} direction={} transfer={} max-packet-size={}",
                            hex_u8(endpoint.address),
                            endpoint.direction,
                            endpoint.transfer_type,
                            endpoint.max_packet_size
                        );
                    }
                }

                if interface.descriptor_shape_pairs.is_empty() {
                    println!("      Descriptor-shape AT candidates: none");
                } else {
                    for pair in &interface.descriptor_shape_pairs {
                        println!(
                            "      Descriptor-shape AT candidate: interface={} alt={} bulk-in={} bulk-out={}",
                            pair.interface_number,
                            pair.alternate_setting.unwrap_or_default(),
                            hex_u8(pair.bulk_in),
                            hex_u8(pair.bulk_out)
                        );
                    }
                }
            }
        }
    }
}

fn print_optional(name: &str, value: Option<&str>) {
    if let Some(value) = value {
        println!("  {name}={value}");
    }
}

fn print_optional_masked(name: &str, value: Option<&str>) {
    if let Some(value) = value {
        println!("  {name}={}", mask_identifier(value));
    }
}

fn device_label(device: &UsbDeviceInfo) -> &str {
    device
        .product
        .as_deref()
        .or(device.manufacturer.as_deref())
        .unwrap_or("USB device")
}

fn hex_u16(value: u16) -> String {
    format!("0x{value:04x}")
}

fn hex_u8(value: u8) -> String {
    format!("0x{value:02x}")
}

fn parse_hex_u16(value: &str) -> std::result::Result<u16, String> {
    u16::from_str_radix(strip_hex_prefix(value), 16)
        .map_err(|_| format!("expected hexadecimal u16 value, got {value}"))
}

fn parse_hex_u8(value: &str) -> std::result::Result<u8, String> {
    u8::from_str_radix(strip_hex_prefix(value), 16)
        .map_err(|_| format!("expected hexadecimal u8 value, got {value}"))
}

fn parse_decimal_u8(value: &str) -> std::result::Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("expected decimal u8 value, got {value}"))
}

fn parse_risk_level(value: &str) -> std::result::Result<RiskLevel, String> {
    value.parse()
}

fn parse_sequence_param_value(value: &str) -> std::result::Result<SequenceParamValue, String> {
    let Some((name, param_value)) = value.split_once('=') else {
        return Err(format!("expected NAME=VALUE, got {value}"));
    };
    let name = name.trim();
    if name.is_empty() {
        return Err("parameter name must not be empty".to_owned());
    }
    Ok(SequenceParamValue {
        name: name.to_owned(),
        value: param_value.to_owned(),
    })
}

fn strip_hex_prefix(value: &str) -> &str {
    value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"))
        .unwrap_or_else(|| value.trim())
}

#[cfg(test)]
mod tests;
