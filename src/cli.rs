use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use crate::app::operation::{DefinitionOrigin, ExecutionSurface, OperationIdentity, OperationKind};
use crate::at::command::{
    command_with_terminator, has_embedded_command_framing, normalize_command,
};
use crate::at::mask::{mask_declared_sensitive_value, mask_sensitive_values};
use crate::at::parser::parse_response;
use crate::at::response::AtStatus;
use crate::at::review::{
    Assessment, ExecutionAuthorization, ExecutionReview, authorize_execution,
    is_prompt_required_command, review_direct_command,
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
    execute_sequence, render_sequence_review, review_sequence_execution,
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
    about = "AT command controller for USB cellular modems",
    after_help = "Examples:
  atctl devices
  atctl send AT
  atctl tui"
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
    #[command(about = "List, inspect, or run one-shot product and loaded file presets")]
    Preset(PresetArgs),
    #[command(about = "List, inspect, or run product and loaded multi-step Sequences")]
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
        help = "Approve a known confirmation-required operation without an interactive prompt"
    )]
    pub yes: bool,

    #[arg(
        long = "allow-unknown",
        help = "Additionally authorize an unknown operation; requires --yes"
    )]
    pub allow_unknown: bool,
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
    #[command(about = "Show complete details for one loaded preset")]
    Show(PresetShowArgs),
    #[command(about = "Run one loaded preset by name")]
    Run(PresetRunArgs),
}

#[derive(Debug, Args)]
pub struct PresetListArgs {
    #[arg(long, help = "Print complete structured JSON records")]
    pub json: bool,

    #[command(flatten)]
    pub preset_locations: PresetFileLocationOptions,
}

#[derive(Debug, Args)]
pub struct PresetShowArgs {
    #[arg(help = "Preset name from atctl preset list")]
    pub name: String,

    #[arg(long, help = "Print complete structured JSON output")]
    pub json: bool,

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
        help = "Approve a known confirmation-required operation without an interactive prompt"
    )]
    pub yes: bool,

    #[arg(
        long = "allow-unknown",
        help = "Additionally authorize an unknown operation; requires --yes"
    )]
    pub allow_unknown: bool,

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
    #[command(about = "Show complete details for one loaded Sequence")]
    Show(SequenceShowArgs),
    #[command(about = "Run one loaded Sequence by name")]
    Run(SequenceRunArgs),
}

#[derive(Debug, Args)]
pub struct SequenceListArgs {
    #[arg(long, help = "Print complete structured JSON records")]
    pub json: bool,

    #[command(flatten)]
    pub sequence_locations: SequenceFileLocationOptions,
}

#[derive(Debug, Args)]
pub struct SequenceShowArgs {
    #[arg(help = "Sequence name from atctl sequence list")]
    pub name: String,

    #[arg(long, help = "Print complete structured JSON output")]
    pub json: bool,

    #[command(flatten)]
    pub sequence_locations: SequenceFileLocationOptions,
}

#[derive(Debug, Args)]
#[command(
    after_help = "Parameters are supplied with repeated --param NAME=VALUE options.
Examples:
  atctl sequence run sms-send-check --param recipient=+819012345678 --param message='hello' --yes
  atctl sequence run soracom-ping-check --sequence-dir examples/sequences --yes
  atctl sequence run soracom-unified-endpoint-tcp-send-check --sequence-dir examples/sequences --param payload='hello' --yes"
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
        help = "Approve a known confirmation-required operation without an interactive prompt"
    )]
    pub yes: bool,

    #[arg(
        long = "allow-unknown",
        help = "Additionally authorize an unknown operation; requires --yes"
    )]
    pub allow_unknown: bool,

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
    pub(crate) surface: ExecutionSurface,
    pub(crate) operation: OperationIdentity,
    pub(crate) masked_operation: OperationIdentity,
    pub(crate) review: ExecutionReview,
    pub(crate) masked_review: ExecutionReview,
    pub(crate) status: AtStatus,
    pub(crate) text: String,
    pub(crate) lines: Vec<String>,
    pub(crate) raw_text: String,
    pub(crate) masked_text: String,
    pub(crate) raw_response: Vec<u8>,
    pub(crate) masked: bool,
    pub(crate) duration: Duration,
}

#[derive(Debug, Serialize)]
struct SendJsonOutput<'a> {
    surface: ExecutionSurface,
    operation: OperationIdentity,
    execution_review: &'a ExecutionReview,
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
    record_command_logs(&args, &execution)?;
    let output = format_send_output(&execution, args.json)?;
    print!("{output}");
    if let Some(path) = args.export_response.as_deref() {
        let export = format_send_export(&execution, args.json)?;
        export_response(path, &export)?;
    }
    send_status_result(&execution, args.ignore_at_error)
}

fn run_preset(args: PresetArgs) -> Result<()> {
    match args.command {
        PresetCommand::List(list_args) => {
            let presets = load_presets(&list_args.preset_locations)?;
            print!("{}", format_preset_list(&presets, list_args.json)?);
            Ok(())
        }
        PresetCommand::Show(show_args) => {
            let presets = load_presets(&show_args.preset_locations)?;
            let preset = find_preset(&presets, &show_args.name)?;
            print!("{}", format_preset_show(preset, show_args.json)?);
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
            let mut prompt = StdioConfirmationPrompt;
            let execution = execute_command_with_confirmation(
                &send_args,
                transport,
                &mut prompt,
                preset.review.clone(),
                ExecutionSurface::Cli,
                OperationIdentity::preset(preset),
                external_preset_source(preset),
            )?;
            record_command_logs(&send_args, &execution)?;
            let output = format_send_output(&execution, send_args.json)?;
            print!("{output}");
            if let Some(path) = send_args.export_response.as_deref() {
                let export = format_send_export(&execution, send_args.json)?;
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
            print!("{}", format_sequence_list(&sequences, list_args.json)?);
            Ok(())
        }
        SequenceCommand::Show(show_args) => {
            let sequences = load_sequences(&show_args.sequence_locations)?;
            let sequence = find_sequence(&sequences, &show_args.name)?;
            print!("{}", format_sequence_show(sequence, show_args.json)?);
            Ok(())
        }
        SequenceCommand::Run(run_args) => {
            validate_optional_response_export(run_args.export_response.as_deref())?;
            let sequences = load_sequences(&run_args.sequence_locations)?;
            let sequence = find_sequence(&sequences, &run_args.name)?;
            let review = render_sequence_review(sequence, &run_args.params)?;
            let execution_review = review_sequence_execution(sequence, &run_args.params)?;
            let confirmation = validate_sequence_confirmation(
                &execution_review,
                run_args.yes,
                run_args.allow_unknown,
            )?;
            let external_source = external_sequence_source(sequence);
            let mut prompt = StdioConfirmationPrompt;
            if confirmation == ExecutionAuthorization::InteractiveRequired {
                prompt.confirm_sequence(
                    &execution_review,
                    &OperationIdentity::sequence(sequence),
                    &sequence.before_running,
                    &review,
                    external_source,
                )?;
            } else {
                notice_authorized_execution_without_interactive_confirmation(
                    &mut prompt,
                    confirmation,
                    &OperationIdentity::sequence(sequence),
                    &execution_review,
                    external_source,
                )?;
            }
            let raw_log = prepare_sequence_raw_log_config(&run_args)?;
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
            let output =
                format_sequence_output(sequence, &execution, !run_args.no_mask, run_args.json)?;
            print!("{output}");
            if let Some(path) = run_args.export_response.as_deref() {
                let export =
                    format_sequence_export(sequence, &execution, !run_args.no_mask, run_args.json)?;
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
        allow_unknown: confirmed && preset.review.assessment == Assessment::Unknown,
    };
    let manual_pair = send_args.usb.manual_endpoint_pair()?;
    let transport = UsbAtTransport::new(UsbAtTransportConfig {
        filter: send_args.usb.to_usb_filter(),
        manual_pair,
        timeout: Duration::from_secs(send_args.usb.timeout),
        probe_timeout: Duration::from_secs(ENDPOINT_PROBE_TIMEOUT_SECS),
    });
    let mut prompt = RejectingConfirmationPrompt;
    let execution = execute_command_with_confirmation(
        &send_args,
        transport,
        &mut prompt,
        preset.review.clone(),
        ExecutionSurface::Tui,
        OperationIdentity::preset(preset),
        None,
    )?;
    record_command_logs(&send_args, &execution)?;
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

    let execution_review = review_sequence_execution(sequence, params)?;
    let confirmation = validate_sequence_confirmation(
        &execution_review,
        confirmed,
        confirmed && execution_review.assessment == Assessment::Unknown,
    )?;
    if confirmation == ExecutionAuthorization::InteractiveRequired {
        return Err(AtctlError::ConfirmationRequired);
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
    let review = review_direct_command(&args.command);
    execute_command_with_confirmation(
        args,
        transport,
        prompt,
        review,
        ExecutionSurface::Cli,
        OperationIdentity::command(&args.command),
        None,
    )
}

fn execute_command_with_confirmation<T, P>(
    args: &SendArgs,
    mut transport: T,
    prompt: &mut P,
    review: ExecutionReview,
    surface: ExecutionSurface,
    operation: OperationIdentity,
    external_source: Option<ExternalDefinitionSource<'_>>,
) -> Result<SendExecution>
where
    T: AtTransport,
    P: ConfirmationPrompt,
{
    if is_prompt_required_command(&args.command) {
        return Err(AtctlError::PromptRequiredOneShot);
    }
    if has_embedded_command_framing(&args.command) {
        return Err(AtctlError::InvalidValue {
            name: "command",
            value: "embedded command framing or control characters are not allowed".to_owned(),
        });
    }
    let confirmation = authorize_execution(&review, args.yes, args.allow_unknown)?;
    if args.usb.timeout == 0 {
        return Err(AtctlError::InvalidValue {
            name: "--timeout",
            value: "must be greater than zero".to_owned(),
        });
    }

    if confirmation == ExecutionAuthorization::InteractiveRequired {
        prompt.confirm(&operation, &review, external_source)?;
    } else {
        notice_authorized_execution_without_interactive_confirmation(
            prompt,
            confirmation,
            &operation,
            &review,
            external_source,
        )?;
    }

    let raw_log = prepare_raw_log_config(args, prompt, surface)?;
    let started = Instant::now();
    transport.open()?;
    let current_review = review_direct_command(&args.command);
    if current_review != review {
        let _ = transport.close();
        return Err(AtctlError::OperationOutsideReviewedBoundary {
            operation: current_review.normalized_operation,
        });
    }
    let tx = command_with_terminator(&args.command);
    if let Err(error) = transport.write_command(&tx) {
        append_cli_raw_error(
            raw_log.as_ref(),
            RawLogTransportError {
                operation: &operation,
                command_name: None,
                command: &args.command,
                review: &review,
                duration: started.elapsed(),
                stage: "write_command",
                error: &error.to_string(),
                tx_bytes: tx.as_bytes(),
                rx_bytes: b"",
            },
        )?;
        let _ = transport.close();
        return Err(error);
    }
    let raw_response = match transport.read_response(Duration::from_secs(args.usb.timeout)) {
        Ok(raw_response) => raw_response,
        Err(error) => {
            append_cli_raw_error(
                raw_log.as_ref(),
                RawLogTransportError {
                    operation: &operation,
                    command_name: None,
                    command: &args.command,
                    review: &review,
                    duration: started.elapsed(),
                    stage: "read_response",
                    error: &error.to_string(),
                    tx_bytes: tx.as_bytes(),
                    rx_bytes: b"",
                },
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
    let masked_text = mask_sensitive_values(&raw_text);
    let masked_lines = masked_text.lines().map(str::to_owned).collect::<Vec<_>>();
    let text = if masked {
        masked_text.clone()
    } else {
        raw_text.clone()
    };
    let lines = if masked {
        masked_lines.clone()
    } else {
        raw_lines.clone()
    };
    let masked_operation = operation.masked();
    let masked_review = review.masked();

    let execution = SendExecution {
        surface,
        operation,
        masked_operation,
        review,
        masked_review,
        status,
        text,
        lines,
        raw_text,
        masked_text,
        raw_response,
        masked,
        duration,
    };

    if let Some(raw_log) = raw_log.as_ref() {
        let mut raw_log = RawLogSink::create(raw_log.clone())?;
        raw_log.append_exchange(RawLogExchange {
            operation: &execution.operation,
            command_name: None,
            command: &args.command,
            review: &execution.review,
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
    error: RawLogTransportError<'_>,
) -> Result<()> {
    let Some(raw_log) = raw_log else {
        return Ok(());
    };
    let mut raw_log = RawLogSink::create(raw_log.clone())?;
    raw_log.append_transport_error(error)
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
        "Review this external definition before running it; atctl validates format and duplicate names and derives the execution review from the final operation, but cannot establish device-, SIM-, network-, or endpoint-specific behavior that is not covered by a product rule."
    );
}

fn confirmation_token(review: &ExecutionReview) -> &'static str {
    if review.assessment == Assessment::Unknown {
        "unknown"
    } else {
        "run"
    }
}

fn print_operation_identity(operation: &OperationIdentity) {
    for line in operation_identity_output_lines(operation) {
        eprintln!("{line}");
    }
}

fn operation_identity_output_lines(operation: &OperationIdentity) -> Vec<String> {
    let mut lines = vec![format!(
        "Type: {}",
        match operation.kind {
            OperationKind::Command => "Command",
            OperationKind::Preset => "Preset",
            OperationKind::Sequence => "Sequence",
        }
    )];
    if let Some(name) = &operation.name {
        lines.push(format!("Name: {name}"));
    }
    if let Some(command) = &operation.command {
        lines.push(format!("AT command: {}", mask_sensitive_values(command)));
    }
    if let Some(purpose) = &operation.purpose {
        lines.push(format!("Purpose: {purpose}"));
    }
    lines
}

fn print_execution_review_sections(review: &ExecutionReview) {
    for line in execution_review_output_lines(review) {
        eprintln!("{line}");
    }
}

fn execution_review_output_lines(review: &ExecutionReview) -> Vec<String> {
    let mut lines = Vec::new();
    if review.assessment == Assessment::Unknown {
        lines.push("! Effects are not fully known".to_owned());
    }
    for section in review.presentation_sections() {
        lines.push(format!("{}:", section.kind.heading()));
        for item in section.items {
            lines.push(format!("  - {item}"));
        }
    }
    lines
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
        operation: &OperationIdentity,
        review: &ExecutionReview,
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()>;

    fn notice_authorized_execution(
        &mut self,
        operation: &OperationIdentity,
        review: &ExecutionReview,
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        let _ = (operation, review);
        if let Some(source) = external_source {
            self.notice_external_definition(source)?;
        }
        Ok(())
    }

    fn confirm_sequence(
        &mut self,
        review: &ExecutionReview,
        operation: &OperationIdentity,
        _before_running: &[String],
        _review_values: &[SequenceReviewValue],
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        self.confirm(operation, review, external_source)
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
        operation: &OperationIdentity,
        review: &ExecutionReview,
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        if !io::stdin().is_terminal() {
            return Err(AtctlError::ConfirmationRequired);
        }

        eprintln!("Command requires confirmation before sending.");
        print_operation_identity(operation);
        print_execution_review_sections(review);
        if let Some(source) = external_source {
            print_external_definition_notice(source);
        }
        let token = confirmation_token(review);
        eprint!("Type `{token}` to continue: ");
        io::stderr()
            .flush()
            .map_err(|error| AtctlError::Transport(format!("failed to flush prompt: {error}")))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|error| {
            AtctlError::Transport(format!("failed to read confirmation: {error}"))
        })?;

        if input.trim() == token {
            Ok(())
        } else {
            Err(AtctlError::ConfirmationRequired)
        }
    }

    fn confirm_sequence(
        &mut self,
        execution_review: &ExecutionReview,
        operation: &OperationIdentity,
        before_running: &[String],
        review: &[SequenceReviewValue],
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        if !io::stdin().is_terminal() {
            return Err(AtctlError::ConfirmationRequired);
        }

        eprintln!("Sequence requires confirmation before sending.");
        print_operation_identity(operation);
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
        print_execution_review_sections(execution_review);
        let token = confirmation_token(execution_review);
        eprint!("Type `{token}` to continue: ");
        io::stderr()
            .flush()
            .map_err(|error| AtctlError::Transport(format!("failed to flush prompt: {error}")))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|error| {
            AtctlError::Transport(format!("failed to read confirmation: {error}"))
        })?;

        if input.trim() == token {
            Ok(())
        } else {
            Err(AtctlError::ConfirmationRequired)
        }
    }

    fn notice_authorized_execution(
        &mut self,
        operation: &OperationIdentity,
        review: &ExecutionReview,
        external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        eprintln!("Authorized operation review:");
        print_operation_identity(operation);
        print_execution_review_sections(review);
        if let Some(source) = external_source {
            print_external_definition_notice(source);
        }
        Ok(())
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

fn notice_authorized_execution_without_interactive_confirmation<P>(
    prompt: &mut P,
    confirmation: ExecutionAuthorization,
    operation: &OperationIdentity,
    review: &ExecutionReview,
    external_source: Option<ExternalDefinitionSource<'_>>,
) -> Result<()>
where
    P: ConfirmationPrompt,
{
    if confirmation == ExecutionAuthorization::InteractiveRequired {
        return Ok(());
    }
    if confirmation == ExecutionAuthorization::NonInteractiveApproved {
        prompt.notice_authorized_execution(operation, review, external_source)
    } else if let Some(source) = external_source {
        prompt.notice_external_definition(source)
    } else {
        Ok(())
    }
}

struct RejectingConfirmationPrompt;

impl ConfirmationPrompt for RejectingConfirmationPrompt {
    fn confirm(
        &mut self,
        _operation: &OperationIdentity,
        _review: &ExecutionReview,
        _external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        Err(AtctlError::ConfirmationRequired)
    }
}

#[cfg(test)]
struct NonInteractiveConfirmationPrompt;

#[cfg(test)]
impl ConfirmationPrompt for NonInteractiveConfirmationPrompt {
    fn confirm(
        &mut self,
        _operation: &OperationIdentity,
        _review: &ExecutionReview,
        _external_source: Option<ExternalDefinitionSource<'_>>,
    ) -> Result<()> {
        Err(AtctlError::ConfirmationRequired)
    }
}

fn prepare_raw_log_config<P>(
    args: &SendArgs,
    prompt: &mut P,
    surface: ExecutionSurface,
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
    Ok(Some(RawLogConfig::new(path.clone(), surface)))
}

fn prepare_sequence_raw_log_config(args: &SequenceRunArgs) -> Result<Option<RawLogConfig>> {
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
    Ok(Some(RawLogConfig::new(path.clone(), ExecutionSurface::Cli)))
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
    Ok(Some(RawLogConfig::new(path.clone(), ExecutionSurface::Pty)))
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

#[derive(Debug, Serialize)]
struct PresetDiscovery<'a> {
    operation: OperationIdentity,
    categories: &'a [String],
    timeout_secs: Option<u64>,
    execution_review: &'a ExecutionReview,
}

#[derive(Debug, Serialize)]
struct SequenceDiscovery<'a> {
    operation: OperationIdentity,
    categories: &'a [String],
    timeout_secs: Option<u64>,
    before_running: &'a [String],
    parameters: Vec<SequenceParameterDiscovery<'a>>,
    execution_review: &'a ExecutionReview,
}

#[derive(Debug, Serialize)]
struct SequenceParameterDiscovery<'a> {
    name: &'a str,
    label: &'a str,
    required: bool,
    sensitive: bool,
    default: Option<&'a str>,
    source: &'static str,
    candidate: Option<&'static str>,
    hint: Option<&'a str>,
}

fn preset_discovery(preset: &Preset) -> PresetDiscovery<'_> {
    PresetDiscovery {
        operation: OperationIdentity::preset(preset),
        categories: &preset.categories,
        timeout_secs: preset.timeout_secs,
        execution_review: &preset.review,
    }
}

fn sequence_discovery(sequence: &Sequence) -> SequenceDiscovery<'_> {
    SequenceDiscovery {
        operation: OperationIdentity::sequence(sequence),
        categories: &sequence.categories,
        timeout_secs: sequence.timeout_secs,
        before_running: &sequence.before_running,
        parameters: sequence
            .params
            .iter()
            .map(|param| SequenceParameterDiscovery {
                name: &param.name,
                label: &param.label,
                required: param.required,
                sensitive: param.sensitive,
                default: param.default_value.as_deref(),
                source: param.source.label(),
                candidate: param.candidate.map(|candidate| candidate.label()),
                hint: param.hint.as_deref(),
            })
            .collect(),
        execution_review: &sequence.review,
    }
}

fn format_preset_list(presets: &[Preset], json: bool) -> Result<String> {
    if json {
        let rows = presets.iter().map(preset_discovery).collect::<Vec<_>>();
        return Ok(format!("{}\n", serde_json::to_string_pretty(&rows)?));
    }
    Ok(format_discovery_table(
        presets.iter().map(|preset| DiscoveryRow {
            name: &preset.name,
            preview: &preset.command,
            attention: confirmation_exception_label(&preset.review),
            source_title: (!preset.origin.is_built_in()).then(|| preset.origin.label()),
            source_path: preset.origin.file_path(),
        }),
        "AT COMMAND",
    ))
}

fn format_sequence_list(sequences: &[Sequence], json: bool) -> Result<String> {
    if json {
        let rows = sequences.iter().map(sequence_discovery).collect::<Vec<_>>();
        return Ok(format!("{}\n", serde_json::to_string_pretty(&rows)?));
    }
    Ok(format_discovery_table(
        sequences.iter().map(|sequence| DiscoveryRow {
            name: &sequence.name,
            preview: &sequence.summary,
            attention: confirmation_exception_label(&sequence.review),
            source_title: (!sequence.origin.is_built_in()).then(|| sequence.origin.label()),
            source_path: sequence.origin.file_path(),
        }),
        "PURPOSE",
    ))
}

fn format_preset_show(preset: &Preset, json: bool) -> Result<String> {
    if json {
        return Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&preset_discovery(preset))?
        ));
    }
    let mut lines = vec![
        "Type: Preset".to_owned(),
        format!("Name: {}", preset.name),
        format!("AT command: {}", preset.command),
    ];
    push_definition_lines(&mut lines, &OperationIdentity::preset(preset));
    lines.push(format!(
        "Categories: {}",
        category_summary(&preset.categories)
    ));
    lines.push(format!(
        "Timeout: {} seconds{}",
        preset.timeout_secs.unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS),
        if preset.timeout_secs.is_some() {
            ""
        } else {
            " (default)"
        }
    ));
    lines.push(String::new());
    lines.push("Execution review:".to_owned());
    lines.extend(complete_execution_review_output_lines(&preset.review));
    Ok(format!("{}\n", lines.join("\n")))
}

fn format_sequence_show(sequence: &Sequence, json: bool) -> Result<String> {
    if json {
        return Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&sequence_discovery(sequence))?
        ));
    }
    let mut lines = vec![
        "Type: Sequence".to_owned(),
        format!("Name: {}", sequence.name),
        format!("Purpose: {}", sequence.summary),
    ];
    push_definition_lines(&mut lines, &OperationIdentity::sequence(sequence));
    lines.push(format!(
        "Categories: {}",
        category_summary(&sequence.categories)
    ));
    lines.push(format!(
        "Timeout: {} seconds{}",
        sequence
            .timeout_secs
            .unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS),
        if sequence.timeout_secs.is_some() {
            ""
        } else {
            " (default)"
        }
    ));
    if !sequence.before_running.is_empty() {
        lines.push("Before running:".to_owned());
        lines.extend(
            sequence
                .before_running
                .iter()
                .map(|item| format!("  - {item}")),
        );
    }
    if sequence.params.is_empty() {
        lines.push("Parameters: none".to_owned());
    } else {
        lines.push("Parameters:".to_owned());
        for param in &sequence.params {
            lines.push(format!(
                "  - {} ({}){}{}",
                param.name,
                param.label,
                if param.required {
                    "; required"
                } else {
                    "; optional"
                },
                if param.sensitive { "; sensitive" } else { "" }
            ));
            lines.push(format!("    Source: {}", param.source.label()));
            if let Some(default) = &param.default_value {
                lines.push(format!("    Default: {default}"));
            }
            if let Some(candidate) = param.candidate {
                lines.push(format!("    Candidates: {}", candidate.label()));
            }
            if let Some(hint) = &param.hint {
                lines.push(format!("    Hint: {hint}"));
            }
        }
    }
    lines.push(String::new());
    lines.push("Execution review:".to_owned());
    lines.extend(complete_execution_review_output_lines(&sequence.review));
    Ok(format!("{}\n", lines.join("\n")))
}

struct DiscoveryRow<'a> {
    name: &'a str,
    preview: &'a str,
    attention: Option<&'static str>,
    source_title: Option<&'a str>,
    source_path: Option<&'a str>,
}

fn format_discovery_table<'a>(
    rows: impl Iterator<Item = DiscoveryRow<'a>>,
    preview_heading: &str,
) -> String {
    let rows = rows.collect::<Vec<_>>();
    let width = if io::stdout().is_terminal() {
        crossterm::terminal::size()
            .map(|(width, _)| usize::from(width))
            .unwrap_or(120)
    } else {
        120
    };
    let attention_width = rows
        .iter()
        .filter_map(|row| row.attention)
        .map(display_width)
        .max()
        .unwrap_or_else(|| display_width("ATTENTION"));
    let max_name = rows
        .iter()
        .map(|row| display_width(row.name))
        .max()
        .unwrap_or_else(|| display_width("NAME"));
    let name_width = max_name
        .max(display_width("NAME"))
        .min(28)
        .min(width.saturating_sub(attention_width + 5).max(1));
    let preview_width = width
        .saturating_sub(name_width + attention_width + 4)
        .max(1);
    let mut output = format!(
        "{}  {}  {}\n",
        pad_display("NAME", name_width),
        pad_display(preview_heading, preview_width),
        "ATTENTION"
    );
    let mut current_source: Option<(&str, Option<&str>)> = None;
    for row in rows {
        let source = row.source_title.map(|title| (title, row.source_path));
        if source != current_source {
            if let Some(title) = row.source_title {
                output.push('\n');
                output.push_str(&format!("Loaded from: {title}\n"));
                if let Some(path) = row.source_path {
                    output.push_str(&format!("File: {path}\n"));
                }
            }
            current_source = source;
        }
        output.push_str(&format!(
            "{}  {}  {}\n",
            pad_display(&truncate_display(row.name, name_width), name_width),
            pad_display(&truncate_display(row.preview, preview_width), preview_width),
            row.attention.unwrap_or("")
        ));
    }
    output
}

fn category_summary(categories: &[String]) -> String {
    if categories.is_empty() {
        "none".to_owned()
    } else {
        categories.join(", ")
    }
}

fn push_definition_lines(lines: &mut Vec<String>, operation: &OperationIdentity) {
    let Some(definition) = &operation.definition else {
        return;
    };
    lines.push(format!(
        "Definition origin: {}",
        match definition.origin {
            DefinitionOrigin::Product => "product",
            DefinitionOrigin::File => "file",
            DefinitionOrigin::Runtime => "runtime",
        }
    ));
    if let Some(title) = &definition.title {
        lines.push(format!("Definition title: {title}"));
    }
    if let Some(path) = &definition.path {
        lines.push(format!("Definition file: {path}"));
    }
}

fn display_width(value: &str) -> usize {
    ratatui::text::Line::from(value.to_owned()).width()
}

fn truncate_display(value: &str, width: usize) -> String {
    if display_width(value) <= width {
        return value.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_owned();
    }
    let mut output = String::new();
    let target = width - 1;
    for character in value.chars() {
        let candidate = format!("{output}{character}");
        if display_width(&candidate) > target {
            break;
        }
        output.push(character);
    }
    output.push('…');
    output
}

fn pad_display(value: &str, width: usize) -> String {
    format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(display_width(value)))
    )
}

fn confirmation_exception_label(review: &ExecutionReview) -> Option<&'static str> {
    if !review.requires_confirmation() {
        None
    } else if review.assessment == Assessment::Unknown {
        Some("! Confirmation required (effects unknown)")
    } else {
        Some("! Confirmation required")
    }
}

fn complete_execution_review_output_lines(review: &ExecutionReview) -> Vec<String> {
    fn values<'a>(items: impl Iterator<Item = &'a str>) -> String {
        let items = items.collect::<Vec<_>>();
        if items.is_empty() {
            "none".to_owned()
        } else {
            items.join("; ")
        }
    }

    let mut lines = vec![
        format!("Assessment: {}", review.assessment),
        format!(
            "Effects: {}",
            values(review.effects.iter().map(|effect| effect.label()))
        ),
        format!(
            "Data handling: {}",
            values(review.data_handling.iter().map(|item| item.label()))
        ),
        format!("Reversibility: {}", review.reversibility.label()),
        format!(
            "Consequences: {}",
            values(review.consequences.iter().map(String::as_str))
        ),
    ];
    if !review.not_established.is_empty() {
        lines.push(format!(
            "Not established: {}",
            values(review.not_established.iter().map(String::as_str))
        ));
    }
    lines.push(format!(
        "Confirmation: {}",
        if review.requires_confirmation() {
            "required"
        } else {
            "not required"
        }
    ));
    if !review.confirmation_reasons.is_empty() {
        lines.push(format!(
            "Confirmation reasons: {}",
            values(review.confirmation_reasons.iter().map(String::as_str))
        ));
    }
    lines
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
        allow_unknown: args.allow_unknown,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoggingPaths {
    pub(crate) state_dir: PathBuf,
    pub(crate) session_dir: PathBuf,
}

fn record_command_logs(args: &SendArgs, execution: &SendExecution) -> Result<()> {
    let Some(paths) = normal_logging_paths(args.no_log)? else {
        return Ok(());
    };
    let record = CommandLogRecord {
        timestamp: now_timestamp(),
        surface: execution.surface,
        masked_operation: execution.masked_operation.clone(),
        masked_review: execution.masked_review.clone(),
        status: execution.status.clone(),
        duration: execution.duration,
        masked_response: execution.masked_text.clone(),
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
        surface: ExecutionSurface::Cli,
        masked_operation: OperationIdentity::sequence(sequence).masked(),
        masked_review: execution.masked_review.clone(),
        status: execution.status.clone(),
        duration: execution.duration,
        masked_response: execution.masked_transcript.clone(),
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
        surface: ExecutionSurface::Tui,
        masked_operation: OperationIdentity::sequence(sequence).masked(),
        masked_review: execution.masked_review.clone(),
        status: execution.status.clone(),
        duration: execution.duration,
        masked_response: execution.masked_transcript.clone(),
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
        let review = if execution.masked {
            execution.masked_review.clone()
        } else {
            execution.review.clone()
        };
        let operation = if execution.masked {
            execution.masked_operation.clone()
        } else {
            execution.operation.clone()
        };
        let output = SendJsonOutput {
            surface: execution.surface,
            operation,
            execution_review: &review,
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
    surface: ExecutionSurface,
    operation: OperationIdentity,
    execution_review: &'a ExecutionReview,
    status: &'a AtStatus,
    masked: bool,
    response: &'a str,
    lines: &'a [String],
}

fn format_send_export(execution: &SendExecution, json: bool) -> Result<String> {
    let operation = if execution.masked {
        execution.masked_operation.clone()
    } else {
        execution.operation.clone()
    };
    if json {
        let review = if execution.masked {
            execution.masked_review.clone()
        } else {
            execution.review.clone()
        };
        let output = SendExportJsonOutput {
            surface: execution.surface,
            operation,
            execution_review: &review,
            status: &execution.status,
            masked: execution.masked,
            response: &execution.text,
            lines: &execution.lines,
        };
        return Ok(format!("{}\n", serde_json::to_string(&output)?));
    }

    let response = execution.text.trim_end_matches(['\r', '\n']);
    let command = operation.command.as_deref().unwrap_or("AT command");
    let starts_with_command = response
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| normalize_command(line) == normalize_command(command));
    if starts_with_command {
        Ok(format!("{response}\n"))
    } else {
        Ok(format!("{command}\n{response}\n"))
    }
}

#[derive(Debug, Serialize)]
struct SequenceJsonOutput<'a> {
    surface: ExecutionSurface,
    operation: OperationIdentity,
    execution_review: &'a ExecutionReview,
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
    sequence: &Sequence,
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
        let review = if masked {
            execution.masked_review.clone()
        } else {
            execution.review.clone()
        };
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
            surface: ExecutionSurface::Cli,
            operation: OperationIdentity::sequence(sequence),
            execution_review: &review,
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
    sequence: &Sequence,
    execution: &SequenceExecution,
    masked: bool,
    json: bool,
) -> Result<String> {
    if json {
        return format_sequence_output(sequence, execution, masked, true);
    }

    let transcript = if masked {
        &execution.masked_transcript
    } else {
        &execution.raw_transcript
    };
    Ok(format!(
        "Sequence: {}\nPurpose: {}\n\n{}\n",
        sequence.name,
        sequence.summary,
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
        println!("  {name}={}", mask_declared_sensitive_value(value));
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
