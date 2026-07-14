use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

mod clipboard;
mod response_state;
mod theme;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Alignment, Frame, Rect};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, LineGauge, List, ListItem, Paragraph, Wrap};

use crate::at::command::{command_with_terminator, normalize_command};
use crate::at::mask::mask_sensitive_values;
use crate::at::risk::{RiskLevel, classify_direct_command, is_prompt_required_command};
use crate::cli::{
    DEFAULT_COMMAND_TIMEOUT_SECS, LoggingPaths, PresetFileLocationOptions, SendExecution,
    SequenceFileLocationOptions, TuiDeviceSelection, TuiThemeChoice, execute_tui_preset,
    execute_tui_sequence, load_presets, load_sequences, logging_paths, tui_device_filter,
};
use crate::log::history::{LogListingKind, list_logs};
use crate::log::raw::{
    RAW_LOG_ACK, RawLogConfig, RawLogExchange, RawLogSink, RawLogTransportError,
};
use crate::log::session::now_timestamp;
#[cfg(test)]
use crate::paths::default_state_dir;
use crate::presets::model::{Preset, PresetOrigin};
use crate::response_export::{response_export_path, write_response_export};
use crate::sequences::engine::{
    SequenceExecution, SequenceParamValue, SequenceValueCandidate, SequenceValueCandidateSet,
    format_missing_sequence_param, render_sequence_review, required_param_summary,
    value_candidate_sets_from_text,
};
use crate::sequences::model::{Sequence, SequenceOrigin};
use crate::sequences::model::{SequenceCandidateSource, SequenceParam, SequenceParamSource};
use crate::usb::device::UsbDeviceInfo;
use crate::usb::transport::{UsbDeviceListMode, list_devices};
use crate::{AtctlError, Result};

use self::clipboard::osc52_clipboard_sequence;
use self::response_state::ResponseState;
use self::theme::{TuiStyleRole, TuiTheme};

type CrosstermTerminal = Terminal<CrosstermBackend<Stdout>>;

const COPY_REQUEST_SENT_FEEDBACK: &str = "Copy request sent.";
const OUTPUT_UNMASK_ACK: &str = "unmask";
const RESPONSE_COPY_ACK: &str = "copy";
const RESPONSE_EXPORT_ACK: &str = "export";
const EXTERNAL_DEFINITION_REVIEW_NOTICE: &str = "Review this external definition before running it; atctl validates format, duplicate names, masking, and effective risk, but does not certify that it is appropriate for your device, SIM, network, or endpoint.";
const EXTERNAL_DEFINITION_CONFIRMATION_NOTICE: &str = "Review external definition before running.";

#[derive(Debug, Clone)]
struct TuiState {
    focus: Pane,
    show_help: bool,
    selected_category: usize,
    selected_command: usize,
    selected_control: usize,
    highlighted_device: usize,
    active_device: Option<usize>,
    highlighted_all_usb_device: usize,
    device_view: DeviceView,
    selected_log: usize,
    categories: Vec<String>,
    commands: Vec<ExecutableItem>,
    devices: Vec<UsbDeviceInfo>,
    all_usb_devices: Vec<UsbDeviceInfo>,
    log_paths: LoggingPaths,
    logs: Vec<LogEntry>,
    logs_error: Option<String>,
    response: ResponseState,
    response_cleared_at: Option<String>,
    response_scroll: usize,
    response_visible_height: usize,
    devices_visible_height: usize,
    categories_visible_height: usize,
    commands_visible_height: usize,
    controls_visible_height: usize,
    logs_visible_height: usize,
    status: String,
    status_role: TuiStyleRole,
    confirmation: Option<ConfirmationState>,
    sequence_input: Option<SequenceInputState>,
    sequence_candidate_sets: Vec<TuiSequenceCandidateSet>,
    output_masking_enabled: bool,
    normal_logging_enabled: bool,
    output_masking_ack_input: Option<String>,
    response_action_confirmation: Option<ResponseActionConfirmationState>,
    raw_log_path_input: Option<RawLogPathInputState>,
    raw_log_ack_input: Option<RawLogAckInputState>,
    raw_capture: Option<RawLogSink>,
    search_input: Option<SearchInputState>,
    search_query: String,
    edit_input: Option<EditInputState>,
    ad_hoc_input: Option<AdHocInputState>,
    timeout_input: Option<TimeoutInputState>,
    timeout_override_secs: Option<u64>,
    controls_feedback: Option<ControlsFeedback>,
    response_action_feedback: Option<ControlsFeedback>,
    action_menu: Option<ActionMenuState>,
    pending_execution: Option<PendingExecution>,
    running_execution: Option<RunningExecution>,
    active_command: Option<CommandStatus>,
    viewed_log: Option<ViewedLog>,
    exported_response: Option<ExportedResponse>,
    theme: TuiTheme,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Pane {
    Devices,
    Categories,
    Commands,
    Controls,
    Response,
    Status,
    History,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum DeviceView {
    OperationTargets,
    AllUsbTroubleshooting,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ControlAction {
    AdHocCommand,
    EditCommand,
    SetTimeout,
    RawExport,
    ToggleOutputMasking,
}

#[derive(Debug, Clone)]
struct ControlRow {
    action: ControlAction,
    label: String,
    inline_state: Option<String>,
    enabled: bool,
    unavailable_message: &'static str,
}

#[derive(Debug, Clone)]
struct ControlsFeedback {
    message: String,
    role: TuiStyleRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TuiAction {
    Continue,
    Quit,
    CopyToClipboard(String),
    ChooseResponseExportDirectory(ResponseExportRequest),
    RevealPath {
        path: PathBuf,
        origin: ActionMenuKind,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ActionMenuKind {
    Response,
    Log,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ActionMenuAction {
    CopyResponse,
    ExportResponse,
    ClearResponse,
    CopyDisplayedLog,
    RevealInFinder,
    CloseLogView,
    OpenLog,
}

#[derive(Debug, Clone)]
struct ActionMenuState {
    kind: ActionMenuKind,
    selected: usize,
    feedback: Option<ControlsFeedback>,
    feedback_scope: ActionMenuFeedbackScope,
    log_target: Option<LogEntry>,
    response_export: Option<ResponseExportRequest>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ActionMenuFeedbackScope {
    Action,
    ModalState,
}

#[derive(Debug, Clone)]
struct ActionMenuRow {
    action: ActionMenuAction,
    label: String,
    enabled: bool,
    unavailable_message: String,
}

#[derive(Debug, Clone)]
struct ConfirmationState {
    preset: Preset,
    input: String,
}

#[derive(Debug, Clone)]
struct SequenceInputState {
    sequence: Sequence,
    values: Vec<String>,
    active_param: usize,
    active_candidate: usize,
    confirmation_input: String,
    pending_candidate_action: Option<SequenceCandidateAction>,
    phase: SequenceInputPhase,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct TuiSequenceCandidateSet {
    candidate: SequenceCandidateSource,
    candidates: Vec<SequenceValueCandidate>,
    source_label: String,
    acquired_at: String,
}

impl TuiSequenceCandidateSet {
    fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    fn len(&self) -> usize {
        self.candidates.len()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum SequenceInputPhase {
    Params,
    CandidateActionConfirmation,
    Confirmation,
}

#[derive(Debug, Clone, Default)]
struct SearchInputState {
    input: String,
}

#[derive(Debug, Clone, Default)]
struct RawLogPathInputState {
    input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct RawLogAckInputState {
    path: PathBuf,
    input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct EditInputState {
    input: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct AdHocInputState {
    input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct TimeoutInputState {
    input: String,
    error: Option<String>,
}

impl ConfirmationState {
    fn new(preset: Preset) -> Self {
        Self {
            preset,
            input: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingExecution {
    item: ExecutableItem,
    confirmed: bool,
    timeout_secs: u64,
    device_selection: Option<TuiDeviceSelection>,
    sequence_params: Vec<SequenceParamValue>,
    normal_logging_enabled: bool,
}

#[derive(Debug, Clone)]
struct TuiSessionOptions {
    theme: TuiTheme,
    output_masking_enabled: bool,
    normal_logging_enabled: bool,
}

#[derive(Debug, Clone)]
struct RunningExecution {
    started_at: Instant,
    timeout: Duration,
}

#[derive(Debug)]
struct TuiExecutionResult {
    item: ExecutableItem,
    timeout_secs: u64,
    result: Result<ExecutionOutput>,
    raw_capture: Option<RawLogSink>,
}

#[derive(Debug, Clone)]
struct LogEntry {
    kind: LogListingKind,
    path: PathBuf,
    label: String,
}

#[derive(Debug, Clone)]
struct ViewedLog {
    kind: LogListingKind,
    path: PathBuf,
    label: String,
}

#[derive(Debug, Clone)]
struct ExportedResponse {
    path: PathBuf,
    response_label: String,
    finished_at: Option<String>,
    masked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResponseExportRequest {
    file_name: String,
    contents: String,
    response_label: String,
    finished_at: Option<String>,
    masked: bool,
}

#[derive(Debug, Clone)]
struct ResponseActionConfirmationState {
    action: ResponseActionConfirmation,
    input: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum ResponseActionConfirmation {
    Copy {
        contents: String,
        response_label: String,
    },
    Export {
        request: ResponseExportRequest,
        path: PathBuf,
    },
}

#[derive(Debug, Clone)]
struct CommandStatus {
    state: CommandRunState,
    kind: ExecutableKind,
    name: String,
    command: Option<String>,
    source_title: Option<String>,
    risk: RiskLevel,
    summary: StatusSummary,
    finished_at: Option<String>,
}

#[derive(Debug, Clone)]
enum StatusSummary {
    None,
    Completed { status: String, duration_ms: u128 },
    Failed,
}

#[derive(Debug)]
enum CommandListRow<'a> {
    BlankSeparator,
    KindHeader(&'static str),
    SourceHeader(String),
    Command {
        command_index: usize,
        command: &'a ExecutableItem,
    },
}

#[derive(Debug, Clone)]
enum ExecutableItem {
    Preset(Preset),
    Sequence(Sequence),
    CandidateAction {
        action: SequenceCandidateAction,
        preset: Preset,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ExecutableKind {
    Command,
    Sequence,
    CandidateAction,
}

#[derive(Debug, Clone)]
enum ExecutionOutput {
    Command(SendExecution),
    Sequence(SequenceExecution),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CommandRunState {
    Confirming,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ExecutableItem {
    fn kind(&self) -> ExecutableKind {
        match self {
            Self::Preset(_) => ExecutableKind::Command,
            Self::Sequence(_) => ExecutableKind::Sequence,
            Self::CandidateAction { .. } => ExecutableKind::CandidateAction,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Preset(preset) => &preset.name,
            Self::Sequence(sequence) => &sequence.name,
            Self::CandidateAction { action, .. } => action.label,
        }
    }

    fn categories(&self) -> &[String] {
        match self {
            Self::Preset(preset) => &preset.categories,
            Self::Sequence(sequence) => &sequence.categories,
            Self::CandidateAction { preset, .. } => &preset.categories,
        }
    }

    fn risk(&self) -> RiskLevel {
        match self {
            Self::Preset(preset) => preset.risk,
            Self::Sequence(sequence) => sequence.risk,
            Self::CandidateAction { preset, .. } => preset.risk,
        }
    }

    fn timeout_secs(&self) -> Option<u64> {
        match self {
            Self::Preset(preset) => preset.timeout_secs,
            Self::Sequence(sequence) => sequence.timeout_secs,
            Self::CandidateAction { preset, .. } => preset.timeout_secs,
        }
    }

    fn command_text(&self) -> Option<&str> {
        match self {
            Self::Preset(preset) => Some(&preset.command),
            Self::Sequence(_) => None,
            Self::CandidateAction { preset, .. } => Some(&preset.command),
        }
    }

    fn summary(&self) -> Option<&str> {
        match self {
            Self::Preset(_) => None,
            Self::Sequence(sequence) => Some(&sequence.summary),
            Self::CandidateAction { .. } => None,
        }
    }

    fn source_label(&self) -> &str {
        match self {
            Self::Preset(preset) => preset.origin.label(),
            Self::Sequence(sequence) => sequence.origin.label(),
            Self::CandidateAction { .. } => "candidate action",
        }
    }

    fn source_detail(&self) -> Option<&str> {
        match self {
            Self::Preset(preset) => preset.origin.detail(),
            Self::Sequence(sequence) => sequence.origin.detail(),
            Self::CandidateAction { .. } => None,
        }
    }

    fn source_file_path(&self) -> Option<&str> {
        match self {
            Self::Preset(preset) => preset.origin.file_path(),
            Self::Sequence(sequence) => sequence.origin.file_path(),
            Self::CandidateAction { .. } => None,
        }
    }

    fn sort_key(&self) -> (u8, u8, String) {
        match self {
            Self::Preset(preset) => {
                let (kind, label) = preset.origin.sort_key();
                (0, kind, label)
            }
            Self::Sequence(sequence) => {
                let (kind, label) = sequence.origin.sort_key();
                (1, kind, label)
            }
            Self::CandidateAction { action, .. } => (2, 0, action.label.to_owned()),
        }
    }

    fn as_preset(&self) -> Option<&Preset> {
        match self {
            Self::Preset(preset) => Some(preset),
            Self::Sequence(_) => None,
            Self::CandidateAction { preset, .. } => Some(preset),
        }
    }

    fn as_sequence(&self) -> Option<&Sequence> {
        match self {
            Self::Preset(_) => None,
            Self::Sequence(sequence) => Some(sequence),
            Self::CandidateAction { .. } => None,
        }
    }
}

impl ExecutableKind {
    fn noun(self) -> &'static str {
        match self {
            Self::Command => "Command",
            Self::Sequence => "Sequence",
            Self::CandidateAction => "Action",
        }
    }
}

impl CommandRunState {
    fn terminal_event_label(self) -> Option<&'static str> {
        match self {
            Self::Completed => Some("Completed"),
            Self::Failed => Some("Failed"),
            Self::Cancelled => Some("Cancelled"),
            Self::Confirming | Self::Running => None,
        }
    }
}

impl CommandStatus {
    fn new(
        state: CommandRunState,
        item: &ExecutableItem,
        _timeout_secs: u64,
        summary: StatusSummary,
    ) -> Self {
        Self {
            state,
            kind: item.kind(),
            name: item.name().to_owned(),
            command: item.command_text().map(str::to_owned),
            source_title: item.source_detail().map(str::to_owned),
            risk: item.risk(),
            summary,
            finished_at: None,
        }
    }

    fn with_finished_at(mut self, finished_at: String) -> Self {
        self.finished_at = Some(finished_at);
        self
    }

    fn state_label(&self) -> &'static str {
        match self.state {
            CommandRunState::Confirming => "confirming",
            CommandRunState::Running => "running",
            CommandRunState::Completed => "completed",
            CommandRunState::Failed => "failed",
            CommandRunState::Cancelled => "cancelled",
        }
    }

    fn target_status_label(&self) -> &'static str {
        self.kind.noun()
    }

    fn terminal_event_label(&self) -> Option<&'static str> {
        self.state.terminal_event_label()
    }

    fn status_summary_line(&self) -> Option<String> {
        match &self.summary {
            StatusSummary::None => None,
            StatusSummary::Completed {
                status,
                duration_ms,
            } if self.kind == ExecutableKind::CandidateAction => {
                Some(format!("Action result: {status} {duration_ms}ms"))
            }
            StatusSummary::Completed {
                status,
                duration_ms,
            } => Some(format!("Result: {status} {duration_ms}ms")),
            StatusSummary::Failed if self.kind == ExecutableKind::CandidateAction => {
                Some("Action result: failed".to_owned())
            }
            StatusSummary::Failed => Some("Result: failed".to_owned()),
        }
    }
}

trait TuiCommandExecutor {
    fn execute_item(
        &mut self,
        pending: &PendingExecution,
        raw_log: Option<&mut RawLogSink>,
    ) -> Result<ExecutionOutput>;
}

struct UsbTuiCommandExecutor;

impl TuiCommandExecutor for UsbTuiCommandExecutor {
    fn execute_item(
        &mut self,
        pending: &PendingExecution,
        raw_log: Option<&mut RawLogSink>,
    ) -> Result<ExecutionOutput> {
        match &pending.item {
            ExecutableItem::Preset(preset) => execute_tui_preset(
                preset,
                pending.confirmed,
                pending.timeout_secs,
                pending.device_selection,
                pending.normal_logging_enabled,
            )
            .map(ExecutionOutput::Command),
            ExecutableItem::CandidateAction { preset, .. } => execute_tui_preset(
                preset,
                pending.confirmed,
                pending.timeout_secs,
                pending.device_selection,
                pending.normal_logging_enabled,
            )
            .map(ExecutionOutput::Command),
            ExecutableItem::Sequence(sequence) => execute_tui_sequence(
                sequence,
                &pending.sequence_params,
                pending.confirmed,
                pending.timeout_secs,
                pending.device_selection,
                pending.normal_logging_enabled,
                raw_log,
            )
            .map(ExecutionOutput::Sequence),
        }
    }
}

pub fn run(
    theme_choice: Option<TuiThemeChoice>,
    no_mask: bool,
    no_log: bool,
    preset_locations: PresetFileLocationOptions,
    sequence_locations: SequenceFileLocationOptions,
) -> Result<()> {
    let commands = load_tui_presets(&preset_locations)?;
    let sequences = load_tui_sequences(&sequence_locations)?;
    let devices = load_tui_devices()?;
    let log_paths = logging_paths()?;
    let logs = log_entries_from_paths(&log_paths)?;
    let mut state = TuiState::new_with_all_usb_and_masking_and_log_paths(
        executable_items(commands, sequences),
        devices.targets,
        devices.all_usb,
        logs,
        log_paths,
        TuiSessionOptions {
            theme: TuiTheme::from_choice(theme_choice),
            output_masking_enabled: !no_mask,
            normal_logging_enabled: !no_log,
        },
    );
    let (execution_tx, execution_rx) = mpsc::channel();
    let mut terminal = TerminalSession::enter()?;

    loop {
        apply_finished_executions(&mut state, &execution_rx);
        terminal.draw(&mut state)?;
        if event::poll(Duration::from_millis(100)).map_err(terminal_error)? {
            let Event::Key(key) = event::read().map_err(terminal_error)? else {
                continue;
            };
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match handle_key_code(&mut state, key.code) {
                TuiAction::Continue => {}
                TuiAction::Quit => break,
                TuiAction::CopyToClipboard(text) => {
                    finish_clipboard_copy(&mut state, terminal.copy_to_clipboard(&text));
                }
                TuiAction::ChooseResponseExportDirectory(request) => {
                    let result = terminal.choose_response_export_directory();
                    finish_response_export(&mut state, request, result);
                }
                TuiAction::RevealPath { path, origin } => {
                    let result = terminal.reveal_path(&path);
                    finish_path_reveal(&mut state, &path, origin, result);
                }
            }
            if state.pending_execution.is_some() {
                start_pending_execution(&mut state, &execution_tx);
            }
        }
    }

    Ok(())
}

fn load_tui_presets(preset_locations: &PresetFileLocationOptions) -> Result<Vec<Preset>> {
    load_presets(preset_locations)
}

fn load_tui_sequences(sequence_locations: &SequenceFileLocationOptions) -> Result<Vec<Sequence>> {
    load_sequences(sequence_locations)
}

fn executable_items(presets: Vec<Preset>, sequences: Vec<Sequence>) -> Vec<ExecutableItem> {
    presets
        .into_iter()
        .map(ExecutableItem::Preset)
        .chain(sequences.into_iter().map(ExecutableItem::Sequence))
        .collect()
}

#[derive(Debug)]
struct TuiDeviceInventory {
    targets: Vec<UsbDeviceInfo>,
    all_usb: Vec<UsbDeviceInfo>,
}

fn load_tui_devices() -> Result<TuiDeviceInventory> {
    let filter = tui_device_filter()?;
    Ok(TuiDeviceInventory {
        targets: list_devices(&filter, UsbDeviceListMode::AtTargets)?,
        all_usb: list_devices(&filter, UsbDeviceListMode::AllUsb)?,
    })
}

fn log_entries_from_paths(paths: &LoggingPaths) -> Result<Vec<LogEntry>> {
    let logs = list_logs(&paths.state_dir, &paths.session_dir)?;
    Ok(logs
        .into_iter()
        .map(|log| {
            let kind = match log.kind {
                LogListingKind::History => "history",
                LogListingKind::Session => "session",
            };
            let label = format!("{kind}: {}", compact_path_label(&log.path));
            LogEntry {
                kind: log.kind,
                path: log.path,
                label,
            }
        })
        .collect())
}

fn refresh_log_summaries(state: &mut TuiState) -> Result<()> {
    let paths = state.log_paths.clone();
    refresh_log_summaries_from_paths(state, &paths)
}

fn refresh_log_summaries_from_paths(state: &mut TuiState, paths: &LoggingPaths) -> Result<()> {
    let logs = log_entries_from_paths(paths)?;
    state.logs = logs;
    state.logs_error = None;
    clamp_selected_log(state);
    Ok(())
}

fn refresh_log_summaries_after_execution(state: &mut TuiState) {
    refresh_log_summaries_or_record_error(state);
}

fn refresh_log_summaries_or_record_error(state: &mut TuiState) {
    if let Err(error) = refresh_log_summaries(state) {
        state.logs_error = Some(format!("Refresh failed: {error}"));
    }
}

fn clamp_selected_log(state: &mut TuiState) {
    if state.logs.is_empty() {
        state.selected_log = 0;
    } else if state.selected_log >= state.logs.len() {
        state.selected_log = state.logs.len() - 1;
    }
}

fn find_log_entry_index(logs: &[LogEntry], target: &LogEntry) -> Option<usize> {
    logs.iter()
        .position(|log| log.kind == target.kind && log.path == target.path)
}

fn compact_path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

struct TerminalSession {
    terminal: CrosstermTerminal,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().map_err(terminal_error)?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide).map_err(terminal_error)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).map_err(terminal_error)?;
        Ok(Self { terminal })
    }

    fn draw(&mut self, state: &mut TuiState) -> Result<()> {
        self.terminal
            .draw(|frame| render_frame(frame, state))
            .map(|_| ())
            .map_err(terminal_error)
    }

    fn copy_to_clipboard(&mut self, text: &str) -> Result<()> {
        let sequence = osc52_clipboard_sequence(text);
        self.terminal
            .backend_mut()
            .write_all(sequence.as_bytes())
            .map_err(terminal_error)?;
        self.terminal.backend_mut().flush().map_err(terminal_error)
    }

    fn choose_response_export_directory(&mut self) -> Result<Option<PathBuf>> {
        choose_response_export_directory()
    }

    fn reveal_path(&mut self, path: &Path) -> Result<()> {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| {
                AtctlError::Transport(format!(
                    "failed to reveal {} in Finder: {error}",
                    path.display()
                ))
            })
    }
}

#[cfg(target_os = "macos")]
fn choose_response_export_directory() -> Result<Option<PathBuf>> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "set selectedFolder to choose folder with prompt \"Choose a folder for the exported Response\"",
            "-e",
            "return POSIX path of selectedFolder",
        ])
        .output()
        .map_err(|error| {
            AtctlError::Transport(format!("failed to open Response export folder chooser: {error}"))
        })?;

    if output.status.success() {
        let directory = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if directory.is_empty() {
            return Err(AtctlError::Transport(
                "Response export folder chooser returned no folder".to_owned(),
            ));
        }
        return Ok(Some(PathBuf::from(directory)));
    }

    let error = String::from_utf8_lossy(&output.stderr);
    if error.contains("-128") || error.contains("User canceled") {
        Ok(None)
    } else {
        Err(AtctlError::Transport(format!(
            "Response export folder chooser failed: {}",
            error.trim()
        )))
    }
}

#[cfg(not(target_os = "macos"))]
fn choose_response_export_directory() -> Result<Option<PathBuf>> {
    Err(AtctlError::NotImplemented(
        "Response export folder chooser is currently available on macOS",
    ))
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), Show, LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

impl TuiState {
    #[cfg(test)]
    fn new(
        commands: Vec<Preset>,
        devices: Vec<UsbDeviceInfo>,
        logs: Vec<LogEntry>,
        theme: TuiTheme,
    ) -> Self {
        Self::new_with_all_usb(
            executable_items(commands, Vec::new()),
            devices.clone(),
            devices,
            logs,
            theme,
        )
    }

    #[cfg(test)]
    fn new_with_all_usb(
        commands: Vec<ExecutableItem>,
        devices: Vec<UsbDeviceInfo>,
        all_usb_devices: Vec<UsbDeviceInfo>,
        logs: Vec<LogEntry>,
        theme: TuiTheme,
    ) -> Self {
        Self::new_with_all_usb_and_masking(commands, devices, all_usb_devices, logs, theme, true)
    }

    #[cfg(test)]
    fn new_with_all_usb_and_masking(
        commands: Vec<ExecutableItem>,
        devices: Vec<UsbDeviceInfo>,
        all_usb_devices: Vec<UsbDeviceInfo>,
        logs: Vec<LogEntry>,
        theme: TuiTheme,
        output_masking_enabled: bool,
    ) -> Self {
        let state_dir = default_state_dir().expect("test state directory should resolve");
        let log_paths = LoggingPaths {
            session_dir: state_dir.join("logs"),
            state_dir,
        };
        Self::new_with_all_usb_and_masking_and_log_paths(
            commands,
            devices,
            all_usb_devices,
            logs,
            log_paths,
            TuiSessionOptions {
                theme,
                output_masking_enabled,
                normal_logging_enabled: true,
            },
        )
    }

    fn new_with_all_usb_and_masking_and_log_paths(
        commands: Vec<ExecutableItem>,
        devices: Vec<UsbDeviceInfo>,
        all_usb_devices: Vec<UsbDeviceInfo>,
        logs: Vec<LogEntry>,
        log_paths: LoggingPaths,
        options: TuiSessionOptions,
    ) -> Self {
        let commands = order_tui_commands(commands);
        let categories = categories_from_commands(&commands);
        let response = initial_response_state(&commands);
        let active_device = if devices.len() == 1 { Some(0) } else { None };
        let focus = if devices.len() > 1 {
            Pane::Devices
        } else {
            Pane::Commands
        };
        let status = match devices.len() {
            0 => "No matching USB device is visible.".to_owned(),
            1 => "Ready.".to_owned(),
            _ => "Select a USB device before sending.".to_owned(),
        };
        Self {
            focus,
            show_help: false,
            selected_category: 0,
            selected_command: 0,
            selected_control: 0,
            highlighted_device: 0,
            active_device,
            highlighted_all_usb_device: 0,
            device_view: DeviceView::OperationTargets,
            selected_log: 0,
            categories,
            commands,
            devices,
            all_usb_devices,
            log_paths,
            logs,
            logs_error: None,
            response,
            response_cleared_at: None,
            response_scroll: 0,
            response_visible_height: 1,
            devices_visible_height: 6,
            categories_visible_height: 6,
            commands_visible_height: 6,
            controls_visible_height: 6,
            logs_visible_height: 6,
            status,
            status_role: TuiStyleRole::Status,
            confirmation: None,
            sequence_input: None,
            sequence_candidate_sets: Vec::new(),
            output_masking_enabled: options.output_masking_enabled,
            normal_logging_enabled: options.normal_logging_enabled,
            output_masking_ack_input: None,
            response_action_confirmation: None,
            raw_log_path_input: None,
            raw_log_ack_input: None,
            raw_capture: None,
            search_input: None,
            search_query: String::new(),
            edit_input: None,
            ad_hoc_input: None,
            timeout_input: None,
            timeout_override_secs: None,
            controls_feedback: None,
            response_action_feedback: None,
            action_menu: None,
            pending_execution: None,
            running_execution: None,
            active_command: None,
            viewed_log: None,
            exported_response: None,
            theme: options.theme,
        }
    }

    fn selected_command(&self) -> Option<&ExecutableItem> {
        self.visible_commands().get(self.selected_command).copied()
    }

    fn selected_log(&self) -> Option<&LogEntry> {
        self.logs.get(self.selected_log)
    }

    fn active_device(&self) -> Option<&UsbDeviceInfo> {
        self.active_device.and_then(|index| self.devices.get(index))
    }

    fn target_index_for_device(&self, device: &UsbDeviceInfo) -> Option<usize> {
        self.devices
            .iter()
            .position(|target| same_runtime_usb_device(target, device))
    }

    fn visible_commands(&self) -> Vec<&ExecutableItem> {
        let search_query = self.search_query.trim();
        let Some(category) = self.categories.get(self.selected_category) else {
            return self
                .commands
                .iter()
                .filter(|command| command_matches_search(command, search_query))
                .collect();
        };

        if category == "all" {
            return self
                .commands
                .iter()
                .filter(|command| command_matches_search(command, search_query))
                .collect();
        }

        self.commands
            .iter()
            .filter(|command| {
                command
                    .categories()
                    .iter()
                    .any(|command_category| command_category == category)
            })
            .filter(|command| command_matches_search(command, search_query))
            .collect()
    }
}

fn categories_from_commands(commands: &[ExecutableItem]) -> Vec<String> {
    let mut categories = BTreeSet::from(["all".to_owned()]);
    for command in commands {
        categories.extend(
            command
                .categories()
                .iter()
                .filter(|category| is_preset_category(category))
                .cloned(),
        );
    }
    categories.into_iter().collect()
}

fn order_tui_commands(mut commands: Vec<ExecutableItem>) -> Vec<ExecutableItem> {
    commands.sort_by_key(|command| command.sort_key());
    commands
}

fn initial_response_state(commands: &[ExecutableItem]) -> ResponseState {
    let mut sources = BTreeSet::new();
    for command in commands {
        if let (Some(label), Some(path)) = (command.source_detail(), command.source_file_path()) {
            sources.insert((label.to_owned(), path.to_owned()));
        }
    }

    if sources.is_empty() {
        return ResponseState::masked(
            "Select a command. Enter runs safe/read commands and opens confirmation for write-risk commands.",
        );
    }

    let mut lines = vec![
        "External definitions loaded for this TUI session.".to_owned(),
        "Loading them does not send AT commands by itself, but running loaded items may change modem state, send SMS, or transmit network payloads.".to_owned(),
        EXTERNAL_DEFINITION_REVIEW_NOTICE.to_owned(),
        String::new(),
        "Loaded sources:".to_owned(),
    ];
    for (label, path) in sources {
        lines.push(format!("- {label}: {path}"));
    }
    ResponseState::masked(lines.join("\n"))
}

fn is_preset_category(category: &str) -> bool {
    category != "all"
}

fn command_matches_search(command: &ExecutableItem, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query = query.to_ascii_lowercase();
    command.name().to_ascii_lowercase().contains(&query)
        || command
            .command_text()
            .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
        || command
            .summary()
            .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
        || command.source_label().to_ascii_lowercase().contains(&query)
        || command
            .categories()
            .iter()
            .any(|category| category.to_ascii_lowercase().contains(&query))
}

fn has_file_preset_sets(state: &TuiState) -> bool {
    state.commands.iter().any(
        |command| matches!(command, ExecutableItem::Preset(preset) if !preset.origin.is_built_in()),
    )
}

fn has_file_sequence_sets(state: &TuiState) -> bool {
    state
        .commands
        .iter()
        .any(|command| matches!(command, ExecutableItem::Sequence(sequence) if !sequence.origin.is_built_in()))
}

fn should_show_preset_source_detail(origin: &PresetOrigin) -> bool {
    origin.detail().is_some()
}

fn should_show_sequence_source_detail(origin: &SequenceOrigin) -> bool {
    origin.detail().is_some()
}

fn effective_timeout_secs(state: &TuiState, command: &ExecutableItem) -> u64 {
    state
        .timeout_override_secs
        .or(command.timeout_secs())
        .unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS)
}

fn output_masking_state_label(state: &TuiState) -> &'static str {
    if state.output_masking_enabled {
        "on"
    } else {
        "off"
    }
}

fn should_show_output_masking_context(state: &TuiState) -> bool {
    state.viewed_log.is_none() && (!state.output_masking_enabled || state.response.has_raw_text())
}

fn selected_device_for_execution(state: &TuiState) -> Option<TuiDeviceSelection> {
    state.active_device().map(|device| TuiDeviceSelection {
        vendor_id: device.vendor_id,
        product_id: device.product_id,
        bus: device.bus,
        address: device.address,
    })
}

fn same_runtime_usb_device(left: &UsbDeviceInfo, right: &UsbDeviceInfo) -> bool {
    left.bus == right.bus
        && left.address == right.address
        && left.vendor_id == right.vendor_id
        && left.product_id == right.product_id
}

fn device_gate_message(state: &TuiState) -> Option<&'static str> {
    if state.active_device().is_some() {
        None
    } else if state.devices.is_empty() {
        Some("No matching USB device is visible.")
    } else {
        Some("Select a USB device before sending.")
    }
}

fn block_device_dependent_action(state: &mut TuiState) {
    let message = device_gate_message(state).unwrap_or("Select a USB device before sending.");
    state.status = message.to_owned();
    state.status_role = TuiStyleRole::Warning;
    let recovery = if state.devices.is_empty() {
        "\nReview Devices and all USB. After changing the USB connection, restart `atctl tui` to rescan."
    } else {
        ""
    };
    set_response(
        state,
        ResponseState::masked(format!(
            "{message}\nDevice-dependent actions are disabled until a USB device is selected.{recovery}"
        )),
    );
    if !state.devices.is_empty() {
        state.focus = Pane::Devices;
    } else if !state.all_usb_devices.is_empty() {
        state.focus = Pane::Devices;
        state.device_view = DeviceView::AllUsbTroubleshooting;
    }
}

fn select_highlighted_device(state: &mut TuiState) {
    if state.device_view == DeviceView::AllUsbTroubleshooting {
        select_highlighted_all_usb_device(state);
        return;
    }

    if state.highlighted_device >= state.devices.len() && !state.all_usb_devices.is_empty() {
        toggle_device_view(state);
        state.highlighted_all_usb_device = 0;
        return;
    }

    if state.devices.is_empty() {
        state.active_device = None;
        state.status = "No matching USB device is visible.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return;
    }

    state.highlighted_device = state.highlighted_device.min(state.devices.len() - 1);
    state.active_device = Some(state.highlighted_device);
    let detail = state
        .active_device()
        .map(device_display_label)
        .unwrap_or_else(|| "USB device".to_owned());
    state.status = format!("Selected USB device: {detail}.");
    state.status_role = TuiStyleRole::Status;
    state.focus = Pane::Commands;
}

fn select_highlighted_all_usb_device(state: &mut TuiState) {
    if state.highlighted_all_usb_device >= state.all_usb_devices.len() {
        toggle_device_view(state);
        state.highlighted_device = state.highlighted_device.min(state.devices.len());
        return;
    }

    if state.all_usb_devices.is_empty() {
        state.status = "No USB devices are visible through libusb.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return;
    }

    state.highlighted_all_usb_device = state
        .highlighted_all_usb_device
        .min(state.all_usb_devices.len() - 1);
    let device = &state.all_usb_devices[state.highlighted_all_usb_device];
    if let Some(target_index) = state.target_index_for_device(device) {
        state.highlighted_device = target_index;
        state.active_device = Some(target_index);
        state.device_view = DeviceView::OperationTargets;
        let detail = state
            .active_device()
            .map(device_display_label)
            .unwrap_or_else(|| "USB device".to_owned());
        state.status = format!("Selected USB device: {detail}.");
        state.status_role = TuiStyleRole::Status;
        state.focus = Pane::Commands;
    } else {
        state.active_device = None;
        state.status = "Diagnostic-only USB device; AT sending is disabled.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        set_response(
            state,
            ResponseState::masked(format!(
                "Diagnostic-only USB device\n{}\n{}:{} bus={} address={}\n\nThis item is visible through libusb but is not an atctl operation target.",
                device_display_label(device),
                hex_u16(device.vendor_id),
                hex_u16(device.product_id),
                device.bus,
                device.address
            )),
        );
    }
}

fn toggle_device_view(state: &mut TuiState) {
    state.focus = Pane::Devices;
    state.device_view = match state.device_view {
        DeviceView::OperationTargets => {
            state.status = "Showing all USB devices for troubleshooting.".to_owned();
            DeviceView::AllUsbTroubleshooting
        }
        DeviceView::AllUsbTroubleshooting => {
            state.status = "Showing atctl operation targets.".to_owned();
            DeviceView::OperationTargets
        }
    };
    state.status_role = TuiStyleRole::Status;
}

fn handle_key_code(state: &mut TuiState, key: KeyCode) -> TuiAction {
    if state.show_help {
        return handle_help_key(state, key);
    }

    if state.action_menu.is_some() {
        return handle_action_menu_key(state, key);
    }

    if state.output_masking_ack_input.is_some() {
        return handle_output_masking_confirmation_key(state, key);
    }

    if state.response_action_confirmation.is_some() {
        return handle_response_action_confirmation_key(state, key);
    }

    if state.raw_log_path_input.is_some() {
        return handle_raw_log_path_input_key(state, key);
    }

    if state.raw_log_ack_input.is_some() {
        return handle_raw_log_ack_input_key(state, key);
    }

    if state.timeout_input.is_some() {
        return handle_timeout_input_key(state, key);
    }

    if state.search_input.is_some() {
        return handle_search_input_key(state, key);
    }

    if state.edit_input.is_some() {
        return handle_edit_input_key(state, key);
    }

    if state.sequence_input.is_some() {
        return handle_sequence_input_key(state, key);
    }

    if state.confirmation.is_some() {
        return handle_confirmation_key(state, key);
    }

    if state.ad_hoc_input.is_some() {
        return handle_ad_hoc_input_key(state, key);
    }

    if state.running_execution.is_some() && is_blocked_while_running(key) {
        state.status = "Command is running; wait for completion or timeout.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return TuiAction::Continue;
    }

    match key {
        KeyCode::Char('q') => TuiAction::Quit,
        KeyCode::Char('?') => {
            state.show_help = true;
            TuiAction::Continue
        }
        KeyCode::Left => {
            state.focus = state.focus.previous();
            TuiAction::Continue
        }
        KeyCode::Right | KeyCode::Tab => {
            state.focus = state.focus.next();
            TuiAction::Continue
        }
        KeyCode::Up => {
            if state.focus == Pane::Response {
                scroll_response(state, -1);
            } else {
                move_selection(state, -1);
            }
            TuiAction::Continue
        }
        KeyCode::Down => {
            if state.focus == Pane::Response {
                scroll_response(state, 1);
            } else {
                move_selection(state, 1);
            }
            TuiAction::Continue
        }
        KeyCode::PageUp => {
            if state.focus == Pane::Response {
                scroll_response(state, -(state.response_visible_height.max(1) as isize));
            } else {
                move_selection(state, -(focused_page_size(state) as isize));
            }
            TuiAction::Continue
        }
        KeyCode::PageDown => {
            if state.focus == Pane::Response {
                scroll_response(state, state.response_visible_height.max(1) as isize);
            } else {
                move_selection(state, focused_page_size(state) as isize);
            }
            TuiAction::Continue
        }
        KeyCode::Home => {
            if state.focus == Pane::Response {
                state.response_scroll = 0;
            } else {
                move_selection_to_boundary(state, ListBoundary::Start);
            }
            TuiAction::Continue
        }
        KeyCode::End => {
            if state.focus == Pane::Response {
                state.response_scroll = response_max_scroll(state);
            } else {
                move_selection_to_boundary(state, ListBoundary::End);
            }
            TuiAction::Continue
        }
        KeyCode::Enter => {
            match state.focus {
                Pane::Devices => select_highlighted_device(state),
                Pane::Categories => {
                    state.focus = Pane::Commands;
                    state.status = "Commands / Sequences focused.".to_owned();
                    state.status_role = TuiStyleRole::Status;
                }
                Pane::Commands => {
                    if let Some(command) = state.selected_command().cloned() {
                        begin_item_execution(state, command, "Confirmation required.");
                    }
                }
                Pane::Controls => return execute_selected_control(state),
                Pane::Response => open_response_action_menu(state),
                Pane::History => open_log_action_menu(state),
                Pane::Status => {}
            }
            TuiAction::Continue
        }
        KeyCode::Char('/') => {
            state.search_input = Some(SearchInputState {
                input: state.search_query.clone(),
            });
            state.focus = Pane::Commands;
            state.status = "Command search input.".to_owned();
            state.status_role = TuiStyleRole::Status;
            TuiAction::Continue
        }
        _ => TuiAction::Continue,
    }
}

fn is_blocked_while_running(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('/')
    )
}

fn handle_help_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            state.show_help = false;
            TuiAction::Continue
        }
        _ => TuiAction::Continue,
    }
}

fn execute_selected_control(state: &mut TuiState) -> TuiAction {
    let rows = control_rows(state);
    let Some(row) = rows.get(state.selected_control) else {
        return TuiAction::Continue;
    };

    if !row.enabled {
        set_controls_feedback(state, TuiStyleRole::Warning, row.unavailable_message);
        return TuiAction::Continue;
    }

    match row.action {
        ControlAction::AdHocCommand => open_ad_hoc_input(state),
        ControlAction::EditCommand => open_edit_input(state),
        ControlAction::SetTimeout => open_timeout_input(state),
        ControlAction::RawExport => {
            toggle_raw_capture(state);
            TuiAction::Continue
        }
        ControlAction::ToggleOutputMasking => toggle_output_masking(state),
    }
}

fn set_controls_feedback(state: &mut TuiState, role: TuiStyleRole, message: impl Into<String>) {
    state.controls_feedback = Some(ControlsFeedback {
        message: message.into(),
        role,
    });
}

fn set_action_menu_feedback(state: &mut TuiState, role: TuiStyleRole, message: impl Into<String>) {
    if let Some(menu) = state.action_menu.as_mut() {
        menu.feedback = Some(ControlsFeedback {
            message: message.into(),
            role,
        });
        menu.feedback_scope = ActionMenuFeedbackScope::Action;
    }
}

fn clear_controls_feedback(state: &mut TuiState) {
    state.controls_feedback = None;
}

fn open_response_action_menu(state: &mut TuiState) {
    let response_export = response_export_request(state);
    state.action_menu = Some(ActionMenuState {
        kind: ActionMenuKind::Response,
        selected: 0,
        feedback: None,
        feedback_scope: ActionMenuFeedbackScope::Action,
        log_target: None,
        response_export,
    });
    state.status = "Response actions.".to_owned();
    state.status_role = TuiStyleRole::Status;
}

fn open_log_action_menu(state: &mut TuiState) {
    let requested_log = state.selected_log().cloned();
    let mut log_target = requested_log.clone();
    let mut feedback = None;

    if let Err(error) = refresh_log_summaries(state) {
        state.logs_error = Some(format!("Refresh failed: {error}"));
    } else if let Some(requested) = requested_log {
        if let Some(index) = find_log_entry_index(&state.logs, &requested) {
            state.selected_log = index;
            log_target = Some(state.logs[index].clone());
        } else {
            log_target = None;
            feedback = Some(ControlsFeedback {
                message: format!(
                    "Selected log no longer exists: {}. Logs list refreshed.",
                    requested.label
                ),
                role: TuiStyleRole::Warning,
            });
        }
    } else {
        log_target = state.selected_log().cloned();
    }

    if log_target.is_none() && feedback.is_none() {
        state.status = "No logs are available.".to_owned();
        state.status_role = TuiStyleRole::Status;
        return;
    }

    state.action_menu = Some(ActionMenuState {
        kind: ActionMenuKind::Log,
        selected: 0,
        feedback_scope: if feedback.is_some() {
            ActionMenuFeedbackScope::ModalState
        } else {
            ActionMenuFeedbackScope::Action
        },
        feedback,
        log_target,
        response_export: None,
    });
    state.status = "Log actions.".to_owned();
    state.status_role = TuiStyleRole::Status;
}

fn handle_action_menu_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.action_menu = None;
            state.status = "Action menu closed.".to_owned();
            state.status_role = TuiStyleRole::Status;
            TuiAction::Continue
        }
        KeyCode::Up => {
            move_action_menu_selection(state, -1);
            TuiAction::Continue
        }
        KeyCode::Down => {
            move_action_menu_selection(state, 1);
            TuiAction::Continue
        }
        KeyCode::Home => {
            if let Some(menu) = state.action_menu.as_mut() {
                menu.selected = 0;
                clear_transient_action_menu_feedback(menu);
            }
            TuiAction::Continue
        }
        KeyCode::End => {
            let Some(kind) = state.action_menu.as_ref().map(|menu| menu.kind) else {
                return TuiAction::Continue;
            };
            let len = action_menu_rows(state, kind).len();
            if let Some(menu) = state.action_menu.as_mut() {
                menu.selected = len.saturating_sub(1);
                clear_transient_action_menu_feedback(menu);
            }
            TuiAction::Continue
        }
        KeyCode::Enter => execute_selected_action_menu_row(state),
        _ => TuiAction::Continue,
    }
}

fn move_action_menu_selection(state: &mut TuiState, delta: isize) {
    let Some(kind) = state.action_menu.as_ref().map(|menu| menu.kind) else {
        return;
    };
    let len = action_menu_rows(state, kind).len();
    if let Some(menu) = state.action_menu.as_mut() {
        menu.selected = move_index(menu.selected, len, delta);
        clear_transient_action_menu_feedback(menu);
    }
}

fn clear_transient_action_menu_feedback(menu: &mut ActionMenuState) {
    if menu.feedback_scope == ActionMenuFeedbackScope::Action {
        menu.feedback = None;
    }
}

fn execute_selected_action_menu_row(state: &mut TuiState) -> TuiAction {
    let Some(kind) = state.action_menu.as_ref().map(|menu| menu.kind) else {
        return TuiAction::Continue;
    };
    let rows = action_menu_rows(state, kind);
    let selected = state
        .action_menu
        .as_ref()
        .map(|menu| menu.selected)
        .unwrap_or(0)
        .min(rows.len().saturating_sub(1));
    let Some(row) = rows.get(selected) else {
        return TuiAction::Continue;
    };

    if !row.enabled {
        set_action_menu_feedback(
            state,
            TuiStyleRole::Warning,
            row.unavailable_message.clone(),
        );
        return TuiAction::Continue;
    }

    if kind == ActionMenuKind::Response {
        clear_response_action_feedback(state);
    }

    match row.action {
        ActionMenuAction::CopyDisplayedLog => {
            state.action_menu = None;
            copy_current_response(state)
        }
        ActionMenuAction::CopyResponse => {
            state.action_menu = None;
            if response_has_unmasked_content(state) {
                open_unmasked_response_copy_confirmation(state)
            } else {
                copy_current_response(state)
            }
        }
        ActionMenuAction::ExportResponse => {
            let request = state
                .action_menu
                .as_ref()
                .and_then(|menu| menu.response_export.clone());
            state.action_menu = None;
            match request {
                Some(request) => TuiAction::ChooseResponseExportDirectory(request),
                None => {
                    set_response_action_result(
                        state,
                        TuiStyleRole::Warning,
                        "Response export unavailable.",
                        "No response is available to export.",
                    );
                    TuiAction::Continue
                }
            }
        }
        ActionMenuAction::ClearResponse => {
            state.action_menu = None;
            clear_response(state);
            TuiAction::Continue
        }
        ActionMenuAction::RevealInFinder => {
            let (path, missing_message) = reveal_action_target(state, kind);
            state.action_menu = None;
            reveal_file(state, path, missing_message, kind)
        }
        ActionMenuAction::CloseLogView => {
            state.action_menu = None;
            close_log_view(state);
            TuiAction::Continue
        }
        ActionMenuAction::OpenLog => {
            let log = state
                .action_menu
                .as_ref()
                .and_then(|menu| menu.log_target.clone());
            state.action_menu = None;
            open_log_entry(state, log);
            TuiAction::Continue
        }
    }
}

fn open_ad_hoc_input(state: &mut TuiState) -> TuiAction {
    if device_gate_message(state).is_some() {
        block_device_dependent_action(state);
        return TuiAction::Continue;
    }
    state.ad_hoc_input = Some(AdHocInputState::default());
    state.status = "AT command input.".to_owned();
    state.status_role = TuiStyleRole::Status;
    TuiAction::Continue
}

fn open_edit_input(state: &mut TuiState) -> TuiAction {
    if device_gate_message(state).is_some() {
        block_device_dependent_action(state);
        return TuiAction::Continue;
    }
    let Some(command) = state.selected_command().cloned() else {
        state.status = "No command is selected for editing.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return TuiAction::Continue;
    };
    if let ExecutableItem::Sequence(sequence) = command {
        open_sequence_input(state, sequence);
        return TuiAction::Continue;
    }
    let Some(preset) = command.as_preset() else {
        return TuiAction::Continue;
    };
    state.edit_input = Some(EditInputState {
        input: preset.command.clone(),
        error: None,
    });
    state.status = "Edit command before execution.".to_owned();
    state.status_role = TuiStyleRole::Status;
    TuiAction::Continue
}

fn open_timeout_input(state: &mut TuiState) -> TuiAction {
    let timeout_secs = state
        .selected_command()
        .map(|command| effective_timeout_secs(state, command))
        .unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECS);
    state.timeout_input = Some(TimeoutInputState {
        input: timeout_secs.to_string(),
        error: None,
    });
    state.status = "Timeout input.".to_owned();
    state.status_role = TuiStyleRole::Status;
    TuiAction::Continue
}

fn open_sequence_input(state: &mut TuiState, sequence: Sequence) {
    if device_gate_message(state).is_some() {
        block_device_dependent_action(state);
        return;
    }
    clear_controls_feedback(state);
    let phase = if sequence.params.is_empty() {
        SequenceInputPhase::Confirmation
    } else {
        SequenceInputPhase::Params
    };
    state.sequence_input = Some(SequenceInputState {
        values: sequence_default_input_values(&sequence),
        sequence,
        active_param: 0,
        active_candidate: 0,
        confirmation_input: String::new(),
        pending_candidate_action: None,
        phase,
        error: None,
    });
    state.status = "Run Sequence input.".to_owned();
    state.status_role = TuiStyleRole::Status;
}

fn sequence_default_input_values(sequence: &Sequence) -> Vec<String> {
    sequence
        .params
        .iter()
        .map(|param| param.default_value.clone().unwrap_or_default())
        .collect()
}

fn copy_current_response(state: &mut TuiState) -> TuiAction {
    match copyable_response_text(state) {
        Some(text) => TuiAction::CopyToClipboard(text),
        None => {
            set_response_action_result(
                state,
                TuiStyleRole::Warning,
                "Response copy unavailable.",
                "No response is available to copy.",
            );
            TuiAction::Continue
        }
    }
}

fn open_unmasked_response_copy_confirmation(state: &mut TuiState) -> TuiAction {
    let Some(contents) = copyable_response_text(state) else {
        set_response_action_result(
            state,
            TuiStyleRole::Warning,
            "Response copy unavailable.",
            "No response is available to copy.",
        );
        return TuiAction::Continue;
    };
    state.response_action_confirmation = Some(ResponseActionConfirmationState {
        action: ResponseActionConfirmation::Copy {
            contents,
            response_label: response_export_target_label(state),
        },
        input: String::new(),
        error: None,
    });
    state.status = "Unmasked response copy confirmation required.".to_owned();
    state.status_role = TuiStyleRole::Warning;
    TuiAction::Continue
}

fn close_log_view(state: &mut TuiState) {
    state.response.clear();
    state.response_scroll = 0;
    state.viewed_log = None;
    state.exported_response = None;
    clear_response_action_feedback(state);
    state.status = "Log view closed.".to_owned();
    state.status_role = TuiStyleRole::Status;
}

fn finish_clipboard_copy(state: &mut TuiState, result: Result<()>) {
    match result {
        Ok(()) => {
            set_response_action_result(
                state,
                TuiStyleRole::Status,
                "Response copy requested.",
                COPY_REQUEST_SENT_FEEDBACK,
            );
        }
        Err(error) => {
            set_response_action_result(
                state,
                TuiStyleRole::Error,
                "Response copy failed.",
                format!("Copy request failed: {error}"),
            );
        }
    }
}

fn finish_response_export(
    state: &mut TuiState,
    request: ResponseExportRequest,
    directory_result: Result<Option<PathBuf>>,
) {
    let directory = match directory_result {
        Ok(Some(directory)) => directory,
        Ok(None) => {
            set_response_action_result(
                state,
                TuiStyleRole::Status,
                "Response export cancelled.",
                "Response export cancelled.",
            );
            return;
        }
        Err(error) => {
            set_response_action_result(
                state,
                TuiStyleRole::Error,
                "Response export failed.",
                format!(
                    "Choose an existing folder and export again. Folder selection failed: {error}"
                ),
            );
            return;
        }
    };

    if !directory.is_dir() {
        set_response_action_result(
            state,
            TuiStyleRole::Error,
            "Response export failed.",
            format!(
                "Choose an existing folder and export again. Destination folder does not exist: {}.",
                compact_path_label(&directory)
            ),
        );
        return;
    }

    let path = directory.join(&request.file_name);
    if !request.masked {
        state.response_action_confirmation = Some(ResponseActionConfirmationState {
            action: ResponseActionConfirmation::Export { request, path },
            input: String::new(),
            error: None,
        });
        state.status = "Unmasked response export confirmation required.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return;
    }

    write_response_export_request(state, request, path);
}

fn write_response_export_request(
    state: &mut TuiState,
    request: ResponseExportRequest,
    path: PathBuf,
) {
    match write_response_export(&path, &request.contents) {
        Ok(()) => {
            state.exported_response = Some(ExportedResponse {
                path: path.clone(),
                response_label: request.response_label,
                finished_at: request.finished_at,
                masked: request.masked,
            });
            set_response_action_result(
                state,
                TuiStyleRole::Status,
                "Response export completed.",
                format!("Exported response: {}.", compact_path_label(&path)),
            );
        }
        Err(AtctlError::ResponseExportFileExists { path }) => {
            set_response_action_result(
                state,
                TuiStyleRole::Error,
                "Response export failed.",
                format!(
                    "Choose another folder and export again. File already exists: {}.",
                    compact_path_label(Path::new(&path))
                ),
            );
        }
        Err(AtctlError::ResponseExportParentUnavailable { path }) => {
            set_response_action_result(
                state,
                TuiStyleRole::Error,
                "Response export failed.",
                format!(
                    "Choose an existing folder and export again. Destination folder does not exist: {}.",
                    compact_path_label(Path::new(&path))
                ),
            );
        }
        Err(error) => {
            set_response_action_result(
                state,
                TuiStyleRole::Error,
                "Response export failed.",
                format!("Response export failed: {error}"),
            );
        }
    }
}

fn reveal_action_target(state: &TuiState, kind: ActionMenuKind) -> (Option<PathBuf>, &'static str) {
    match kind {
        ActionMenuKind::Log => (
            selected_action_log(state).map(|log| log.path.clone()),
            "Saved log no longer exists.",
        ),
        ActionMenuKind::Response if state.viewed_log.is_some() => (
            state.viewed_log.as_ref().map(|log| log.path.clone()),
            "Saved log no longer exists.",
        ),
        ActionMenuKind::Response => (
            state
                .exported_response
                .as_ref()
                .map(|response| response.path.clone()),
            "Exported response no longer exists.",
        ),
    }
}

fn reveal_file(
    state: &mut TuiState,
    path: Option<PathBuf>,
    missing_message: &'static str,
    origin: ActionMenuKind,
) -> TuiAction {
    let Some(path) = path else {
        set_action_result(
            state,
            origin,
            TuiStyleRole::Warning,
            "Reveal unavailable.",
            missing_message,
        );
        return TuiAction::Continue;
    };
    if !path.is_file() {
        set_action_result(
            state,
            origin,
            TuiStyleRole::Warning,
            "Reveal unavailable.",
            missing_message,
        );
        return TuiAction::Continue;
    }
    TuiAction::RevealPath { path, origin }
}

fn finish_path_reveal(
    state: &mut TuiState,
    path: &Path,
    origin: ActionMenuKind,
    result: Result<()>,
) {
    match result {
        Ok(()) => {
            set_action_result(
                state,
                origin,
                TuiStyleRole::Status,
                "Reveal request sent.",
                format!(
                    "Reveal in Finder request sent: {}.",
                    compact_path_label(path)
                ),
            );
        }
        Err(error) => {
            set_action_result(
                state,
                origin,
                TuiStyleRole::Error,
                "Reveal request failed.",
                format!("Reveal in Finder request failed: {error}"),
            );
        }
    }
}

fn set_action_result(
    state: &mut TuiState,
    origin: ActionMenuKind,
    role: TuiStyleRole,
    status: impl Into<String>,
    message: impl Into<String>,
) {
    if origin == ActionMenuKind::Response {
        set_response_action_result(state, role, status, message);
    } else {
        state.status = message.into();
        state.status_role = role;
    }
}

fn set_response_action_result(
    state: &mut TuiState,
    role: TuiStyleRole,
    status: impl Into<String>,
    message: impl Into<String>,
) {
    state.status = status.into();
    state.status_role = role;
    state.response_action_feedback = Some(ControlsFeedback {
        message: message.into(),
        role,
    });
}

fn clear_response_action_feedback(state: &mut TuiState) {
    state.response_action_feedback = None;
}

fn set_response(state: &mut TuiState, response: ResponseState) {
    state.response = response;
    state.response_cleared_at = None;
    state.response_scroll = 0;
    state.exported_response = None;
    clear_response_action_feedback(state);
}

fn toggle_output_masking(state: &mut TuiState) -> TuiAction {
    if state.output_masking_enabled {
        state.output_masking_ack_input = Some(String::new());
        state.status = "Output masking confirmation required.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        set_controls_feedback(state, TuiStyleRole::Warning, "Confirm output masking off.");
    } else {
        state.output_masking_enabled = true;
        state.output_masking_ack_input = None;
        state.response_scroll = 0;
        state.status = "Output masking enabled.".to_owned();
        state.status_role = TuiStyleRole::Status;
        set_controls_feedback(state, TuiStyleRole::Status, "Output masking on.");
    }
    TuiAction::Continue
}

fn clear_response(state: &mut TuiState) {
    state.response.clear();
    state.response_cleared_at = Some(now_timestamp().display().to_owned());
    state.response_scroll = 0;
    state.exported_response = None;
    set_response_action_result(
        state,
        TuiStyleRole::Status,
        "Response body cleared.",
        "Response body cleared.",
    );
}

fn handle_output_masking_confirmation_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.output_masking_ack_input = None;
            state.status = "Output masking unchanged.".to_owned();
            state.status_role = TuiStyleRole::Status;
            set_controls_feedback(state, TuiStyleRole::Status, "Output masking unchanged.");
        }
        KeyCode::Enter => {
            let input = state.output_masking_ack_input.take().unwrap_or_default();
            if input.trim() == OUTPUT_UNMASK_ACK {
                state.output_masking_enabled = false;
                state.response_scroll = 0;
                state.status = "Output masking disabled for this TUI session.".to_owned();
                state.status_role = TuiStyleRole::Warning;
                set_controls_feedback(state, TuiStyleRole::Warning, "Output masking off.");
            } else {
                state.status = "Output masking confirmation rejected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                state.response_scroll = 0;
                set_controls_feedback(
                    state,
                    TuiStyleRole::Error,
                    "Output masking confirmation rejected.",
                );
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = state.output_masking_ack_input.as_mut() {
                input.pop();
            }
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.output_masking_ack_input.as_mut() {
                input.push(value);
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_response_action_confirmation_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => {
            let action = state.response_action_confirmation.take();
            let message = match action.map(|confirmation| confirmation.action) {
                Some(ResponseActionConfirmation::Copy { .. }) => "Response copy cancelled.",
                Some(ResponseActionConfirmation::Export { .. }) => "Response export cancelled.",
                None => "Response action cancelled.",
            };
            set_response_action_result(state, TuiStyleRole::Status, message, message);
        }
        KeyCode::Enter => {
            let Some(mut confirmation) = state.response_action_confirmation.take() else {
                return TuiAction::Continue;
            };
            let expected = response_action_confirmation_ack(&confirmation.action);
            if confirmation.input.trim() != expected {
                confirmation.error = Some(format!("Type `{expected}` exactly."));
                state.response_action_confirmation = Some(confirmation);
                state.status = "Unmasked response confirmation rejected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                return TuiAction::Continue;
            }

            match confirmation.action {
                ResponseActionConfirmation::Copy { contents, .. } => {
                    state.status = "Unmasked response copy confirmed.".to_owned();
                    state.status_role = TuiStyleRole::Warning;
                    return TuiAction::CopyToClipboard(contents);
                }
                ResponseActionConfirmation::Export { request, path } => {
                    write_response_export_request(state, request, path);
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(confirmation) = state.response_action_confirmation.as_mut() {
                confirmation.input.pop();
                confirmation.error = None;
            }
        }
        KeyCode::Char(value) => {
            if let Some(confirmation) = state.response_action_confirmation.as_mut() {
                confirmation.input.push(value);
                confirmation.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn response_action_confirmation_ack(action: &ResponseActionConfirmation) -> &'static str {
    match action {
        ResponseActionConfirmation::Copy { .. } => RESPONSE_COPY_ACK,
        ResponseActionConfirmation::Export { .. } => RESPONSE_EXPORT_ACK,
    }
}

fn handle_raw_log_path_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            state.raw_log_path_input = None;
            state.status = "Raw diagnostic export cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
            set_controls_feedback(state, TuiStyleRole::Status, "Raw export cancelled.");
        }
        KeyCode::Char('q')
            if state
                .raw_log_path_input
                .as_ref()
                .is_some_and(|input| input.input.is_empty()) =>
        {
            state.raw_log_path_input = None;
            state.status = "Raw diagnostic export cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
            set_controls_feedback(state, TuiStyleRole::Status, "Raw export cancelled.");
        }
        KeyCode::Enter => {
            let Some(input_state) = state.raw_log_path_input.as_mut() else {
                return TuiAction::Continue;
            };
            let value = input_state.input.trim();
            if value.is_empty() {
                input_state.error = Some("Enter a raw export file path.".to_owned());
                state.status = "Raw diagnostic export path is required.".to_owned();
                state.status_role = TuiStyleRole::Error;
                set_controls_feedback(state, TuiStyleRole::Error, "Raw export path required.");
                return TuiAction::Continue;
            }
            let path = PathBuf::from(value);
            if path.exists() {
                input_state.error = Some("File already exists; choose a new path.".to_owned());
                state.status = "Raw diagnostic export refuses to overwrite files.".to_owned();
                state.status_role = TuiStyleRole::Error;
                set_controls_feedback(
                    state,
                    TuiStyleRole::Error,
                    "Raw export file already exists.",
                );
                return TuiAction::Continue;
            }
            state.raw_log_path_input = None;
            state.raw_log_ack_input = Some(RawLogAckInputState {
                path,
                input: String::new(),
                error: None,
            });
            state.status = "Raw diagnostic export acknowledgement required.".to_owned();
            state.status_role = TuiStyleRole::Warning;
        }
        KeyCode::Backspace => {
            if let Some(input) = state.raw_log_path_input.as_mut() {
                input.input.pop();
                input.error = None;
            }
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.raw_log_path_input.as_mut() {
                input.input.push(value);
                input.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_raw_log_ack_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            state.raw_log_ack_input = None;
            state.status = "Raw diagnostic export cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
            set_controls_feedback(state, TuiStyleRole::Status, "Raw export cancelled.");
        }
        KeyCode::Char('q')
            if state
                .raw_log_ack_input
                .as_ref()
                .is_some_and(|input| input.input.is_empty()) =>
        {
            state.raw_log_ack_input = None;
            state.status = "Raw diagnostic export cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
            set_controls_feedback(state, TuiStyleRole::Status, "Raw export cancelled.");
        }
        KeyCode::Enter => {
            let Some(input_state) = state.raw_log_ack_input.take() else {
                return TuiAction::Continue;
            };
            if input_state.input.trim() != RAW_LOG_ACK {
                state.raw_log_ack_input = Some(RawLogAckInputState {
                    error: Some(format!("Type `{RAW_LOG_ACK}` exactly.")),
                    ..input_state
                });
                state.status = "Raw diagnostic export acknowledgement rejected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                set_controls_feedback(
                    state,
                    TuiStyleRole::Error,
                    "Raw export acknowledgement rejected.",
                );
                return TuiAction::Continue;
            }

            match RawLogSink::create(RawLogConfig::new(input_state.path.clone(), "tui", "tui")) {
                Ok(sink) => {
                    state.raw_capture = Some(sink);
                    state.status = format!(
                        "Raw diagnostic export started: {}.",
                        compact_path_label(&input_state.path)
                    );
                    state.status_role = TuiStyleRole::Warning;
                    set_controls_feedback(state, TuiStyleRole::Warning, "Raw export started.");
                }
                Err(error) => {
                    state.raw_capture = None;
                    state.status = format!("Raw diagnostic export failed: {error}");
                    state.status_role = TuiStyleRole::Error;
                    set_controls_feedback(
                        state,
                        TuiStyleRole::Error,
                        format!("Raw export failed: {error}"),
                    );
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = state.raw_log_ack_input.as_mut() {
                input.input.pop();
                input.error = None;
            }
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.raw_log_ack_input.as_mut() {
                input.input.push(value);
                input.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_search_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            state.search_input = None;
            state.status = "Command search cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Enter => {
            let Some(input_state) = state.search_input.take() else {
                return TuiAction::Continue;
            };
            state.search_query = input_state.input.trim().to_owned();
            state.selected_command = 0;
            state.focus = Pane::Commands;
            if state.search_query.is_empty() {
                state.status = "Command search cleared.".to_owned();
            } else {
                let count = state.visible_commands().len();
                state.status = format!("Command search `{}` matched {count}.", state.search_query);
            }
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Backspace => {
            if let Some(input) = state.search_input.as_mut() {
                input.input.pop();
            }
        }
        KeyCode::Char('q')
            if state
                .search_input
                .as_ref()
                .is_some_and(|input| input.input.is_empty()) =>
        {
            state.search_input = None;
            state.status = "Command search cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.search_input.as_mut() {
                input.input.push(value);
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_edit_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            state.edit_input = None;
            state.status = "Edit-before-run cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Enter => {
            let Some(input_state) = state.edit_input.take() else {
                return TuiAction::Continue;
            };
            let command = input_state.input.trim();
            match validate_ad_hoc_command(command) {
                Ok(()) => {
                    let preset = one_shot_preset("edited", "edited", "edited", command);
                    begin_item_execution(
                        state,
                        ExecutableItem::Preset(preset),
                        "Confirmation required for edited command.",
                    );
                }
                Err(message) => {
                    state.edit_input = Some(EditInputState {
                        input: input_state.input,
                        error: Some(message.clone()),
                    });
                    state.status = message;
                    state.status_role = TuiStyleRole::Error;
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = state.edit_input.as_mut() {
                input.input.pop();
                input.error = None;
            }
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.edit_input.as_mut() {
                input.input.push(value);
                input.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_timeout_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            state.timeout_input = None;
            state.status = "Timeout change cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Enter => {
            let Some(input_state) = state.timeout_input.take() else {
                return TuiAction::Continue;
            };
            let input = input_state.input.trim();
            if input.eq_ignore_ascii_case("default") {
                state.timeout_override_secs = None;
                state.status = "Timeout override cleared.".to_owned();
                state.status_role = TuiStyleRole::Status;
            } else {
                match input.parse::<u64>() {
                    Ok(value) if value > 0 => {
                        state.timeout_override_secs = Some(value);
                        state.status = format!("Timeout override set to {value}s.");
                        state.status_role = TuiStyleRole::Status;
                    }
                    _ => {
                        state.timeout_input = Some(TimeoutInputState {
                            input: input_state.input,
                            error: Some("Enter seconds greater than 0 or `default`.".to_owned()),
                        });
                        state.status = "Invalid timeout value.".to_owned();
                        state.status_role = TuiStyleRole::Error;
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = state.timeout_input.as_mut() {
                input.input.pop();
                input.error = None;
            }
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.timeout_input.as_mut() {
                input.input.push(value);
                input.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_confirmation_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            if let Some(confirmation) = &state.confirmation {
                let item = ExecutableItem::Preset(confirmation.preset.clone());
                let timeout_secs = effective_timeout_secs(state, &item);
                state.active_command = Some(CommandStatus::new(
                    CommandRunState::Cancelled,
                    &item,
                    timeout_secs,
                    StatusSummary::None,
                ));
            }
            state.confirmation = None;
            state.status = "Command cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
            set_response(state, ResponseState::masked("Command was not sent."));
        }
        KeyCode::Enter => {
            let Some(confirmation) = state.confirmation.take() else {
                return TuiAction::Continue;
            };
            let expected = confirmation.preset.risk.to_string();
            if confirmation.input.trim() == expected {
                let item = ExecutableItem::Preset(confirmation.preset);
                let timeout_secs = effective_timeout_secs(state, &item);
                schedule_item_execution(state, item, true, timeout_secs, Vec::new());
            } else {
                let item = ExecutableItem::Preset(confirmation.preset);
                state.status = "Confirmation rejected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                state.active_command = Some(CommandStatus::new(
                    CommandRunState::Cancelled,
                    &item,
                    effective_timeout_secs(state, &item),
                    StatusSummary::None,
                ));
                set_response(
                    state,
                    ResponseState::masked(format!(
                        "Command was not sent.\nRisk confirmation did not match `{expected}`."
                    )),
                );
            }
        }
        KeyCode::Backspace => {
            if let Some(confirmation) = state.confirmation.as_mut() {
                confirmation.input.pop();
            }
        }
        KeyCode::Char(value) => {
            if let Some(confirmation) = state.confirmation.as_mut() {
                confirmation.input.push(value);
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn handle_sequence_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            cancel_sequence_input(state, "Sequence run cancelled.");
        }
        KeyCode::Char('q')
            if state.sequence_input.as_ref().is_some_and(|input| {
                let active_value_empty = input
                    .values
                    .get(input.active_param)
                    .is_none_or(|value| value.is_empty());
                input.phase == SequenceInputPhase::Params
                    && active_value_empty
                    && input.confirmation_input.is_empty()
            }) =>
        {
            cancel_sequence_input(state, "Sequence run cancelled.");
        }
        KeyCode::Enter => {
            if !select_active_sequence_candidate(state) {
                submit_sequence_input(state);
            }
        }
        KeyCode::Tab if matches_sequence_phase(state, SequenceInputPhase::Params) => {
            if let Some(input) = state.sequence_input.as_mut() {
                move_to_next_sequence_param(input);
                input.error = None;
            }
        }
        KeyCode::Down if matches_sequence_phase(state, SequenceInputPhase::Params) => {
            let candidate_count = active_sequence_candidate_count(state);
            if let Some(input) = state.sequence_input.as_mut() {
                if candidate_count > 0 {
                    input.active_candidate =
                        (input.active_candidate + 1).min(candidate_count.saturating_sub(1));
                } else {
                    move_to_next_sequence_param(input);
                }
                input.error = None;
            }
        }
        KeyCode::Up if matches_sequence_phase(state, SequenceInputPhase::Params) => {
            let candidate_count = active_sequence_candidate_count(state);
            if let Some(input) = state.sequence_input.as_mut() {
                if candidate_count > 0 {
                    input.active_candidate = input.active_candidate.saturating_sub(1);
                } else {
                    input.active_param = input.active_param.saturating_sub(1);
                    input.active_candidate = 0;
                }
                input.error = None;
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = state.sequence_input.as_mut() {
                match input.phase {
                    SequenceInputPhase::Params => {
                        if let Some(value) = input.values.get_mut(input.active_param) {
                            value.pop();
                        }
                        input.active_candidate = 0;
                    }
                    SequenceInputPhase::CandidateActionConfirmation => {
                        input.confirmation_input.pop();
                    }
                    SequenceInputPhase::Confirmation => {
                        input.confirmation_input.pop();
                    }
                }
                input.error = None;
            }
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.sequence_input.as_mut() {
                match input.phase {
                    SequenceInputPhase::Params => {
                        if let Some(current) = input.values.get_mut(input.active_param) {
                            current.push(value);
                        }
                        input.active_candidate = 0;
                    }
                    SequenceInputPhase::CandidateActionConfirmation => {
                        input.confirmation_input.push(value);
                    }
                    SequenceInputPhase::Confirmation => {
                        input.confirmation_input.push(value);
                    }
                }
                input.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn matches_sequence_phase(state: &TuiState, phase: SequenceInputPhase) -> bool {
    state
        .sequence_input
        .as_ref()
        .is_some_and(|input| input.phase == phase)
}

fn move_to_next_sequence_param(input: &mut SequenceInputState) {
    input.active_param = (input.active_param + 1).min(input.values.len().saturating_sub(1));
    input.active_candidate = 0;
}

fn active_sequence_candidate_count(state: &TuiState) -> usize {
    let Some(candidate) = active_sequence_candidate_source(state) else {
        return 0;
    };
    let action_count = candidate_actions(candidate).len();
    if let Some(candidate_set) = candidate_set_for_source(state, candidate)
        && !candidate_set.is_empty()
    {
        return candidate_set.len() + action_count;
    }
    if action_count == 0 {
        0
    } else {
        action_count + 1
    }
}

fn select_active_sequence_candidate(state: &mut TuiState) -> bool {
    let Some(input) = &state.sequence_input else {
        return false;
    };
    if input.phase != SequenceInputPhase::Params {
        return false;
    }
    let Some(candidate) = active_sequence_candidate_source(state) else {
        return false;
    };
    if let Some(candidate_set) = candidate_set_for_source(state, candidate).cloned()
        && !candidate_set.is_empty()
    {
        if state
            .sequence_input
            .as_ref()
            .is_some_and(|input| input.active_candidate < candidate_set.len())
        {
            return select_active_sequence_value_candidate(state, &candidate_set);
        }
        return run_active_sequence_candidate_action(state, candidate, candidate_set.len());
    }
    run_active_sequence_candidate_action(state, candidate, 1)
}

fn select_active_sequence_value_candidate(
    state: &mut TuiState,
    candidate_set: &TuiSequenceCandidateSet,
) -> bool {
    let Some(input) = &state.sequence_input else {
        return false;
    };
    let candidate = candidate_set
        .candidates
        .get(
            input
                .active_candidate
                .min(candidate_set.len().saturating_sub(1)),
        )
        .cloned();
    let Some(candidate) = candidate else {
        return false;
    };
    let current = input
        .values
        .get(input.active_param)
        .map(|value| value.trim())
        .unwrap_or("");
    if current == candidate.value {
        return false;
    }
    if let Some(input) = state.sequence_input.as_mut()
        && let Some(value) = input.values.get_mut(input.active_param)
    {
        *value = candidate.value;
        input.error = None;
        state.status = "Sequence candidate selected.".to_owned();
        state.status_role = TuiStyleRole::Status;
        return true;
    }
    false
}

fn run_active_sequence_candidate_action(
    state: &mut TuiState,
    candidate: SequenceCandidateSource,
    action_start_index: usize,
) -> bool {
    let Some(input) = &state.sequence_input else {
        return false;
    };
    if input.active_candidate < action_start_index {
        return false;
    }
    let action_index = input.active_candidate - action_start_index;
    let Some(action) = candidate_actions(candidate).get(action_index).cloned() else {
        return false;
    };
    let item = candidate_action_item(action);
    if item.risk().requires_confirmation() {
        if let Some(input) = state.sequence_input.as_mut() {
            input.phase = SequenceInputPhase::CandidateActionConfirmation;
            input.pending_candidate_action = Some(action);
            input.confirmation_input.clear();
            input.error = None;
        }
        state.status = "Action confirmation required.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return true;
    }
    schedule_item_execution(state, item, false, DEFAULT_COMMAND_TIMEOUT_SECS, Vec::new());
    true
}

fn candidate_action_item(action: SequenceCandidateAction) -> ExecutableItem {
    let preset = Preset::new(
        action.label,
        action.command,
        classify_direct_command(action.command).risk,
        action
            .categories
            .iter()
            .map(|category| (*category).to_owned())
            .collect(),
        PresetOrigin::runtime("candidate action"),
    );
    ExecutableItem::CandidateAction { action, preset }
}

fn cancel_sequence_input(state: &mut TuiState, message: &str) {
    let cancelled = state.sequence_input.take();
    state.status = message.to_owned();
    state.status_role = TuiStyleRole::Status;
    set_response(state, ResponseState::masked("Sequence was not run."));
    if let Some(input) = cancelled {
        let item = ExecutableItem::Sequence(input.sequence);
        state.active_command = Some(CommandStatus::new(
            CommandRunState::Cancelled,
            &item,
            effective_timeout_secs(state, &item),
            StatusSummary::None,
        ));
    }
}

fn submit_sequence_input(state: &mut TuiState) {
    let Some(mut input) = state.sequence_input.take() else {
        return;
    };

    match input.phase {
        SequenceInputPhase::Params => {
            if let Err(message) = validate_active_sequence_param(&input) {
                input.error = Some(message.clone());
                state.status = message;
                state.status_role = TuiStyleRole::Error;
                state.sequence_input = Some(input);
                return;
            }
            if input.active_param + 1 < input.sequence.params.len() {
                input.active_param += 1;
                input.active_candidate = 0;
                input.error = None;
                state.status = "Sequence value input.".to_owned();
                state.status_role = TuiStyleRole::Status;
                state.sequence_input = Some(input);
                return;
            }
            match sequence_input_param_values(&input) {
                Ok(params) => {
                    if input.sequence.risk.requires_confirmation() {
                        input.phase = SequenceInputPhase::Confirmation;
                        input.error = None;
                        state.status = "Sequence confirmation required.".to_owned();
                        state.status_role = TuiStyleRole::Warning;
                        state.sequence_input = Some(input);
                    } else {
                        let item = ExecutableItem::Sequence(input.sequence);
                        let timeout_secs = effective_timeout_secs(state, &item);
                        schedule_item_execution(state, item, false, timeout_secs, params);
                    }
                }
                Err(message) => {
                    input.error = Some(message.clone());
                    state.status = message;
                    state.status_role = TuiStyleRole::Error;
                    state.sequence_input = Some(input);
                }
            }
        }
        SequenceInputPhase::CandidateActionConfirmation => {
            let Some(action) = input.pending_candidate_action else {
                input.phase = SequenceInputPhase::Params;
                input.error = Some("No candidate action is selected.".to_owned());
                state.status = "No candidate action selected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                state.sequence_input = Some(input);
                return;
            };
            let item = candidate_action_item(action);
            let expected = item.risk().to_string();
            if item.risk().requires_confirmation() && input.confirmation_input.trim() != expected {
                input.error = Some(format!("Type `{expected}` exactly."));
                state.status = "Action confirmation rejected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                state.sequence_input = Some(input);
                return;
            }
            input.phase = SequenceInputPhase::Params;
            input.pending_candidate_action = None;
            input.confirmation_input.clear();
            input.error = None;
            state.sequence_input = Some(input);
            schedule_item_execution(state, item, true, DEFAULT_COMMAND_TIMEOUT_SECS, Vec::new());
        }
        SequenceInputPhase::Confirmation => {
            let expected = input.sequence.risk.to_string();
            if input.sequence.risk.requires_confirmation()
                && input.confirmation_input.trim() != expected
            {
                input.error = Some(format!("Type `{expected}` exactly."));
                state.status = "Sequence confirmation rejected.".to_owned();
                state.status_role = TuiStyleRole::Error;
                state.sequence_input = Some(input);
                return;
            }
            match sequence_input_param_values(&input) {
                Ok(params) => {
                    let item = ExecutableItem::Sequence(input.sequence);
                    let timeout_secs = effective_timeout_secs(state, &item);
                    schedule_item_execution(state, item, true, timeout_secs, params);
                }
                Err(message) => {
                    input.phase = SequenceInputPhase::Params;
                    input.error = Some(message.clone());
                    state.status = message;
                    state.status_role = TuiStyleRole::Error;
                    state.sequence_input = Some(input);
                }
            }
        }
    }
}

fn validate_active_sequence_param(input: &SequenceInputState) -> std::result::Result<(), String> {
    let Some(param) = input.sequence.params.get(input.active_param) else {
        return Ok(());
    };
    let value = input
        .values
        .get(input.active_param)
        .map(String::as_str)
        .unwrap_or("");
    if param.required && value.trim().is_empty() {
        Err(format_missing_sequence_param(param))
    } else {
        Ok(())
    }
}

fn sequence_input_param_values(
    input: &SequenceInputState,
) -> std::result::Result<Vec<SequenceParamValue>, String> {
    let mut values = Vec::new();
    for (index, param) in input.sequence.params.iter().enumerate() {
        let value = input.values.get(index).cloned().unwrap_or_default();
        if param.required && value.trim().is_empty() {
            return Err(format_missing_sequence_param(param));
        }
        values.push(SequenceParamValue {
            name: param.name.clone(),
            value,
        });
    }
    Ok(values)
}

fn handle_ad_hoc_input_key(state: &mut TuiState, key: KeyCode) -> TuiAction {
    match key {
        KeyCode::Esc => {
            state.ad_hoc_input = None;
            state.status = "AT command input cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Enter => {
            let Some(input_state) = state.ad_hoc_input.take() else {
                return TuiAction::Continue;
            };
            let command = input_state.input.trim();
            match validate_ad_hoc_command(command) {
                Ok(()) => {
                    let preset = Preset::ad_hoc(command);
                    begin_item_execution(
                        state,
                        ExecutableItem::Preset(preset),
                        "Confirmation required for AT command.",
                    );
                }
                Err(message) => {
                    state.ad_hoc_input = Some(AdHocInputState {
                        input: input_state.input,
                        error: Some(message.clone()),
                    });
                    state.status = message;
                    state.status_role = TuiStyleRole::Error;
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(input) = state.ad_hoc_input.as_mut() {
                input.input.pop();
                input.error = None;
            }
        }
        KeyCode::Char('q')
            if state
                .ad_hoc_input
                .as_ref()
                .is_some_and(|input| input.input.is_empty()) =>
        {
            state.ad_hoc_input = None;
            state.status = "AT command input cancelled.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        KeyCode::Char(value) => {
            if let Some(input) = state.ad_hoc_input.as_mut() {
                input.input.push(value);
                input.error = None;
            }
        }
        _ => {}
    }

    TuiAction::Continue
}

fn validate_ad_hoc_command(command: &str) -> std::result::Result<(), String> {
    if command.is_empty() {
        return Err("AT command is empty.".to_owned());
    }
    if command.chars().any(|ch| ch.is_control()) {
        return Err("AT command cannot contain control characters.".to_owned());
    }
    if !normalize_command(command).starts_with("AT") {
        return Err("AT command must start with AT.".to_owned());
    }
    if is_prompt_required_command(command) {
        return Err(
            "Prompt-required SMS/multi-step commands are not supported by one-shot AT command input."
                .to_owned(),
        );
    }
    Ok(())
}

fn one_shot_preset(name: &str, preset_set: &str, category: &str, command: &str) -> Preset {
    Preset::new(
        name,
        command,
        classify_direct_command(command).risk,
        vec![category.to_owned()],
        PresetOrigin::runtime(preset_set),
    )
}

fn begin_item_execution(state: &mut TuiState, command: ExecutableItem, confirmation_status: &str) {
    clear_controls_feedback(state);
    if device_gate_message(state).is_some() {
        block_device_dependent_action(state);
        return;
    }

    let timeout_secs = effective_timeout_secs(state, &command);
    if let ExecutableItem::Sequence(sequence) = &command {
        if !sequence.params.is_empty() || sequence.risk.requires_confirmation() {
            open_sequence_input(state, sequence.clone());
        } else {
            schedule_item_execution(state, command, false, timeout_secs, Vec::new());
        }
        return;
    }

    let Some(preset) = command.as_preset() else {
        state.status = "Selected item is not a runnable command.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return;
    };
    if preset.risk.requires_confirmation() {
        set_response(
            state,
            ResponseState::masked(format_confirmation_summary(preset)),
        );
        state.status = confirmation_status.to_owned();
        state.status_role = TuiStyleRole::Warning;
        state.active_command = Some(CommandStatus::new(
            CommandRunState::Confirming,
            &command,
            timeout_secs,
            StatusSummary::None,
        ));
        state.confirmation = Some(ConfirmationState::new(preset.clone()));
    } else {
        schedule_item_execution(state, command, false, timeout_secs, Vec::new());
    }
}

fn schedule_item_execution(
    state: &mut TuiState,
    command: ExecutableItem,
    confirmed: bool,
    timeout_secs: u64,
    sequence_params: Vec<SequenceParamValue>,
) {
    clear_controls_feedback(state);
    if device_gate_message(state).is_some() {
        block_device_dependent_action(state);
        return;
    }

    set_response(
        state,
        ResponseState::masked("Waiting for modem response..."),
    );
    state.output_masking_ack_input = None;
    state.ad_hoc_input = None;
    state.viewed_log = None;
    state.status = match command.kind() {
        ExecutableKind::Command => "Running command.".to_owned(),
        ExecutableKind::Sequence => "Running Sequence.".to_owned(),
        ExecutableKind::CandidateAction => "Running action.".to_owned(),
    };
    state.status_role = TuiStyleRole::Status;
    state.active_command = Some(CommandStatus::new(
        CommandRunState::Running,
        &command,
        timeout_secs,
        StatusSummary::None,
    ));
    let device_selection = selected_device_for_execution(state);
    state.pending_execution = Some(PendingExecution {
        item: command,
        confirmed,
        timeout_secs,
        device_selection,
        sequence_params,
        normal_logging_enabled: state.normal_logging_enabled,
    });
}

fn response_export_target_label(state: &TuiState) -> String {
    state
        .active_command
        .as_ref()
        .map(|command| {
            let target = if command.kind == ExecutableKind::CandidateAction {
                "Candidate action"
            } else {
                command.target_status_label()
            };
            format!("{target}: {}", command.name)
        })
        .unwrap_or_else(|| "Initial notice".to_owned())
}

fn toggle_raw_capture(state: &mut TuiState) {
    if let Some(capture) = state.raw_capture.take() {
        state.status = format!(
            "Raw diagnostic export stopped: {}.",
            compact_path_label(capture.path())
        );
        state.status_role = TuiStyleRole::Status;
        set_controls_feedback(state, TuiStyleRole::Status, "Raw export stopped.");
        return;
    }

    state.raw_log_path_input = Some(RawLogPathInputState::default());
    state.raw_log_ack_input = None;
    state.status = "Raw diagnostic export path input.".to_owned();
    state.status_role = TuiStyleRole::Warning;
    set_controls_feedback(state, TuiStyleRole::Warning, "Choose raw export file.");
}

fn response_export_request(state: &TuiState) -> Option<ResponseExportRequest> {
    if state.viewed_log.is_some() {
        return None;
    }

    let text = copyable_response_text(state)?;
    let timestamp = now_timestamp();
    let response_label = response_export_target_label(state);
    let path = response_export_path(Path::new(""), &response_label, timestamp.file_stem());
    let file_name = path.file_name()?.to_str()?.to_owned();
    Some(ResponseExportRequest {
        file_name,
        contents: format!("{}\n", text.trim_end_matches(['\r', '\n'])),
        response_label,
        finished_at: state
            .active_command
            .as_ref()
            .and_then(|command| command.finished_at.clone()),
        masked: !response_has_unmasked_content(state),
    })
}

fn response_has_unmasked_content(state: &TuiState) -> bool {
    state.viewed_log.is_none()
        && !state.output_masking_enabled
        && copyable_response_text_for_masking(state, false)
            != copyable_response_text_for_masking(state, true)
}

fn start_pending_execution(state: &mut TuiState, sender: &Sender<TuiExecutionResult>) {
    if state.running_execution.is_some() {
        state.status = "Command is already running.".to_owned();
        state.status_role = TuiStyleRole::Warning;
        return;
    }

    let Some(pending) = state.pending_execution.take() else {
        return;
    };
    let raw_capture = if pending.item.as_sequence().is_some() {
        state.raw_capture.take()
    } else {
        None
    };

    state.running_execution = Some(RunningExecution {
        started_at: Instant::now(),
        timeout: Duration::from_secs(pending.timeout_secs),
    });

    let sender = sender.clone();
    thread::spawn(move || {
        let mut executor = UsbTuiCommandExecutor;
        let mut raw_capture = raw_capture;
        let result = executor.execute_item(&pending, raw_capture.as_mut());
        let item = pending.item;
        let _ = sender.send(TuiExecutionResult {
            item,
            timeout_secs: pending.timeout_secs,
            result,
            raw_capture,
        });
    });
}

fn apply_finished_executions(state: &mut TuiState, receiver: &Receiver<TuiExecutionResult>) {
    while let Ok(message) = receiver.try_recv() {
        if let Some(raw_capture) = message.raw_capture {
            state.raw_capture = Some(raw_capture);
        }
        finish_execution_result(state, message.item, message.timeout_secs, message.result);
    }
}

fn open_log_entry(state: &mut TuiState, log: Option<LogEntry>) {
    let Some(log) = log else {
        state.status = "No log selected.".to_owned();
        state.status_role = TuiStyleRole::Status;
        return;
    };

    match fs::read_to_string(&log.path) {
        Ok(content) => {
            let masked_content = mask_sensitive_values(&content);
            set_response(state, ResponseState::masked(masked_content));
            state.output_masking_ack_input = None;
            state.active_command = None;
            state.viewed_log = Some(ViewedLog {
                kind: log.kind,
                path: log.path,
                label: log.label,
            });
            state.focus = Pane::Response;
            state.status = "Opened masked log.".to_owned();
            state.status_role = TuiStyleRole::Status;
        }
        Err(error) => {
            let missing_file = error.kind() == io::ErrorKind::NotFound;
            let error_text = error.to_string();
            if missing_file {
                refresh_log_summaries_or_record_error(state);
            }
            let mut response = format!(
                "Failed to read selected log.\n\nLog: {}\nReason: {}",
                log.label, error_text
            );
            if missing_file {
                response.push_str("\nLogs list refreshed.");
            }
            set_response(state, ResponseState::masked(response));
            state.active_command = None;
            state.viewed_log = None;
            state.status = "Failed to open log.".to_owned();
            state.status_role = TuiStyleRole::Error;
        }
    }
}

#[cfg(test)]
fn execute_pending_command<E>(state: &mut TuiState, executor: &mut E)
where
    E: TuiCommandExecutor,
{
    let Some(pending) = state.pending_execution.take() else {
        return;
    };
    let mut raw_capture = if pending.item.as_sequence().is_some() {
        state.raw_capture.take()
    } else {
        None
    };

    let result = executor.execute_item(&pending, raw_capture.as_mut());
    if let Some(raw_capture) = raw_capture {
        state.raw_capture = Some(raw_capture);
    }
    finish_execution_result(state, pending.item, pending.timeout_secs, result);
}

fn finish_execution_result(
    state: &mut TuiState,
    item: ExecutableItem,
    timeout_secs: u64,
    result: Result<ExecutionOutput>,
) {
    let failed_duration = state
        .running_execution
        .take()
        .map(|running| running.started_at.elapsed())
        .unwrap_or_else(|| Duration::from_secs(0));
    let finished_at = now_timestamp().display().to_owned();

    match result {
        Ok(execution) => {
            let (status, duration, masked_text, raw_text) = match &execution {
                ExecutionOutput::Command(execution) => (
                    execution.status.clone(),
                    execution.duration,
                    format_successful_execution(execution, false),
                    format_successful_execution(execution, true),
                ),
                ExecutionOutput::Sequence(execution) => (
                    execution.status.clone(),
                    execution.duration,
                    execution.masked_transcript.clone(),
                    execution.raw_transcript.clone(),
                ),
            };
            set_response(state, ResponseState::with_raw(masked_text, raw_text));
            update_sequence_candidates_from_execution(state, &execution);
            let successful_status = status.is_success();
            state.status = if successful_status {
                format!(
                    "Completed {}: status={} duration={}ms",
                    item.kind().noun(),
                    status,
                    duration.as_millis()
                )
            } else {
                format!(
                    "{} failed: status={} duration={}ms",
                    item.kind().noun(),
                    status,
                    duration.as_millis()
                )
            };
            state.status_role = if successful_status {
                TuiStyleRole::Status
            } else {
                TuiStyleRole::Error
            };
            if matches!(item, ExecutableItem::CandidateAction { .. })
                && let Some(input) = state.sequence_input.as_mut()
            {
                input.phase = SequenceInputPhase::Params;
                input.pending_candidate_action = None;
                input.confirmation_input.clear();
                input.error = None;
            }
            let raw_capture_result = match (&item, &execution) {
                (ExecutableItem::Preset(preset), ExecutionOutput::Command(execution)) => {
                    append_tui_raw_capture(state, preset, execution)
                }
                (
                    ExecutableItem::CandidateAction { preset, .. },
                    ExecutionOutput::Command(execution),
                ) => append_tui_raw_capture(state, preset, execution),
                (ExecutableItem::Sequence(_), ExecutionOutput::Sequence(_)) => Ok(()),
                _ => Ok(()),
            };
            if let Err(error) = raw_capture_result {
                state.status = format!("Completed but raw diagnostic export failed: {error}");
                state.status_role = TuiStyleRole::Error;
            }
            state.active_command = Some(
                CommandStatus::new(
                    if successful_status {
                        CommandRunState::Completed
                    } else {
                        CommandRunState::Failed
                    },
                    &item,
                    timeout_secs,
                    StatusSummary::Completed {
                        status: status.to_string(),
                        duration_ms: duration.as_millis(),
                    },
                )
                .with_finished_at(finished_at),
            );
        }
        Err(error) => {
            let detail = error.to_string();
            let raw_log_error =
                append_tui_raw_error_for_item(state, &item, failed_duration, &detail);
            set_response(
                state,
                ResponseState::masked(format!(
                    "Result: failed\n\n{} failed before response.\n{detail}",
                    item.kind().noun()
                )),
            );
            if matches!(item, ExecutableItem::CandidateAction { .. })
                && let Some(input) = state.sequence_input.as_mut()
            {
                input.phase = SequenceInputPhase::Params;
                input.pending_candidate_action = None;
                input.confirmation_input.clear();
                input.error = Some("Candidate action failed.".to_owned());
            }
            if let Err(raw_log_error) = raw_log_error {
                state.status = format!(
                    "{} failed and raw diagnostic export failed: {raw_log_error}",
                    item.kind().noun()
                );
            } else {
                state.status = format!("{} failed.", item.kind().noun());
            }
            state.status_role = TuiStyleRole::Error;
            state.active_command = Some(
                CommandStatus::new(
                    CommandRunState::Failed,
                    &item,
                    timeout_secs,
                    StatusSummary::Failed,
                )
                .with_finished_at(finished_at),
            );
        }
    }
    refresh_log_summaries_after_execution(state);
}

fn update_sequence_candidates_from_execution(state: &mut TuiState, execution: &ExecutionOutput) {
    let (candidate_sets, source_label) = match execution {
        ExecutionOutput::Command(execution) => (
            value_candidate_sets_from_text(&execution.raw_text),
            command_candidate_source(&execution.raw_text),
        ),
        ExecutionOutput::Sequence(execution) => (
            execution.value_candidate_sets.clone(),
            format!("last {} result", execution.name),
        ),
    };
    for candidate_set in candidate_sets {
        let candidate = candidate_set.candidate;
        upsert_sequence_candidate_set(
            state,
            candidate_set,
            source_label.clone(),
            now_timestamp().display().to_owned(),
        );
        if active_sequence_candidate_source(state) == Some(candidate)
            && let Some(input) = state.sequence_input.as_mut()
        {
            input.active_candidate = 0;
        }
    }
}

fn upsert_sequence_candidate_set(
    state: &mut TuiState,
    candidate_set: SequenceValueCandidateSet,
    source_label: String,
    acquired_at: String,
) {
    if candidate_set.candidates.is_empty() {
        return;
    }
    let set = TuiSequenceCandidateSet {
        candidate: candidate_set.candidate,
        candidates: candidate_set.candidates,
        source_label,
        acquired_at,
    };
    if let Some(existing) = state
        .sequence_candidate_sets
        .iter_mut()
        .find(|existing| existing.candidate == set.candidate)
    {
        *existing = set;
    } else {
        state.sequence_candidate_sets.push(set);
    }
}

fn command_candidate_source(raw_text: &str) -> String {
    let normalized = raw_text.to_ascii_uppercase();
    if normalized.contains("AT+CMGL") {
        "last direct AT+CMGL result".to_owned()
    } else if normalized.contains("AT+CGACT") || normalized.contains("AT+CGDCONT") {
        "last direct PDP context check result".to_owned()
    } else {
        "last matching command result".to_owned()
    }
}

fn append_tui_raw_capture(
    state: &mut TuiState,
    preset: &Preset,
    execution: &SendExecution,
) -> Result<()> {
    let Some(capture) = state.raw_capture.as_mut() else {
        return Ok(());
    };
    let tx = command_with_terminator(&preset.command);
    capture.append_exchange(RawLogExchange {
        command_name: Some(&preset.name),
        command: &preset.command,
        risk: execution.risk,
        status: &execution.status,
        duration: execution.duration,
        tx_bytes: tx.as_bytes(),
        rx_bytes: &execution.raw_response,
    })
}

fn append_tui_raw_error_for_item(
    state: &mut TuiState,
    item: &ExecutableItem,
    duration: Duration,
    error: &str,
) -> Result<()> {
    match item {
        ExecutableItem::Preset(preset) => append_tui_raw_error(state, preset, duration, error),
        ExecutableItem::CandidateAction { preset, .. } => {
            append_tui_raw_error(state, preset, duration, error)
        }
        ExecutableItem::Sequence(_) => Ok(()),
    }
}

fn append_tui_raw_error(
    state: &mut TuiState,
    preset: &Preset,
    duration: Duration,
    error: &str,
) -> Result<()> {
    let Some(capture) = state.raw_capture.as_mut() else {
        return Ok(());
    };
    let tx = command_with_terminator(&preset.command);
    capture.append_transport_error(RawLogTransportError {
        command_name: Some(&preset.name),
        command: &preset.command,
        risk: preset.risk,
        duration,
        stage: "execute",
        error,
        tx_bytes: tx.as_bytes(),
        rx_bytes: b"",
    })
}

fn format_successful_execution(execution: &SendExecution, raw: bool) -> String {
    let response_text = if raw {
        &execution.raw_text
    } else {
        &execution.text
    };

    if response_text.is_empty() {
        format!("No response body.\nStatus: {}", execution.status)
    } else {
        response_text.clone()
    }
}

fn format_confirmation_summary(command: &Preset) -> String {
    let mut lines = vec![
        "Command requires confirmation before sending.".to_owned(),
        format!("Name: {}", command.name),
    ];
    if should_show_preset_source_detail(&command.origin) {
        lines.push(format!("Source: {}", command.origin.label()));
        if let Some(path) = command.origin.file_path() {
            lines.push(format!("File: {path}"));
            lines.push(EXTERNAL_DEFINITION_CONFIRMATION_NOTICE.to_owned());
        }
    }
    lines.extend([
        format!("Command: {}", command.command),
        format!("Risk: {}", risk_label_text(command.risk)),
        format!("Expected effect: {}", risk_expected_effect(command.risk)),
        String::new(),
        format!("Type `{}` to send, or press Esc to cancel.", command.risk),
    ]);
    lines.join("\n")
}

fn risk_label_text(risk: crate::at::risk::RiskLevel) -> String {
    format!("[{risk}]")
}

fn risk_expected_effect(risk: crate::at::risk::RiskLevel) -> &'static str {
    match risk {
        crate::at::risk::RiskLevel::Safe => "read-only or harmless command",
        crate::at::risk::RiskLevel::Sensitive => "reads sensitive modem or subscriber data",
        crate::at::risk::RiskLevel::Write => "may change modem runtime state",
        crate::at::risk::RiskLevel::Persistent => "may change persistent modem configuration",
        crate::at::risk::RiskLevel::Dangerous => "may disrupt connectivity or modem operation",
        crate::at::risk::RiskLevel::Unknown => "effect is not known; treat as potentially unsafe",
    }
}

fn move_selection(state: &mut TuiState, delta: isize) {
    match state.focus {
        Pane::Devices => {
            if state.device_view == DeviceView::AllUsbTroubleshooting {
                state.highlighted_all_usb_device = move_index(
                    state.highlighted_all_usb_device,
                    all_usb_selectable_count(state),
                    delta,
                );
            } else {
                state.highlighted_device = move_index(
                    state.highlighted_device,
                    operation_target_selectable_count(state),
                    delta,
                );
            }
        }
        Pane::Categories => {
            let previous_category = state.selected_category;
            state.selected_category =
                move_index(state.selected_category, state.categories.len(), delta);
            if state.selected_category != previous_category {
                state.selected_command = 0;
                state.viewed_log = None;
            }
        }
        Pane::Commands => {
            let previous_command = state.selected_command;
            state.selected_command = move_index(
                state.selected_command,
                state.visible_commands().len(),
                delta,
            );
            if state.selected_command != previous_command {
                state.viewed_log = None;
            }
        }
        Pane::History => {
            state.selected_log = move_index(state.selected_log, state.logs.len(), delta);
        }
        Pane::Controls => {
            state.selected_control =
                move_index(state.selected_control, control_rows(state).len(), delta);
        }
        _ => {}
    }
}

#[derive(Debug, Copy, Clone)]
enum ListBoundary {
    Start,
    End,
}

fn move_selection_to_boundary(state: &mut TuiState, boundary: ListBoundary) {
    match state.focus {
        Pane::Devices => {
            if state.device_view == DeviceView::AllUsbTroubleshooting {
                state.highlighted_all_usb_device =
                    boundary_index(all_usb_selectable_count(state), boundary);
            } else {
                state.highlighted_device =
                    boundary_index(operation_target_selectable_count(state), boundary);
            }
        }
        Pane::Categories => {
            let previous_category = state.selected_category;
            state.selected_category = boundary_index(state.categories.len(), boundary);
            if state.selected_category != previous_category {
                state.selected_command = 0;
                state.viewed_log = None;
            }
        }
        Pane::Commands => {
            let previous_command = state.selected_command;
            state.selected_command = boundary_index(state.visible_commands().len(), boundary);
            if state.selected_command != previous_command {
                state.viewed_log = None;
            }
        }
        Pane::History => {
            state.selected_log = boundary_index(state.logs.len(), boundary);
        }
        Pane::Controls => {
            state.selected_control = boundary_index(control_rows(state).len(), boundary);
        }
        _ => {}
    }
}

fn boundary_index(len: usize, boundary: ListBoundary) -> usize {
    match boundary {
        ListBoundary::Start => 0,
        ListBoundary::End => len.saturating_sub(1),
    }
}

fn focused_page_size(state: &TuiState) -> usize {
    match state.focus {
        Pane::Devices => device_item_capacity(state),
        Pane::Categories => state.categories_visible_height.max(1),
        Pane::Commands => state.commands_visible_height.max(1),
        Pane::Controls => state.controls_visible_height.max(1),
        Pane::History => log_item_capacity(state),
        _ => 1,
    }
}

fn scroll_response(state: &mut TuiState, delta: isize) {
    let max_scroll = response_max_scroll(state);
    if delta.is_negative() {
        state.response_scroll = state.response_scroll.saturating_sub(delta.unsigned_abs());
    } else {
        state.response_scroll = state
            .response_scroll
            .saturating_add(delta as usize)
            .min(max_scroll);
    }
}

fn response_max_scroll(state: &TuiState) -> usize {
    response_lines(state)
        .len()
        .saturating_sub(state.response_visible_height.max(1))
}

fn pane_inner_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

fn utility_column_width(width: u16) -> u16 {
    (width / 5).clamp(33, 36)
}

fn category_column_width(width: u16) -> u16 {
    (width / 6).clamp(18, 26)
}

fn device_panel_height(state: &TuiState, area: Rect) -> u16 {
    let device_count_for_height = if state.device_view == DeviceView::AllUsbTroubleshooting {
        state.all_usb_devices.len()
    } else {
        state.devices.len()
    };
    if device_count_for_height == 0 {
        area.height.min(6)
    } else {
        area.height.min(8)
    }
}

fn viewport_start(selected: usize, len: usize, visible_slots: usize) -> usize {
    let visible_slots = visible_slots.max(1);
    if len <= visible_slots {
        0
    } else {
        selected
            .saturating_sub(visible_slots / 2)
            .min(len - visible_slots)
    }
}

fn device_item_capacity(state: &TuiState) -> usize {
    let fixed_rows = if state.device_view == DeviceView::AllUsbTroubleshooting {
        4
    } else if state.devices.is_empty() {
        0
    } else {
        let selection_hint = usize::from(state.active_device.is_none() && state.devices.len() > 1);
        3 + selection_hint
    };
    state
        .devices_visible_height
        .saturating_sub(fixed_rows)
        .max(1)
}

fn log_item_capacity(state: &TuiState) -> usize {
    state.logs_visible_height.saturating_sub(1).max(1)
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let max = len - 1;
    current.saturating_add_signed(delta).min(max)
}

impl Pane {
    fn next(self) -> Self {
        match self {
            Self::Devices => Self::Categories,
            Self::Categories => Self::Commands,
            Self::Commands => Self::Controls,
            Self::Controls => Self::Response,
            Self::Response => Self::History,
            Self::History => Self::Devices,
            Self::Status => Self::Categories,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Devices => Self::History,
            Self::Categories => Self::Devices,
            Self::Commands => Self::Categories,
            Self::Controls => Self::Commands,
            Self::Response => Self::Controls,
            Self::History => Self::Response,
            Self::Status => Self::History,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Devices => "Devices",
            Self::Categories => "Categories",
            Self::Commands => "Commands / Sequences",
            Self::Controls => "Controls",
            Self::Response => "Response",
            Self::Status => "Status",
            Self::History => "Logs",
        }
    }
}

fn render_frame(frame: &mut Frame<'_>, state: &mut TuiState) {
    let theme = state.theme;
    let area = frame.area();
    frame.render_widget(
        Block::default().style(theme.style(TuiStyleRole::Background)),
        area,
    );
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    let top_band_height = root[0].height / 2;
    let bands = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_band_height), Constraint::Fill(1)])
        .split(root[0]);
    let utility_width = utility_column_width(root[0].width);
    let category_width = category_column_width(root[0].width);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(utility_width),
            Constraint::Length(category_width),
            Constraint::Fill(1),
        ])
        .split(bands[0]);
    let top_left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(device_panel_height(state, top[0])),
            Constraint::Fill(1),
        ])
        .split(top[0]);
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(utility_width), Constraint::Fill(1)])
        .split(bands[1]);
    let bottom_main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(bottom[1]);

    state.devices_visible_height = pane_inner_height(top_left[0]);
    state.categories_visible_height = pane_inner_height(top[1]);
    state.commands_visible_height = pane_inner_height(top[2]);
    state.controls_visible_height = pane_inner_height(bottom[0]);
    state.logs_visible_height = pane_inner_height(bottom_main[1]);
    render_devices(frame, top_left[0], state, &theme);
    render_status(frame, top_left[1], state, &theme);
    render_categories(frame, top[1], state, &theme);
    render_commands(frame, top[2], state, &theme);
    render_controls(frame, bottom[0], state, &theme);
    render_response(frame, bottom_main[0], state, &theme);
    render_history(frame, bottom_main[1], state, &theme);
    render_footer(frame, root[1], state, &theme);

    if let Some(confirmation) = &state.confirmation {
        render_confirmation(frame, centered_rect(76, 64, area), confirmation, &theme);
    }
    if state.output_masking_ack_input.is_some() {
        render_output_masking_confirmation(frame, centered_rect(76, 64, area), state, &theme);
    }
    if state.response_action_confirmation.is_some() {
        render_response_action_confirmation(frame, centered_rect(76, 64, area), state, &theme);
    }
    if state.raw_log_path_input.is_some() {
        render_raw_log_path_input(frame, centered_rect(76, 36, area), state, &theme);
    }
    if state.raw_log_ack_input.is_some() {
        render_raw_log_ack_input(frame, centered_rect(76, 48, area), state, &theme);
    }
    if state.search_input.is_some() {
        render_search_input(frame, centered_rect(64, 32, area), state, &theme);
    }
    if state.edit_input.is_some() {
        render_edit_input(frame, centered_rect(76, 42, area), state, &theme);
    }
    if state.sequence_input.is_some() {
        render_sequence_input(frame, centered_rect(76, 76, area), state, &theme);
    }
    if state.ad_hoc_input.is_some() {
        render_ad_hoc_input(frame, centered_rect(76, 42, area), state, &theme);
    }
    if state.timeout_input.is_some() {
        render_timeout_input(frame, centered_rect(64, 32, area), state, &theme);
    }
    if state.action_menu.is_some() {
        render_action_menu(frame, centered_rect(76, 80, area), state, &theme);
    }
    if state.show_help {
        render_help(frame, centered_rect(76, 90, area), &theme);
    }
}

fn render_devices(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let content = device_lines(state, theme);
    frame.render_widget(
        List::new(content)
            .block(pane_block(Pane::Devices, state.focus, theme))
            .style(theme.style(TuiStyleRole::Text)),
        area,
    );
}

fn device_lines(state: &TuiState, theme: &TuiTheme) -> Vec<ListItem<'static>> {
    if state.device_view == DeviceView::AllUsbTroubleshooting {
        return all_usb_device_lines(state, theme);
    }

    if state.devices.is_empty() {
        let mut lines = vec![
            ListItem::new("No matching USB"),
            ListItem::new("device visible."),
            ListItem::new("Sending disabled.").style(theme.style(TuiStyleRole::Warning)),
        ];
        if state.all_usb_devices.is_empty() {
            lines.push(ListItem::new("No USB visible.").style(theme.style(TuiStyleRole::Muted)));
        } else {
            lines.push(device_action_item(
                state.highlighted_device == 0 && state.focus == Pane::Devices,
                "Show all USB devices",
                theme,
            ));
        }
        return lines;
    }

    let mut lines = vec![ListItem::new(format!(
        "Visible targets: {}",
        state.devices.len()
    ))];

    if state.focus != Pane::Devices
        && let Some(device) = state.active_device()
    {
        lines.extend(device_detail_lines(device, theme));
        if state.devices.len() > 1 {
            lines.push(
                ListItem::new("Tab to Devices to reselect").style(theme.style(TuiStyleRole::Muted)),
            );
        }
        if !state.all_usb_devices.is_empty() {
            lines.push(
                ListItem::new("Enter Devices for all USB").style(theme.style(TuiStyleRole::Muted)),
            );
        }
        return lines;
    }

    if state.active_device.is_none() && state.devices.len() > 1 {
        lines.push(ListItem::new("Select target first.").style(theme.style(TuiStyleRole::Warning)));
    }

    let visible_slots = device_item_capacity(state);
    let selected_device = state
        .highlighted_device
        .min(state.devices.len().saturating_sub(1));
    let start = viewport_start(selected_device, state.devices.len(), visible_slots);
    lines.extend(
        state
            .devices
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_slots)
            .map(|(index, device)| {
                let prefix = match (
                    index == state.highlighted_device,
                    state.active_device == Some(index),
                ) {
                    (true, true) => ">*",
                    (true, false) => "> ",
                    (false, true) => "* ",
                    (false, false) => "  ",
                };
                let line = format!(
                    "{prefix}{label} {vid}:{pid} bus={bus} addr={address}",
                    label = device_display_label(device),
                    vid = hex_u16(device.vendor_id),
                    pid = hex_u16(device.product_id),
                    bus = device.bus,
                    address = device.address
                );
                let style = if index == state.highlighted_device {
                    theme.style(TuiStyleRole::Selected)
                } else if state.active_device == Some(index) {
                    theme.style(TuiStyleRole::Status)
                } else {
                    theme.style(TuiStyleRole::Text)
                };
                ListItem::new(line).style(style)
            }),
    );
    lines.push(ListItem::new("Enter select").style(theme.style(TuiStyleRole::Muted)));
    if !state.all_usb_devices.is_empty() {
        lines.push(device_action_item(
            state.highlighted_device >= state.devices.len(),
            "Show all USB devices",
            theme,
        ));
    }
    lines
}

fn all_usb_device_lines(state: &TuiState, theme: &TuiTheme) -> Vec<ListItem<'static>> {
    if state.all_usb_devices.is_empty() {
        return vec![
            ListItem::new("All USB: 0"),
            ListItem::new("No USB visible.").style(theme.style(TuiStyleRole::Warning)),
            device_action_item(
                state.highlighted_all_usb_device == 0 && state.focus == Pane::Devices,
                "Show operation targets",
                theme,
            ),
        ];
    }

    let mut lines = vec![
        ListItem::new(format!("All USB: {}", state.all_usb_devices.len())),
        ListItem::new("Troubleshooting view").style(theme.style(TuiStyleRole::Muted)),
    ];
    let visible_slots = device_item_capacity(state);
    let selected_device = state
        .highlighted_all_usb_device
        .min(state.all_usb_devices.len().saturating_sub(1));
    let start = viewport_start(selected_device, state.all_usb_devices.len(), visible_slots);
    lines.extend(
        state
            .all_usb_devices
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_slots)
            .map(|(index, device)| {
                let target_index = state.target_index_for_device(device);
                let marker = if target_index.is_some() {
                    "[target]"
                } else {
                    "[diagnostic-only]"
                };
                let active = target_index.is_some_and(|target| state.active_device == Some(target));
                let prefix = match (index == state.highlighted_all_usb_device, active) {
                    (true, true) => ">*",
                    (true, false) => "> ",
                    (false, true) => "* ",
                    (false, false) => "  ",
                };
                let line = format!(
                    "{prefix}{marker} {label} {vid}:{pid} bus={bus} addr={address}",
                    label = device_display_label(device),
                    vid = hex_u16(device.vendor_id),
                    pid = hex_u16(device.product_id),
                    bus = device.bus,
                    address = device.address
                );
                let style = if index == state.highlighted_all_usb_device {
                    theme.style(TuiStyleRole::Selected)
                } else if active {
                    theme.style(TuiStyleRole::Status)
                } else if target_index.is_some() {
                    theme.style(TuiStyleRole::Text)
                } else {
                    theme.style(TuiStyleRole::Muted)
                };
                ListItem::new(line).style(style)
            }),
    );
    lines.push(ListItem::new("Enter target only").style(theme.style(TuiStyleRole::Muted)));
    lines.push(device_action_item(
        state.highlighted_all_usb_device >= state.all_usb_devices.len(),
        "Show operation targets",
        theme,
    ));
    lines
}

fn device_action_item(selected: bool, label: &'static str, theme: &TuiTheme) -> ListItem<'static> {
    selected_item(selected, label, theme)
}

fn operation_target_selectable_count(state: &TuiState) -> usize {
    state.devices.len() + usize::from(!state.all_usb_devices.is_empty())
}

fn all_usb_selectable_count(state: &TuiState) -> usize {
    state.all_usb_devices.len() + 1
}

fn device_detail_lines(device: &UsbDeviceInfo, theme: &TuiTheme) -> Vec<ListItem<'static>> {
    let mut lines =
        vec![ListItem::new("Selected device:").style(theme.style(TuiStyleRole::Status))];
    if let Some(manufacturer) = &device.manufacturer {
        lines.push(ListItem::new(format!("Manufacturer: {manufacturer}")));
    }
    if let Some(product) = &device.product {
        lines.push(ListItem::new(format!("Product: {product}")));
    }
    lines.push(ListItem::new(format!("VID: {}", hex_u16(device.vendor_id))));
    lines.push(ListItem::new(format!(
        "PID: {}",
        hex_u16(device.product_id)
    )));
    lines.push(ListItem::new(format!("Bus: {}", device.bus)));
    lines.push(ListItem::new(format!("Address: {}", device.address)));
    lines
}

fn device_display_label(device: &UsbDeviceInfo) -> String {
    device
        .product
        .as_deref()
        .or(device.manufacturer.as_deref())
        .unwrap_or("USB device")
        .to_owned()
}

fn hex_u16(value: u16) -> String {
    format!("0x{value:04x}")
}

fn render_categories(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let visible_slots = pane_inner_height(area).max(1);
    let start = viewport_start(
        state.selected_category,
        state.categories.len(),
        visible_slots,
    );
    let items = state
        .categories
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_slots)
        .map(|(index, category)| selected_item(index == state.selected_category, category, theme))
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items)
            .block(pane_block(Pane::Categories, state.focus, theme))
            .style(theme.style(TuiStyleRole::Text)),
        area,
    );
}

fn render_controls(frame: &mut Frame<'_>, area: Rect, state: &mut TuiState, theme: &TuiTheme) {
    let rows = control_rows(state);
    let selected = state.selected_control.min(rows.len().saturating_sub(1));
    let feedback = controls_feedback_for_render(state, &rows, selected);

    frame.render_widget(Clear, area);
    let block = pane_block(Pane::Controls, state.focus, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (list_area, feedback_area) = controls_inner_areas(inner, feedback.as_ref());
    let visible_slots = list_area.height.max(1) as usize;
    state.controls_visible_height = visible_slots;
    let start = viewport_start(selected, rows.len(), visible_slots);
    let items = rows
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_slots)
        .map(|(index, row)| control_row_item(index == selected, row, theme))
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).style(theme.style(TuiStyleRole::Text)),
        list_area,
    );

    if let (Some(feedback), Some(area)) = (feedback, feedback_area) {
        render_surface_feedback(frame, area, &feedback, theme);
    }
}

fn control_row_item(selected: bool, row: &ControlRow, theme: &TuiTheme) -> ListItem<'static> {
    let prefix = if selected { "> " } else { "  " };
    let text = match &row.inline_state {
        Some(inline_state) => format!("{prefix}{} {inline_state}", row.label),
        None => format!("{prefix}{}", row.label),
    };
    let style = if selected {
        theme.style(TuiStyleRole::Selected)
    } else if row.action == ControlAction::ToggleOutputMasking
        && row.inline_state.as_deref() == Some("off")
    {
        theme.style(TuiStyleRole::Warning)
    } else if row.enabled {
        theme.style(TuiStyleRole::Text)
    } else {
        theme.style(TuiStyleRole::Muted)
    };
    ListItem::new(text).style(style)
}

fn controls_inner_areas(inner: Rect, feedback: Option<&ControlsFeedback>) -> (Rect, Option<Rect>) {
    if feedback.is_none() || inner.height < 3 {
        return (inner, None);
    }

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(inner);
    (areas[0], Some(areas[1]))
}

fn controls_feedback_for_render(
    state: &TuiState,
    rows: &[ControlRow],
    selected: usize,
) -> Option<ControlsFeedback> {
    state.controls_feedback.clone().or_else(|| {
        if state.focus != Pane::Controls {
            return None;
        }
        rows.get(selected)
            .filter(|row| !row.enabled)
            .map(|row| ControlsFeedback {
                message: row.unavailable_message.to_owned(),
                role: TuiStyleRole::Muted,
            })
    })
}

fn render_surface_feedback(
    frame: &mut Frame<'_>,
    area: Rect,
    feedback: &ControlsFeedback,
    theme: &TuiTheme,
) {
    let mut lines = Vec::new();
    if area.height > 1 {
        lines.push(Line::from(Span::styled(
            "-".repeat(area.width as usize),
            theme.style(TuiStyleRole::Muted),
        )));
    }
    lines.push(Line::from(Span::styled(
        feedback.message.clone(),
        theme.style(feedback.role),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn control_rows(state: &TuiState) -> Vec<ControlRow> {
    let device_ready = device_gate_message(state).is_none();
    let raw_export_label = if state.raw_capture.is_some() {
        "Stop raw export"
    } else {
        "Start raw export"
    };
    let edit_selected_label = if state
        .selected_command()
        .is_some_and(|command| command.as_sequence().is_some())
    {
        "Sequence inputs"
    } else {
        "Edit selected"
    };

    vec![
        control_row(
            ControlAction::AdHocCommand,
            "AT command",
            None,
            device_ready,
            "Select a device first.",
        ),
        control_row(
            ControlAction::EditCommand,
            edit_selected_label,
            None,
            device_ready && state.selected_command().is_some(),
            if device_ready {
                "No command is selected."
            } else {
                "Select a device first."
            },
        ),
        control_row(
            ControlAction::SetTimeout,
            "Timeout",
            Some(
                state
                    .selected_command()
                    .map(|command| format!("{}s", effective_timeout_secs(state, command)))
                    .unwrap_or_else(|| format!("{}s", DEFAULT_COMMAND_TIMEOUT_SECS)),
            ),
            true,
            "",
        ),
        control_row(ControlAction::RawExport, raw_export_label, None, true, ""),
        control_row(
            ControlAction::ToggleOutputMasking,
            "Output masking",
            Some(output_masking_state_label(state).to_owned()),
            true,
            "",
        ),
    ]
}

fn control_row(
    action: ControlAction,
    label: &str,
    inline_state: Option<String>,
    enabled: bool,
    unavailable_message: &'static str,
) -> ControlRow {
    ControlRow {
        action,
        label: label.to_owned(),
        inline_state,
        enabled,
        unavailable_message,
    }
}

fn action_menu_rows(state: &TuiState, kind: ActionMenuKind) -> Vec<ActionMenuRow> {
    match kind {
        ActionMenuKind::Response => response_action_rows(state),
        ActionMenuKind::Log => log_action_rows(state),
    }
}

fn response_action_rows(state: &TuiState) -> Vec<ActionMenuRow> {
    if state.viewed_log.is_some() {
        return log_view_response_action_rows(state);
    }

    let copy_ready = copyable_response_text(state).is_some();
    let response_export = response_export_request(state);
    let export_ready = response_export.is_some();
    let unmasked = response_has_unmasked_content(state);
    let clear_ready = !state.response.is_empty();
    let mut rows = Vec::new();

    if copy_ready {
        rows.push(action_menu_row(
            ActionMenuAction::CopyResponse,
            if unmasked {
                "Copy unmasked response"
            } else {
                "Copy response"
            },
            true,
            "No response is available to copy.",
        ));
    }
    if export_ready {
        rows.push(action_menu_row(
            ActionMenuAction::ExportResponse,
            if response_export.is_some_and(|request| !request.masked) {
                "Export unmasked response..."
            } else {
                "Export response..."
            },
            true,
            "No response is available to export.",
        ));
    }
    if let Some(exported_response) = &state.exported_response {
        rows.push(action_menu_row(
            ActionMenuAction::RevealInFinder,
            "Reveal in Finder",
            exported_response.path.is_file(),
            "Exported response no longer exists.",
        ));
    }
    if clear_ready {
        rows.push(action_menu_row(
            ActionMenuAction::ClearResponse,
            "Clear response",
            true,
            "No response is available to clear.",
        ));
    }

    rows
}

fn log_view_response_action_rows(state: &TuiState) -> Vec<ActionMenuRow> {
    let mut rows = Vec::new();

    if copyable_response_text(state).is_some() {
        rows.push(action_menu_row(
            ActionMenuAction::CopyDisplayedLog,
            "Copy displayed log",
            true,
            "No log body is available to copy.",
        ));
    }
    let reveal_ready = state
        .viewed_log
        .as_ref()
        .is_some_and(|log| log.path.is_file());
    rows.push(action_menu_row(
        ActionMenuAction::RevealInFinder,
        "Reveal in Finder",
        reveal_ready,
        "Saved log no longer exists.",
    ));
    rows.push(action_menu_row(
        ActionMenuAction::CloseLogView,
        "Close log view",
        true,
        "No log is open in Response.",
    ));

    rows
}

fn log_action_rows(state: &TuiState) -> Vec<ActionMenuRow> {
    let log_ready = selected_action_log(state).is_some();
    let mut rows = Vec::new();

    if log_ready {
        rows.push(action_menu_row(
            ActionMenuAction::OpenLog,
            "Open log in Response",
            true,
            "No log is selected.",
        ));
        let reveal_ready = selected_action_log(state).is_some_and(|log| log.path.is_file());
        rows.push(action_menu_row(
            ActionMenuAction::RevealInFinder,
            "Reveal in Finder",
            reveal_ready,
            "Saved log no longer exists.",
        ));
    }
    rows
}

fn selected_action_log(state: &TuiState) -> Option<&LogEntry> {
    if let Some(menu) = &state.action_menu
        && menu.kind == ActionMenuKind::Log
    {
        return menu.log_target.as_ref();
    }
    state.selected_log()
}

fn action_menu_row(
    action: ActionMenuAction,
    label: impl Into<String>,
    enabled: bool,
    unavailable_message: impl Into<String>,
) -> ActionMenuRow {
    ActionMenuRow {
        action,
        label: label.into(),
        enabled,
        unavailable_message: unavailable_message.into(),
    }
}

fn render_commands(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let commands = state.visible_commands();
    let rows = command_list_rows(&commands);
    let visible_slots = pane_inner_height(area).max(1);
    let selected_row = selected_command_row_index(&rows, state.selected_command);
    let start = viewport_start(selected_row, rows.len(), visible_slots);
    let items = rows
        .iter()
        .skip(start)
        .take(visible_slots)
        .map(|row| command_row_item(row, state.selected_command, theme))
        .collect::<Vec<_>>();
    let title = if state.search_query.is_empty() {
        Pane::Commands.title().to_owned()
    } else {
        format!("Commands / Sequences search: {}", state.search_query)
    };
    frame.render_widget(
        List::new(items)
            .block(pane_block_with_title(
                Pane::Commands,
                state.focus,
                theme,
                title,
            ))
            .style(theme.style(TuiStyleRole::Text)),
        area,
    );
}

fn command_list_rows<'a>(commands: &[&'a ExecutableItem]) -> Vec<CommandListRow<'a>> {
    let show_kind_headers = commands.iter().any(|command| command.as_preset().is_some())
        && commands
            .iter()
            .any(|command| command.as_sequence().is_some());
    let show_preset_headers = commands.iter().any(
        |command| matches!(command, ExecutableItem::Preset(preset) if !preset.origin.is_built_in()),
    );
    let show_sequence_headers = commands.iter().any(|command| {
        matches!(command, ExecutableItem::Sequence(sequence) if !sequence.origin.is_built_in())
    });
    let mut rows = Vec::new();
    let mut current_kind: Option<ExecutableKind> = None;
    let mut current_source: Option<&str> = None;
    let mut rows_in_current_kind = 0usize;

    for (command_index, command) in commands.iter().enumerate() {
        if current_kind != Some(command.kind()) {
            if current_kind.is_some() {
                rows.push(CommandListRow::BlankSeparator);
            }
            if show_kind_headers {
                rows.push(CommandListRow::KindHeader(match command.kind() {
                    ExecutableKind::Command => "Commands",
                    ExecutableKind::Sequence => "Sequences",
                    ExecutableKind::CandidateAction => "Actions",
                }));
            }
            current_kind = Some(command.kind());
            current_source = None;
            rows_in_current_kind = 0;
        }

        let should_group_source = match command {
            ExecutableItem::Preset(_) => show_preset_headers,
            ExecutableItem::Sequence(_) => show_sequence_headers,
            ExecutableItem::CandidateAction { .. } => false,
        };
        if should_group_source
            && let Some(source_label) = command.source_detail()
            && current_source != Some(source_label)
        {
            if rows_in_current_kind > 0 {
                rows.push(CommandListRow::BlankSeparator);
            }
            rows.push(CommandListRow::SourceHeader(source_label.to_owned()));
            current_source = Some(source_label);
            rows_in_current_kind += 1;
        }
        rows.push(CommandListRow::Command {
            command_index,
            command,
        });
        rows_in_current_kind += 1;
    }

    rows
}

fn selected_command_row_index(rows: &[CommandListRow<'_>], selected_command: usize) -> usize {
    rows.iter()
        .position(|row| {
            matches!(
                row,
                CommandListRow::Command { command_index, .. } if *command_index == selected_command
            )
        })
        .unwrap_or(0)
}

fn render_response(frame: &mut Frame<'_>, area: Rect, state: &mut TuiState, theme: &TuiTheme) {
    let lines = response_lines(state);
    let inner = pane_block(Pane::Response, state.focus, theme).inner(area);
    let (body_area, feedback_area) =
        response_inner_areas(inner, state.response_action_feedback.as_ref());
    let visible_height = body_area.height.max(1) as usize;
    state.response_visible_height = visible_height;
    let max_scroll = lines.len().saturating_sub(visible_height);
    let scroll = state.response_scroll.min(max_scroll);
    state.response_scroll = scroll;
    let visible_lines = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect::<Vec<_>>();

    frame.render_widget(Clear, area);
    let block = response_block(
        state,
        theme,
        lines_len_for_title(state),
        visible_height,
        scroll,
    );
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(visible_lines)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        body_area,
    );

    if let (Some(feedback), Some(area)) = (&state.response_action_feedback, feedback_area) {
        render_surface_feedback(frame, area, feedback, theme);
    }
}

fn response_inner_areas(inner: Rect, feedback: Option<&ControlsFeedback>) -> (Rect, Option<Rect>) {
    if feedback.is_none() || inner.height < 2 {
        return (inner, None);
    }

    let feedback_height = inner.height.saturating_sub(1).min(4);
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(feedback_height)])
        .split(inner);
    (areas[0], Some(areas[1]))
}

fn response_lines(state: &TuiState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if should_show_output_masking_context(state) {
        lines.push(Line::from(format!(
            "Output masking: {}",
            output_masking_state_label(state)
        )));
        lines.push(Line::from(""));
    }

    if let Some(cleared_at) = &state.response_cleared_at {
        lines.push(Line::from("Response body cleared."));
        lines.push(Line::from(format!("Cleared: {cleared_at}")));
    } else if state.response.is_empty() {
        lines.push(Line::from("No response."));
    } else {
        let text_lines = state
            .response
            .visible_text(state.output_masking_enabled || state.viewed_log.is_some())
            .lines()
            .collect::<Vec<_>>();
        if state.viewed_log.is_some() {
            let width = text_lines.len().max(1).to_string().len();
            lines.extend(
                text_lines
                    .iter()
                    .enumerate()
                    .map(|(index, line)| Line::from(format!("{:>width$}  {line}", index + 1))),
            );
        } else {
            lines.extend(text_lines.iter().map(|line| Line::from((*line).to_owned())));
        }
    }

    lines
}

fn lines_len_for_title(state: &TuiState) -> usize {
    response_lines(state).len()
}

fn response_block(
    state: &TuiState,
    theme: &TuiTheme,
    total_lines: usize,
    visible_height: usize,
    scroll: usize,
) -> Block<'static> {
    let title = if state.viewed_log.is_some() {
        response_range_title(total_lines, visible_height, scroll)
    } else {
        Pane::Response.title().to_owned()
    };
    pane_block_with_title(Pane::Response, state.focus, theme, title)
}

fn response_range_title(total_lines: usize, visible_height: usize, scroll: usize) -> String {
    let total = total_lines.max(1);
    let visible = visible_height.max(1);
    let first = scroll.saturating_add(1).min(total);
    let last = scroll.saturating_add(visible).min(total);
    let position = if total <= visible {
        "all"
    } else if scroll == 0 {
        "top/more below"
    } else if last == total {
        "bottom"
    } else {
        "more above/below"
    };
    format!("Response {first}-{last}/{total} {position}")
}

fn render_action_menu(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(menu) = &state.action_menu else {
        return;
    };
    let rows = action_menu_rows(state, menu.kind);
    let selected = menu.selected.min(rows.len().saturating_sub(1));
    let title = match menu.kind {
        ActionMenuKind::Response if state.viewed_log.is_some() => "Log view actions",
        ActionMenuKind::Response => "Response actions",
        ActionMenuKind::Log => "Log actions",
    };
    let mut lines = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        lines.push(action_menu_line(index == selected, row, theme));
    }
    if let Some(feedback) = &menu.feedback {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            feedback.message.clone(),
            theme.style(feedback.role),
        )));
    }
    lines.push(Line::from(""));
    for context in action_menu_context_lines(state, menu.kind) {
        lines.push(Line::from(Span::styled(
            context,
            theme.style(TuiStyleRole::Muted),
        )));
    }
    if rows.is_empty() {
        lines.push(Line::from("Esc or q closes."));
    } else {
        lines.push(Line::from("Enter selects. Esc or q cancels."));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn action_menu_context_lines(state: &TuiState, kind: ActionMenuKind) -> Vec<String> {
    match kind {
        ActionMenuKind::Response if state.viewed_log.is_some() => state
            .viewed_log
            .as_ref()
            .map(|log| vec![format!("Log: {}", log.label)])
            .unwrap_or_default(),
        ActionMenuKind::Response => response_action_context_lines(state),
        ActionMenuKind::Log => selected_action_log(state)
            .map(|log| vec![format!("Log: {}", log.label)])
            .unwrap_or_default(),
    }
}

fn response_action_context_lines(state: &TuiState) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(request) = state
        .action_menu
        .as_ref()
        .and_then(|menu| menu.response_export.as_ref())
    {
        lines.push(format!("Response: {}", request.response_label));
        if let Some(finished_at) = &request.finished_at {
            lines.push(format!("Completed: {finished_at}"));
        }
        lines.push("Export format: UTF-8 text".to_owned());
        lines.push(format!(
            "Export content: {}",
            if request.masked { "masked" } else { "unmasked" }
        ));
        lines.push(format!("File name: {}", request.file_name));
    }
    if let Some(exported_response) = &state.exported_response {
        lines.push(format!(
            "Last exported response: {}",
            exported_response.response_label
        ));
        if let Some(finished_at) = &exported_response.finished_at {
            lines.push(format!("Last export completed: {finished_at}"));
        }
        lines.push(format!(
            "Last export content: {}",
            if exported_response.masked {
                "masked"
            } else {
                "unmasked"
            }
        ));
        lines.push(format!(
            "Last exported file: {}",
            compact_path_label(&exported_response.path)
        ));
    }
    lines
}

fn action_menu_line(selected: bool, row: &ActionMenuRow, theme: &TuiTheme) -> Line<'static> {
    let prefix = if selected { "> " } else { "  " };
    let style = if selected {
        theme.style(TuiStyleRole::Selected)
    } else if row.enabled {
        theme.style(TuiStyleRole::Text)
    } else {
        theme.style(TuiStyleRole::Muted)
    };
    Line::from(Span::styled(format!("{prefix}{}", row.label), style))
}

fn copyable_response_text(state: &TuiState) -> Option<String> {
    copyable_response_text_for_masking(state, state.output_masking_enabled)
}

fn copyable_response_text_for_masking(
    state: &TuiState,
    output_masking_enabled: bool,
) -> Option<String> {
    if state.response.is_empty() {
        return None;
    }

    let body = trim_empty_edge_lines(
        state
            .response
            .visible_text(output_masking_enabled || state.viewed_log.is_some()),
    );
    if body.is_empty() {
        return None;
    }

    if state.viewed_log.is_some() {
        return Some(body.to_owned());
    }

    let Some(active) = &state.active_command else {
        return Some(body.to_owned());
    };
    let Some(command) = active.command.as_deref() else {
        return Some(body.to_owned());
    };
    let command = if output_masking_enabled {
        mask_sensitive_values(command)
    } else {
        command.to_owned()
    };
    if body_starts_with_command_echo(body, &command) {
        Some(body.to_owned())
    } else {
        Some(format!("{command}\n{body}"))
    }
}

fn body_starts_with_command_echo(body: &str, command: &str) -> bool {
    body.lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| normalize_command(line) == normalize_command(command))
}

fn trim_empty_edge_lines(value: &str) -> &str {
    value.trim_matches(|ch| ch == '\n')
}

fn render_status(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let mut lines = Vec::new();
    let content_width = area.width.saturating_sub(2) as usize;
    let content_height = area.height.saturating_sub(2) as usize;

    if let Some(viewed_log) = &state.viewed_log {
        lines.push(Line::from(vec![
            Span::styled("Status: ", theme.style(state.status_role)),
            Span::raw(viewed_log_status_label(state)),
        ]));
        let kind = match viewed_log.kind {
            LogListingKind::History => "history",
            LogListingKind::Session => "session",
        };
        lines.push(Line::from(format!("Type: {kind}")));
        lines.push(Line::from(format!("Log: {}", viewed_log.label)));
        lines.push(Line::from("Raw values: not shown"));
    } else if let Some(active) = &state.active_command {
        lines.push(Line::from(vec![
            Span::styled("Status: ", theme.style(state.status_role)),
            Span::raw(active.state_label()),
        ]));
        let target_label = active.target_status_label();
        if state.running_execution.is_some() {
            lines.push(Line::from(format!("{target_label}: {}", active.name)));
            if let Some(source_title) = &active.source_title {
                lines.push(Line::from(format!("Source: {source_title}")));
            }
            if let Some(command) = &active.command {
                lines.push(Line::from(format!("AT command: {command}")));
            }
            lines.push(risk_line("Risk: ", active.risk, theme));
        } else {
            if let (Some(finished_at), Some(label)) =
                (active.finished_at.as_deref(), active.terminal_event_label())
            {
                push_terminal_event_time_status_lines(
                    &mut lines,
                    label,
                    finished_at,
                    content_width,
                );
            }
            lines.push(Line::from(format!("{target_label}: {}", active.name)));
            let post_result_context_lines =
                1 + usize::from(should_show_output_masking_context(state));
            let result_context_lines = minimum_result_status_line_count(active);
            let required_after_optional = result_context_lines + post_result_context_lines;
            let mut optional_context_lines =
                content_height.saturating_sub(lines.len().saturating_add(required_after_optional));
            if optional_context_lines > 0
                && let Some(source_title) = &active.source_title
            {
                lines.push(Line::from(format!("Source: {source_title}")));
                optional_context_lines -= 1;
            }
            if optional_context_lines > 0
                && let Some(command) = &active.command
            {
                lines.push(Line::from(format!("AT command: {command}")));
            }
            if let Some(summary) = active.status_summary_line() {
                lines.push(Line::from(summary));
            }
            lines.push(risk_line("Risk: ", active.risk, theme));
        }
        if should_show_output_masking_context(state) {
            lines.push(Line::from(format!(
                "Output masking: {}",
                output_masking_state_label(state)
            )));
        }
    } else if let Some(selected) = state.selected_command() {
        lines.push(Line::from(vec![
            Span::styled("Status: ", theme.style(state.status_role)),
            Span::raw(state.status.as_str()),
        ]));
        lines.push(Line::from(format!(
            "Selected {}: {}",
            selected.kind().noun(),
            selected.name()
        )));
        if let Some(detail) = selected.source_detail() {
            lines.push(Line::from(format!("Source: {detail}")));
        }
        if let Some(command) = selected.command_text() {
            lines.push(Line::from(format!("AT command: {command}")));
        }
        lines.push(risk_line("Risk: ", selected.risk(), theme));
        lines.push(Line::from(format!(
            "Timeout: {}s",
            effective_timeout_secs(state, selected)
        )));
        if let Some(device) = state.active_device() {
            lines.push(Line::from(format!(
                "Device: bus={} addr={}",
                device.bus, device.address
            )));
        } else if state.devices.is_empty() && !state.all_usb_devices.is_empty() {
            lines.push(Line::from("Device: no operation target"));
        } else if state.devices.is_empty() {
            lines.push(Line::from("Device: none visible"));
        } else {
            lines.push(Line::from("Device: select one first"));
        }
    }

    if let Some(raw_capture) = &state.raw_capture {
        lines.push(Line::from(vec![
            Span::styled("Raw export: ", theme.style(TuiStyleRole::Warning)),
            Span::raw("active"),
        ]));
        lines.push(Line::from(format!(
            "Raw file: {}",
            compact_path_label(raw_capture.path())
        )));
    }

    frame.render_widget(Clear, area);
    let block = pane_block(Pane::Status, state.focus, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(running) = &state.running_execution {
        let progress_height = if inner.height >= 8 {
            3
        } else if inner.height >= 6 {
            2
        } else {
            1
        };
        let context_height = inner.height.saturating_sub(progress_height);
        let status_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(context_height),
                Constraint::Length(progress_height),
            ])
            .split(inner);
        frame.render_widget(
            Paragraph::new(lines).style(theme.style(TuiStyleRole::Text)),
            status_areas[0],
        );
        render_timeout_budget_block(frame, status_areas[1], running, theme);
    } else {
        frame.render_widget(
            Paragraph::new(lines).style(theme.style(TuiStyleRole::Text)),
            inner,
        );
    }
}

fn viewed_log_status_label(state: &TuiState) -> &str {
    match state.status.as_str() {
        "Opened masked log." => "viewing log",
        "Failed to open log." => "failed",
        status => status,
    }
}

fn push_terminal_event_time_status_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &str,
    finished_at: &str,
    width: usize,
) {
    let line = format!("{label}: {finished_at}");
    if line.chars().count() <= width || finished_at.chars().count() > width {
        lines.push(Line::from(line));
    } else {
        lines.push(Line::from(format!("{label}:")));
        lines.push(Line::from(finished_at.to_owned()));
    }
}

fn minimum_result_status_line_count(active: &CommandStatus) -> usize {
    usize::from(active.status_summary_line().is_some())
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    if area.height == 0 {
        return;
    }
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(footer_text(state, area.width as usize))
            .style(theme.style(TuiStyleRole::Muted)),
        area,
    );
}

fn footer_text(state: &TuiState, width: usize) -> String {
    let segments = footer_segments(state);
    fit_footer_segments(&segments, width)
}

fn footer_segments(state: &TuiState) -> Vec<&'static str> {
    if state.show_help {
        return vec!["Esc Close", "? Close", "q Close"];
    }
    if state.action_menu.is_some() {
        return vec!["Enter Select", "Up/Down Move", "Esc Cancel"];
    }
    if state.confirmation.is_some() {
        return vec!["type risk Confirm", "Esc Cancel"];
    }
    if state.output_masking_ack_input.is_some() {
        return vec!["type unmask Disable", "Esc Cancel"];
    }
    if state.raw_log_path_input.is_some() {
        return vec!["Enter Continue", "Esc Cancel"];
    }
    if state.raw_log_ack_input.is_some() {
        return vec!["type raw-log Start", "Esc Cancel"];
    }
    if state.search_input.is_some() {
        return vec!["Enter Apply", "Esc Cancel"];
    }
    if state.edit_input.is_some() {
        return vec!["Enter Run", "Esc Cancel"];
    }
    if let Some(input) = &state.sequence_input {
        return match input.phase {
            SequenceInputPhase::Params => vec!["Enter Next/Run", "Up/Down Param", "Esc Cancel"],
            SequenceInputPhase::CandidateActionConfirmation => {
                vec!["type risk Run", "Esc Cancel"]
            }
            SequenceInputPhase::Confirmation => vec!["type risk Run", "Esc Cancel"],
        };
    }
    if state.ad_hoc_input.is_some() {
        return vec!["Enter Send", "Esc Cancel"];
    }
    if state.timeout_input.is_some() {
        return vec!["Enter Set", "default Reset", "Esc Cancel"];
    }
    if state.running_execution.is_some() {
        return vec!["Running", "? Help"];
    }

    match state.focus {
        Pane::Devices => vec!["Enter Select", "Tab Next", "? Help", "q Quit"],
        Pane::Categories => vec![
            "Enter Commands",
            "Up/Down Move",
            "Tab Next",
            "? Help",
            "q Quit",
        ],
        Pane::Commands => vec!["Enter Run", "/ Search", "Tab Next", "? Help", "q Quit"],
        Pane::Controls => vec!["Enter Use", "Up/Down Move", "Tab Next", "? Help", "q Quit"],
        Pane::Response => vec![
            "Enter Actions",
            "Up/Down Scroll",
            "PgUp/PgDn Page",
            "Tab Next",
            "? Help",
            "q Quit",
        ],
        Pane::History => vec![
            "Enter Actions",
            "Up/Down Move",
            "PgUp/PgDn Page",
            "? Help",
            "q Quit",
        ],
        Pane::Status => vec!["? Help", "q Quit"],
    }
}

fn fit_footer_segments(segments: &[&str], width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut output = String::new();
    for segment in segments {
        let next_len = if output.is_empty() {
            segment.len()
        } else {
            output.len() + 2 + segment.len()
        };
        if next_len > width {
            continue;
        }
        if !output.is_empty() {
            output.push_str("  ");
        }
        output.push_str(segment);
    }
    if output.is_empty() {
        segments
            .first()
            .map(|segment| segment.chars().take(width).collect())
            .unwrap_or_default()
    } else {
        output
    }
}

fn render_timeout_budget_block(
    frame: &mut Frame<'_>,
    area: Rect,
    running: &RunningExecution,
    theme: &TuiTheme,
) {
    let elapsed = running.started_at.elapsed();
    let timeout = running.timeout;
    let remaining = timeout.saturating_sub(elapsed);
    let ratio = if timeout.is_zero() {
        1.0
    } else {
        (elapsed.as_secs_f64() / timeout.as_secs_f64()).clamp(0.0, 1.0)
    };
    let label = timeout_budget_label(
        elapsed.as_secs(),
        timeout.as_secs(),
        remaining.as_secs(),
        area.width as usize,
    );

    if area.height == 0 {
        return;
    }

    if area.height == 1 {
        render_timeout_budget_bar(frame, area, ratio, Some(label), theme, true);
        return;
    }

    if area.height == 2 {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        frame.render_widget(
            Paragraph::new(label).style(theme.style(TuiStyleRole::Muted)),
            areas[0],
        );
        render_timeout_budget_bar(frame, areas[1], ratio, None, theme, true);
        return;
    }

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new("-".repeat(area.width as usize)).style(theme.style(TuiStyleRole::Muted)),
        areas[0],
    );
    frame.render_widget(
        Paragraph::new(label).style(theme.style(TuiStyleRole::Muted)),
        areas[1],
    );
    render_timeout_budget_bar(frame, areas[2], ratio, None, theme, true);
}

fn timeout_budget_label(
    elapsed_secs: u64,
    timeout_secs: u64,
    remaining_secs: u64,
    width: usize,
) -> String {
    let full = format!("Timeout {elapsed_secs}/{timeout_secs}s left {remaining_secs}s");
    if label_fits(&full, width) {
        return full;
    }

    let without_noun = format!("{elapsed_secs}/{timeout_secs}s left {remaining_secs}s");
    if label_fits(&without_noun, width) {
        return without_noun;
    }

    let minimal = format!("{elapsed_secs}/{timeout_secs}s");
    if label_fits(&minimal, width) {
        return minimal;
    }

    String::new()
}

fn label_fits(label: &str, width: usize) -> bool {
    !label.is_empty() && label.chars().count() <= width
}

fn render_timeout_budget_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    ratio: f64,
    label: Option<String>,
    theme: &TuiTheme,
    emphasized: bool,
) {
    let mut gauge = LineGauge::default()
        .ratio(ratio)
        .label(label.unwrap_or_default())
        .filled_style(theme.style(TuiStyleRole::Status))
        .unfilled_style(theme.style(TuiStyleRole::Muted));
    if emphasized {
        gauge = gauge
            .filled_symbol(symbols::block::FULL)
            .unfilled_symbol(symbols::shade::LIGHT);
    }
    frame.render_widget(gauge, area);
}

fn render_history(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let mut lines = vec![Line::from("Saved logs:")];
    if let Some(error) = &state.logs_error {
        lines.push(Line::from(Span::styled(
            error.clone(),
            theme.style(TuiStyleRole::Warning),
        )));
    }

    if state.logs.is_empty() {
        lines.push(Line::from("No logs found."));
    } else {
        let visible_slots = pane_inner_height(area).saturating_sub(lines.len()).max(1);
        let start = viewport_start(state.selected_log, state.logs.len(), visible_slots);
        lines.extend(
            state
                .logs
                .iter()
                .enumerate()
                .skip(start)
                .take(visible_slots)
                .map(|(index, log)| {
                    let prefix = if index == state.selected_log {
                        "> "
                    } else {
                        "  "
                    };
                    let style = if index == state.selected_log {
                        theme.style(TuiStyleRole::Selected)
                    } else {
                        theme.style(TuiStyleRole::Text)
                    };
                    Line::from(Span::styled(format!("{prefix}{}", log.label), style))
                }),
        );
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(Pane::History, state.focus, theme))
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, area: Rect, theme: &TuiTheme) {
    let text = vec![
        Line::from("atctl TUI"),
        Line::from(""),
        Line::from("Close help: Esc / ? / q"),
        Line::from(""),
        Line::from("Tab / Right   Next pane"),
        Line::from("Left          Previous pane"),
        Line::from("Up / Down     Move or scroll"),
        Line::from("PageUp/Down   Page move"),
        Line::from("Home / End    First or last"),
        Line::from("Enter         Use focused pane action"),
        Line::from("Esc           Cancel input/dialog"),
        Line::from("/             Search commands"),
        Line::from("?             Open help"),
        Line::from("q             Quit"),
        Line::from(""),
        Line::from("Risk labels: [safe] [sensitive] [write]"),
        Line::from("             [persistent] [dangerous] [unknown]"),
        Line::from("Confirm: write / persistent / dangerous / unknown"),
        Line::from("Unmasked Response: copy / export confirmation"),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text)),
        area,
    );
}

fn render_confirmation(
    frame: &mut Frame<'_>,
    area: Rect,
    confirmation: &ConfirmationState,
    theme: &TuiTheme,
) {
    let input = if confirmation.input.is_empty() {
        "<empty>".to_owned()
    } else {
        confirmation.input.clone()
    };
    let mut text = vec![
        Line::from("Command requires confirmation before sending."),
        Line::from(""),
        Line::from(format!("Name: {}", confirmation.preset.name)),
    ];
    if should_show_preset_source_detail(&confirmation.preset.origin) {
        text.push(Line::from(format!(
            "Source: {}",
            confirmation.preset.origin.label()
        )));
        if let Some(path) = confirmation.preset.origin.file_path() {
            text.push(Line::from(format!("File: {path}")));
            text.push(Line::from(EXTERNAL_DEFINITION_CONFIRMATION_NOTICE));
        }
    }
    text.extend([
        Line::from(format!("Command: {}", confirmation.preset.command)),
        risk_line("Risk: ", confirmation.preset.risk, theme),
        Line::from(format!(
            "Expected effect: {}",
            risk_expected_effect(confirmation.preset.risk)
        )),
        Line::from(""),
        Line::from(format!(
            "Type `{}` to send. Esc cancels.",
            confirmation.preset.risk
        )),
        Line::from(format!("Input: {input}")),
    ]);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Confirm")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Warning)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_output_masking_confirmation(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    theme: &TuiTheme,
) {
    let input = match &state.output_masking_ack_input {
        Some(input) if !input.is_empty() => input.clone(),
        _ => "<empty>".to_owned(),
    };
    let mut text = vec![
        Line::from(
            "This will show unmasked sensitive modem, subscriber, payload, message, credential, or TCP response values in the TUI Response display.",
        ),
        Line::from("Response copy and explicit export follow the visible Response display."),
        Line::from("Unmasked copy requires `copy`; unmasked export requires `export`."),
        Line::from("Generated history and session logs remain masked."),
        Line::from("Raw diagnostic export is separate and still requires raw-log acknowledgement."),
        Line::from(""),
        Line::from(
            "Scope: this TUI session until output masking is enabled again or the TUI exits.",
        ),
    ];
    text.extend([
        Line::from(""),
        Line::from(format!(
            "Type `{OUTPUT_UNMASK_ACK}` to disable output masking. Esc or q cancels."
        )),
        Line::from(format!("Input: {input}")),
    ]);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Disable output masking?")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Warning)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_response_action_confirmation(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    theme: &TuiTheme,
) {
    let Some(confirmation) = &state.response_action_confirmation else {
        return;
    };
    let input = if confirmation.input.is_empty() {
        "<empty>".to_owned()
    } else {
        confirmation.input.clone()
    };
    let (title, warning, response_label, target) = match &confirmation.action {
        ResponseActionConfirmation::Copy { response_label, .. } => (
            "Copy unmasked response?",
            "The terminal clipboard request will contain the unmasked Response.",
            response_label.as_str(),
            None,
        ),
        ResponseActionConfirmation::Export { request, path } => (
            "Export unmasked response?",
            "The file may contain unmasked identifiers, messages, payloads, or credentials.",
            request.response_label.as_str(),
            Some(path.as_path()),
        ),
    };
    let acknowledgement = response_action_confirmation_ack(&confirmation.action);
    let mut text = vec![
        Line::from(Span::styled(
            format!("Warning: {warning}"),
            theme.style(TuiStyleRole::Warning),
        )),
        Line::from(""),
        Line::from(format!("Response: {response_label}")),
    ];
    if let Some(path) = target {
        text.push(Line::from("Format: UTF-8 text"));
        text.push(Line::from(format!("File: {}", path.display())));
    }
    text.extend([
        Line::from(""),
        Line::from(format!(
            "Type `{acknowledgement}` to continue. Esc or q cancels."
        )),
        Line::from(format!("Input: {input}")),
    ]);
    if let Some(error) = &confirmation.error {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            error.clone(),
            theme.style(TuiStyleRole::Error),
        )));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Warning)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_raw_log_path_input(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TuiState,
    theme: &TuiTheme,
) {
    let Some(input_state) = &state.raw_log_path_input else {
        return;
    };
    let input = if input_state.input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.input.clone()
    };
    let mut text = vec![
        Line::from("Raw diagnostic export writes modem exchange bytes to a file."),
        Line::from("Choose a case-specific path. Existing files are refused."),
        Line::from(""),
        Line::from(format!("Path: {input}")),
    ];
    if let Some(error) = &input_state.error {
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            error.clone(),
            theme.style(TuiStyleRole::Error),
        )]));
    }
    text.push(Line::from(""));
    text.push(Line::from("Press Enter to continue. Esc or q cancels."));

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Raw diagnostic export path")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Warning)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_raw_log_ack_input(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(input_state) = &state.raw_log_ack_input else {
        return;
    };
    let input = if input_state.input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.input.clone()
    };
    let mut text = vec![
        Line::from(
            "This file may contain sensitive modem, subscriber, network, APN, or PDP authentication values.",
        ),
        Line::from(
            "Generated history and session logs remain masked; Response export follows the visible masking state.",
        ),
        Line::from("Capture applies only to commands executed after it starts."),
        Line::from(""),
        Line::from(format!("File: {}", input_state.path.display())),
        Line::from(""),
        Line::from(format!(
            "Type `{RAW_LOG_ACK}` to start raw diagnostic export. Esc or q cancels."
        )),
        Line::from(format!("Input: {input}")),
    ];
    if let Some(error) = &input_state.error {
        text.push(Line::from(""));
        text.push(Line::from(vec![Span::styled(
            error.clone(),
            theme.style(TuiStyleRole::Error),
        )]));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Start raw diagnostic export?")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Warning)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_search_input(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(input_state) = &state.search_input else {
        return;
    };
    let input = if input_state.input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.input.clone()
    };
    let filter_hint = if has_file_preset_sets(state) || has_file_sequence_sets(state) {
        "Filter visible commands and Sequences by name, command text, summary, category, or set."
    } else {
        "Filter visible commands and Sequences by name, command text, summary, or category."
    };
    let text = vec![
        Line::from(filter_hint),
        Line::from("Empty input clears the filter."),
        Line::from(""),
        Line::from(format!("Search: {input}")),
        Line::from(""),
        Line::from("Enter applies. Esc cancels."),
    ];

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Commands / Sequences search")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_edit_input(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(input_state) = &state.edit_input else {
        return;
    };
    let input = if input_state.input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.input.clone()
    };
    let mut text = vec![
        Line::from("Edit the selected command before execution."),
        Line::from("The edited command will be classified before USB access."),
        Line::from(""),
        Line::from(format!("Command: {input}")),
        Line::from(""),
        Line::from("Enter sends or opens confirmation. Esc cancels."),
    ];
    if let Some(error) = &input_state.error {
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled("Error: ", theme.style(TuiStyleRole::Error)),
            Span::raw(error.clone()),
        ]));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Edit before run")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_sequence_input(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(input_state) = &state.sequence_input else {
        return;
    };
    let sequence = &input_state.sequence;
    let mut text = Vec::new();

    match input_state.phase {
        SequenceInputPhase::Params => {
            text.extend([
                Line::from("Run Sequence."),
                Line::from(""),
                Line::from(format!("Name: {}", sequence.name)),
                Line::from(format!("Summary: {}", sequence.summary)),
            ]);
            if should_show_sequence_source_detail(&sequence.origin) {
                text.push(Line::from(format!("Source: {}", sequence.origin.label())));
            }
            text.push(risk_line("Risk: ", sequence.risk, theme));
            text.push(Line::from(format!(
                "Required: {}",
                required_param_summary(sequence)
            )));
            text.push(Line::from(""));
            text.push(Line::from("Values:"));
            for (index, param) in sequence.params.iter().enumerate() {
                let marker = if index == input_state.active_param {
                    "> "
                } else {
                    "  "
                };
                let value = input_state
                    .values
                    .get(index)
                    .map(String::as_str)
                    .unwrap_or("");
                text.push(Line::from(format!(
                    "{marker}{}: {}  {}",
                    sequence_param_label(param),
                    sequence_param_value_display(param, value),
                    sequence_param_state_label(param, value)
                )));
                if index == input_state.active_param
                    && let Some(hint) = sequence_param_resolution_hint(param)
                {
                    text.push(Line::from(format!("    {hint}")));
                }
                if index == input_state.active_param
                    && let Some(candidate) =
                        sequence_param_candidate_source(&input_state.sequence, param)
                {
                    push_sequence_resolution_lines(
                        &mut text,
                        input_state,
                        state,
                        candidate,
                        state.output_masking_enabled,
                    );
                }
            }
            if !sequence.before_running.is_empty() {
                text.push(Line::from(""));
                text.push(Line::from("Before running:"));
                for item in &sequence.before_running {
                    text.push(Line::from(format!("  {item}")));
                }
            }
            text.push(Line::from(""));
            text.push(Line::from(sequence_input_footer(input_state, state)));
        }
        SequenceInputPhase::CandidateActionConfirmation => {
            text = sequence_candidate_action_confirmation_lines(input_state, theme);
        }
        SequenceInputPhase::Confirmation => {
            text = sequence_confirmation_lines(input_state, theme, area);
        }
    }

    if let Some(error) = &input_state.error {
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled("Error: ", theme.style(TuiStyleRole::Error)),
            Span::raw(error.clone()),
        ]));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Run Sequence")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn sequence_candidate_action_confirmation_lines(
    input_state: &SequenceInputState,
    theme: &TuiTheme,
) -> Vec<Line<'static>> {
    let Some(action) = input_state.pending_candidate_action else {
        return vec![
            Line::from("Run Sequence."),
            Line::from(""),
            Line::from("No candidate action is selected."),
        ];
    };
    let item = candidate_action_item(action);
    let risk = item.risk();
    let input = if input_state.confirmation_input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.confirmation_input.clone()
    };
    vec![
        Line::from("Run Sequence."),
        Line::from(""),
        Line::from(format!("Name: {}", input_state.sequence.name)),
        Line::from(""),
        Line::from("Confirm action."),
        Line::from(format!("Action: {}", action.label)),
        Line::from(format!("Command: {}", action.command)),
        risk_line("Risk: ", risk, theme),
        Line::from(format!("Expected effect: {}", risk_expected_effect(risk))),
        Line::from(""),
        Line::from(format!("Type `{risk}` to run.")),
        Line::from("Esc cancels."),
        Line::from(format!("Input: {input}")),
    ]
}

fn sequence_confirmation_lines(
    input_state: &SequenceInputState,
    theme: &TuiTheme,
    area: Rect,
) -> Vec<Line<'static>> {
    let sequence = &input_state.sequence;
    let mut detail = vec![
        Line::from("Run Sequence."),
        Line::from(""),
        Line::from(format!("Name: {}", sequence.name)),
    ];
    if should_show_sequence_source_detail(&sequence.origin) {
        detail.push(Line::from(format!("Source: {}", sequence.origin.label())));
        if let Some(path) = sequence.origin.file_path() {
            detail.push(Line::from(format!("File: {path}")));
            detail.push(Line::from(EXTERNAL_DEFINITION_CONFIRMATION_NOTICE));
        }
    }
    detail.push(risk_line("Risk: ", sequence.risk, theme));

    if !sequence.params.is_empty() {
        detail.push(Line::from(""));
        detail.push(Line::from("Values:"));
        for (index, param) in sequence.params.iter().enumerate() {
            let value = input_state
                .values
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            detail.push(Line::from(format!(
                "  {}: {}  {}",
                sequence_param_label(param),
                sequence_param_value_display(param, value),
                sequence_param_state_label(param, value)
            )));
        }
    }

    if let Ok(values) = sequence_input_param_values(input_state)
        && let Ok(review) = render_sequence_review(sequence, &values)
        && !review.is_empty()
    {
        detail.push(Line::from(""));
        detail.push(Line::from("Review:"));
        for item in review {
            let sensitive = if item.sensitive { " (sensitive)" } else { "" };
            detail.push(Line::from(format!(
                "  {}{}: {}",
                item.label, sensitive, item.value
            )));
        }
    }

    let input = if input_state.confirmation_input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.confirmation_input.clone()
    };
    let mut pinned = vec![
        Line::from(""),
        Line::from(format!(
            "Expected effect: {}",
            risk_expected_effect(sequence.risk)
        )),
        Line::from(""),
    ];
    if sequence.risk.requires_confirmation() {
        pinned.push(Line::from(format!("Type `{}` to run.", sequence.risk)));
    } else {
        pinned.push(Line::from("Press Enter to run."));
    }
    pinned.push(Line::from("Esc cancels."));
    pinned.push(Line::from(format!("Input: {input}")));

    fit_sequence_confirmation_lines(detail, pinned, area.height.saturating_sub(2) as usize)
}

fn fit_sequence_confirmation_lines(
    mut detail: Vec<Line<'static>>,
    pinned: Vec<Line<'static>>,
    visible_rows: usize,
) -> Vec<Line<'static>> {
    if visible_rows == 0 {
        return Vec::new();
    }
    if detail.len() + pinned.len() <= visible_rows {
        detail.extend(pinned);
        return detail;
    }
    if pinned.len() >= visible_rows {
        return pinned.into_iter().take(visible_rows).collect();
    }

    let omission = Line::from("  ... detail omitted to keep confirmation input visible.");
    let detail_rows = visible_rows.saturating_sub(pinned.len() + 1);
    detail.truncate(detail_rows);
    detail.push(omission);
    detail.extend(pinned);
    detail
}

fn sequence_param_label(param: &SequenceParam) -> String {
    if param.sensitive {
        format!("{} (sensitive)", param.label)
    } else {
        param.label.clone()
    }
}

fn sequence_input_footer(input: &SequenceInputState, state: &TuiState) -> &'static str {
    if let Some(candidate) = active_sequence_candidate_source_for_input(input)
        && candidate_set_for_source(state, candidate).is_some_and(|set| !set.is_empty())
    {
        if candidate_actions(candidate).is_empty() {
            "Enter selects or advances. Up/Down selects candidate. Tab changes value. Type edits. Esc cancels."
        } else {
            "Enter selects or runs action. Up/Down selects candidate/action. Tab changes value. Type edits. Esc cancels."
        }
    } else if active_sequence_candidate_source_for_input(input)
        .is_some_and(|candidate| !candidate_actions(candidate).is_empty())
    {
        "Enter advances or runs selected action. Up/Down selects action. Tab changes value. Type edits. Esc cancels."
    } else {
        "Enter advances or runs. Up/Down changes value. Esc cancels."
    }
}

fn active_sequence_candidate_source(state: &TuiState) -> Option<SequenceCandidateSource> {
    state
        .sequence_input
        .as_ref()
        .and_then(active_sequence_candidate_source_for_input)
}

fn active_sequence_candidate_source_for_input(
    input: &SequenceInputState,
) -> Option<SequenceCandidateSource> {
    input
        .sequence
        .params
        .get(input.active_param)
        .and_then(|param| sequence_param_candidate_source(&input.sequence, param))
}

fn sequence_param_candidate_source(
    sequence: &Sequence,
    param: &SequenceParam,
) -> Option<SequenceCandidateSource> {
    param.candidate.or_else(|| {
        (param.source == SequenceParamSource::Select
            && param.name == "index"
            && sequence.categories.iter().any(|category| category == "sms"))
        .then_some(SequenceCandidateSource::SmsMessage)
    })
}

fn candidate_set_for_source(
    state: &TuiState,
    candidate: SequenceCandidateSource,
) -> Option<&TuiSequenceCandidateSet> {
    state
        .sequence_candidate_sets
        .iter()
        .find(|set| set.candidate == candidate)
}

#[derive(Debug, Copy, Clone)]
struct SequenceCandidateAction {
    label: &'static str,
    command: &'static str,
    categories: &'static [&'static str],
}

fn candidate_actions(candidate: SequenceCandidateSource) -> Vec<SequenceCandidateAction> {
    match candidate {
        SequenceCandidateSource::SmsMessage => vec![SequenceCandidateAction {
            label: "Load received SMS list",
            command: "AT+CMGL=\"ALL\"",
            categories: &["sms"],
        }],
        SequenceCandidateSource::PdpContext => vec![
            SequenceCandidateAction {
                label: "Load active PDP contexts",
                command: "AT+CGACT?",
                categories: &["pdp"],
            },
            SequenceCandidateAction {
                label: "Load PDP context definitions",
                command: "AT+CGDCONT?",
                categories: &["pdp", "apn"],
            },
        ],
    }
}

fn push_sequence_resolution_lines(
    text: &mut Vec<Line<'static>>,
    input: &SequenceInputState,
    state: &TuiState,
    candidate: SequenceCandidateSource,
    output_masking_enabled: bool,
) {
    let Some(candidate_set) = candidate_set_for_source(state, candidate) else {
        push_sequence_candidate_action_lines(text, input, candidate);
        return;
    };
    if candidate_set.is_empty() {
        push_sequence_candidate_action_lines(text, input, candidate);
        return;
    }

    let candidates = &candidate_set.candidates;
    let visible_limit = SEQUENCE_CANDIDATE_VISIBLE_ROWS;
    let visible_start = sequence_candidate_visible_start(input.active_candidate, candidates.len());
    let visible_end = (visible_start + visible_limit).min(candidates.len());
    text.push(Line::from(format!(
        "    Candidates: {} ({} total, acquired {})",
        candidate_set.source_label,
        candidates.len(),
        candidate_set.acquired_at
    )));
    text.push(Line::from("    No modem read is performed by this modal."));
    if candidates.len() > visible_limit {
        text.push(Line::from(format!(
            "    Candidate rows {}-{} of {}",
            visible_start + 1,
            visible_end,
            candidates.len()
        )));
    }
    for (index, candidate) in candidates
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(visible_limit)
    {
        let marker = if index == input.active_candidate {
            "> "
        } else {
            "  "
        };
        text.push(Line::from(format!(
            "    {marker}{}",
            sequence_candidate_label(candidate, output_masking_enabled)
        )));
    }
    let actions = candidate_actions(candidate);
    if !actions.is_empty() {
        text.push(Line::from("    Actions:"));
        push_sequence_candidate_action_rows(text, input, &actions, candidates.len());
    }
}

fn push_sequence_candidate_action_lines(
    text: &mut Vec<Line<'static>>,
    input: &SequenceInputState,
    candidate: SequenceCandidateSource,
) {
    text.push(Line::from("    Candidates: none loaded"));
    let actions = candidate_actions(candidate);
    if actions.is_empty() {
        return;
    }
    text.push(Line::from("    Actions:"));
    push_sequence_candidate_action_rows(text, input, &actions, 1);
}

fn push_sequence_candidate_action_rows(
    text: &mut Vec<Line<'static>>,
    input: &SequenceInputState,
    actions: &[SequenceCandidateAction],
    action_start_index: usize,
) {
    for (index, action) in actions.iter().enumerate() {
        let marker = if input.active_candidate == action_start_index + index {
            "> "
        } else {
            "  "
        };
        text.push(Line::from(format!(
            "    {marker}{}  {}",
            action.label, action.command
        )));
    }
}

fn sequence_candidate_visible_start(active_candidate: usize, candidate_count: usize) -> usize {
    let visible_limit = SEQUENCE_CANDIDATE_VISIBLE_ROWS;
    let selected_candidate = active_candidate.min(candidate_count.saturating_sub(1));
    if candidate_count <= visible_limit || selected_candidate < visible_limit {
        0
    } else {
        (selected_candidate + 1).saturating_sub(visible_limit)
    }
}

const SEQUENCE_CANDIDATE_VISIBLE_ROWS: usize = 5;

fn sequence_candidate_label(
    candidate: &SequenceValueCandidate,
    output_masking_enabled: bool,
) -> &str {
    if output_masking_enabled {
        &candidate.masked_label
    } else {
        &candidate.raw_label
    }
}

fn sequence_param_value_display(param: &SequenceParam, value: &str) -> String {
    if value.is_empty() {
        if param.source == SequenceParamSource::User {
            "<empty>".to_owned()
        } else {
            "unresolved".to_owned()
        }
    } else {
        value.to_owned()
    }
}

fn sequence_param_state_label(param: &SequenceParam, value: &str) -> &'static str {
    if value.is_empty() {
        param.source.label()
    } else if param
        .default_value
        .as_deref()
        .is_some_and(|default| default == value)
    {
        "default"
    } else {
        param.source.label()
    }
}

fn sequence_param_resolution_hint(param: &SequenceParam) -> Option<String> {
    if let Some(hint) = &param.hint {
        return Some(hint.clone());
    }
    param
        .default_value
        .as_ref()
        .map(|default| format!("Default value: {default}."))
}

fn render_ad_hoc_input(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(input_state) = &state.ad_hoc_input else {
        return;
    };
    let input = if input_state.input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.input.clone()
    };
    let mut text = vec![
        Line::from("Enter a one-shot AT command."),
        Line::from("The command will be classified before USB access."),
        Line::from(""),
        Line::from(format!("Command: {input}")),
        Line::from(""),
        Line::from("Enter sends or opens confirmation. Esc cancels."),
    ];
    if let Some(error) = &input_state.error {
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled("Error: ", theme.style(TuiStyleRole::Error)),
            Span::raw(error.clone()),
        ]));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("AT command")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_timeout_input(frame: &mut Frame<'_>, area: Rect, state: &TuiState, theme: &TuiTheme) {
    let Some(input_state) = &state.timeout_input else {
        return;
    };
    let input = if input_state.input.is_empty() {
        "<empty>".to_owned()
    } else {
        input_state.input.clone()
    };
    let mut text = vec![
        Line::from("Set the temporary TUI command timeout in seconds."),
        Line::from("Enter `default` to clear the temporary override."),
        Line::from(""),
        Line::from(format!("Timeout: {input}")),
        Line::from(""),
        Line::from("Enter applies. Esc cancels."),
    ];
    if let Some(error) = &input_state.error {
        text.push(Line::from(""));
        text.push(Line::from(vec![
            Span::styled("Error: ", theme.style(TuiStyleRole::Error)),
            Span::raw(error.clone()),
        ]));
    }

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Command timeout")
                    .borders(Borders::ALL)
                    .border_style(theme.style(TuiStyleRole::Focus)),
            )
            .alignment(Alignment::Left)
            .style(theme.style(TuiStyleRole::Text))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn pane_block(pane: Pane, focus: Pane, theme: &TuiTheme) -> Block<'static> {
    pane_block_with_title(pane, focus, theme, pane.title().to_owned())
}

fn pane_block_with_title(
    pane: Pane,
    focus: Pane,
    theme: &TuiTheme,
    title: String,
) -> Block<'static> {
    let style = if pane == focus {
        theme.style(TuiStyleRole::Focus)
    } else {
        theme.style(TuiStyleRole::Text)
    };
    Block::default()
        .title(Span::styled(title, style))
        .borders(Borders::ALL)
        .style(theme.style(TuiStyleRole::Text))
        .border_style(style)
}

fn selected_item(selected: bool, value: &str, theme: &TuiTheme) -> ListItem<'static> {
    if selected {
        ListItem::new(format!("> {value}")).style(theme.style(TuiStyleRole::Selected))
    } else {
        ListItem::new(format!("  {value}"))
    }
}

fn command_row_item(
    row: &CommandListRow<'_>,
    selected_command: usize,
    theme: &TuiTheme,
) -> ListItem<'static> {
    match row {
        CommandListRow::BlankSeparator => ListItem::new(""),
        CommandListRow::KindHeader(label) => ListItem::new(Line::from(Span::styled(
            *label,
            theme.style(TuiStyleRole::Muted),
        ))),
        CommandListRow::SourceHeader(label) => ListItem::new(Line::from(Span::styled(
            label.clone(),
            theme.style(TuiStyleRole::Muted),
        ))),
        CommandListRow::Command {
            command_index,
            command,
        } => command_item(*command_index == selected_command, command, theme),
    }
}

fn command_item(selected: bool, command: &ExecutableItem, theme: &TuiTheme) -> ListItem<'static> {
    let prefix = if selected { "> " } else { "  " };
    let name_style = if selected {
        theme.style(TuiStyleRole::Selected)
    } else {
        theme.style(TuiStyleRole::Text)
    };
    let mut spans = vec![Span::styled(
        format!("{prefix}{}", command.name()),
        name_style,
    )];
    spans.push(Span::raw(" "));
    spans.push(risk_span(command.risk(), theme));
    if let Some(command_text) = command.command_text() {
        spans.push(Span::styled(
            format!(" {command_text}"),
            theme.style(TuiStyleRole::Text),
        ));
    } else if let Some(summary) = command.summary() {
        spans.push(Span::styled(
            format!(" {summary}"),
            theme.style(TuiStyleRole::Text),
        ));
    }
    ListItem::new(Line::from(spans))
}

fn risk_line(
    prefix: &'static str,
    risk: crate::at::risk::RiskLevel,
    theme: &TuiTheme,
) -> Line<'static> {
    Line::from(vec![Span::raw(prefix), risk_span(risk, theme)])
}

fn risk_span(risk: crate::at::risk::RiskLevel, theme: &TuiTheme) -> Span<'static> {
    Span::styled(format!("[{risk}]"), theme.risk_style(risk))
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn terminal_error(error: io::Error) -> AtctlError {
    AtctlError::Transport(format!("terminal error: {error}"))
}

#[cfg(test)]
mod tests;
