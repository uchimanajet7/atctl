use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::prelude::{Color, Modifier};
use ratatui::symbols;

use crate::at::response::AtStatus;
use crate::at::review::{DataHandling, review_direct_command};
use crate::cli::TuiThemeChoice;
use crate::sequences::engine::SmsMessageCandidate;
use crate::sequences::model::{SequenceStep, StepTerminator};

use super::theme::{REQUIRED_STYLE_ROLES, TuiThemeMode};
use super::*;

#[test]
fn categories_include_all_and_categories() {
    let state = test_state();

    assert_eq!(state.categories[0], "all");
    assert!(state.categories.iter().any(|category| category == "basic"));
    assert!(state.categories.iter().any(|category| category == "sim"));
    assert!(!state.categories.iter().any(|category| category == "danger"));
}

#[test]
fn key_handling_toggles_help_and_quits() {
    let mut state = test_state();

    assert_eq!(
        handle_key_code(&mut state, KeyCode::Esc),
        TuiAction::Continue
    );
    assert!(!state.show_help);
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('?')),
        TuiAction::Continue
    );
    assert!(state.show_help);
    let selected_command = state.selected_command;
    let status = state.status.clone();
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('q')),
        TuiAction::Continue
    );
    assert!(state.show_help);
    assert_eq!(state.selected_command, selected_command);
    assert_eq!(state.status, status);
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Esc),
        TuiAction::Continue
    );
    assert!(!state.show_help);
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('q')),
        TuiAction::Quit
    );

    let mut help_closed_by_toggle = test_state();
    handle_key_code(&mut help_closed_by_toggle, KeyCode::Char('?'));
    handle_key_code(&mut help_closed_by_toggle, KeyCode::Char('?'));
    assert!(!help_closed_by_toggle.show_help);
}

#[test]
fn q_is_text_in_text_and_acknowledgement_inputs() {
    let mut state = test_state();

    handle_key_code(&mut state, KeyCode::Char('/'));
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .search_input
            .as_ref()
            .map(|input| input.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.edit_input = Some(EditInputState {
        input: String::new(),
        error: None,
        timeout_secs: DEFAULT_COMMAND_TIMEOUT_SECS,
    });
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state.edit_input.as_ref().map(|input| input.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.timeout_input = Some(TimeoutInputState {
        input: String::new(),
        error: None,
    });
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .timeout_input
            .as_ref()
            .map(|input| input.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.output_masking_ack_input = Some(String::new());
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(state.output_masking_ack_input.as_deref(), Some("q"));
    handle_key_code(&mut state, KeyCode::Esc);

    state.response_action_confirmation = Some(ResponseActionConfirmationState {
        action: ResponseActionConfirmation::Copy {
            contents: "response".to_owned(),
            response_label: "Response".to_owned(),
        },
        input: String::new(),
        error: None,
    });
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .response_action_confirmation
            .as_ref()
            .map(|confirmation| confirmation.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.raw_log_path_input = Some(RawLogPathInputState::default());
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .raw_log_path_input
            .as_ref()
            .map(|input| input.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.raw_log_ack_input = Some(RawLogAckInputState {
        path: PathBuf::from("raw.log"),
        input: String::new(),
        error: None,
    });
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .raw_log_ack_input
            .as_ref()
            .map(|input| input.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.ad_hoc_input = Some(AdHocInputState::default());
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .ad_hoc_input
            .as_ref()
            .map(|input| input.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Esc);

    state.selected_command = 1;
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .confirmation
            .as_ref()
            .map(|confirmation| confirmation.input.as_str()),
        Some("q")
    );

    let mut sequence_state = test_state_with_sequences();
    sequence_state.selected_category = sequence_state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    handle_key_code(&mut sequence_state, KeyCode::Enter);
    handle_key_code(&mut sequence_state, KeyCode::Char('q'));
    assert_eq!(
        sequence_state
            .sequence_input
            .as_ref()
            .and_then(|input| input.values.first())
            .map(String::as_str),
        Some("q")
    );
}

#[test]
fn help_modal_blocks_underlying_actions() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Char('?'));
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert!(state.show_help);
    assert!(executor.calls.is_empty());
    assert!(state.pending_execution.is_none());
}

#[test]
fn help_shows_complete_confirmation_grammar() {
    let mut state = test_state();
    handle_key_code(&mut state, KeyCode::Char('?'));

    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Rows: [confirm] opens confirmation before execution"));
    assert!(buffer.contains("Confirm known operation: run"));
    assert!(buffer.contains("Confirm unknown operation: unknown"));
    assert!(buffer.contains("Unmasked Response: copy / export confirmation"));
}

#[test]
fn status_does_not_render_keyboard_hint_boilerplate() {
    let mut state = test_state();

    let buffer = rendered_buffer(&mut state, 120, 32);

    assert!(!buffer.contains("Keys:"));
    assert!(buffer.contains("Enter Run"));
    assert!(buffer.contains("? Help"));
    assert!(!buffer.contains("t Timeout"));
}

#[test]
fn footer_hints_are_context_sensitive() {
    let mut state = test_state();

    state.focus = Pane::Devices;
    assert!(footer_text(&state, 80).contains("Enter Select"));

    state.focus = Pane::Response;
    assert!(footer_text(&state, 80).contains("Enter Actions"));
    assert!(footer_text(&state, 80).contains("Up/Down Scroll"));

    state.focus = Pane::Controls;
    assert!(footer_text(&state, 80).contains("Enter Use"));

    state.focus = Pane::History;
    assert!(footer_text(&state, 80).contains("Enter Actions"));
}

#[test]
fn non_text_action_menu_ignores_q_and_uses_esc_cancel() {
    let mut state = test_state();
    state.response = ResponseState::masked("OK");
    state.focus = Pane::Response;

    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.action_menu.is_some());
    let selected = state.action_menu.as_ref().map(|menu| menu.selected);
    let status = state.status.clone();
    assert!(footer_text(&state, 80).contains("Esc Cancel"));
    assert!(!footer_text(&state, 80).contains('q'));
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert!(state.action_menu.is_some());
    assert_eq!(
        state.action_menu.as_ref().map(|menu| menu.selected),
        selected
    );
    assert_eq!(state.status, status);
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.action_menu.is_none());
}

#[test]
fn escape_dismisses_every_atctl_owned_modal_state() {
    let mut state = test_state();

    state.show_help = true;
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(!state.show_help);

    state.response = ResponseState::masked("OK");
    state.focus = Pane::Response;
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.action_menu.is_none());

    state.output_masking_ack_input = Some(String::new());
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.output_masking_ack_input.is_none());

    state.response_action_confirmation = Some(ResponseActionConfirmationState {
        action: ResponseActionConfirmation::Copy {
            contents: "response".to_owned(),
            response_label: "Response".to_owned(),
        },
        input: String::new(),
        error: None,
    });
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.response_action_confirmation.is_none());

    state.raw_log_path_input = Some(RawLogPathInputState::default());
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.raw_log_path_input.is_none());

    state.raw_log_ack_input = Some(RawLogAckInputState {
        path: PathBuf::from("raw.log"),
        input: String::new(),
        error: None,
    });
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.raw_log_ack_input.is_none());

    state.search_input = Some(SearchInputState::default());
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.search_input.is_none());

    state.edit_input = Some(EditInputState {
        input: "AT".to_owned(),
        error: None,
        timeout_secs: DEFAULT_COMMAND_TIMEOUT_SECS,
    });
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.edit_input.is_none());

    state.timeout_input = Some(TimeoutInputState {
        input: "120".to_owned(),
        error: None,
    });
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.timeout_input.is_none());

    state.ad_hoc_input = Some(AdHocInputState::default());
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.ad_hoc_input.is_none());

    state.confirmation = Some(ConfirmationState::new(test_commands()[1].clone()));
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.confirmation.is_none());

    let mut sequence_state = test_state_with_sequences();
    sequence_state.selected_category = sequence_state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    handle_key_code(&mut sequence_state, KeyCode::Enter);
    assert!(sequence_state.sequence_input.is_some());
    handle_key_code(&mut sequence_state, KeyCode::Esc);
    assert!(sequence_state.sequence_input.is_none());
}

#[test]
fn footer_omits_low_priority_hints_instead_of_wrapping() {
    let mut state = test_state();
    state.focus = Pane::Commands;

    let footer = footer_text(&state, 22);

    assert!(footer.contains("Enter Run"));
    assert!(footer.len() <= 22);
    assert!(!footer.contains("q Quit"));
}

#[test]
fn layout_uses_aligned_bands_compact_utility_width_and_commands_next_focus() {
    let mut state = test_state();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let (devices_x, devices_y) = buffer_line_position(buffer, "Devices").unwrap();
    let (status_x, status_y) = buffer_line_position(buffer, "Status").unwrap();
    let (controls_x, controls_y) = buffer_line_position(buffer, "Controls").unwrap();
    let (categories_x, categories_y) = buffer_line_position(buffer, "Categories").unwrap();
    let (commands_x, commands_y) = buffer_line_position(buffer, "Commands").unwrap();
    let (response_x, response_y) = buffer_line_position(buffer, "Response").unwrap();
    let (logs_x, logs_y) = buffer_line_position(buffer, "Logs").unwrap();

    assert_eq!(devices_y, categories_y);
    assert_eq!(categories_y, commands_y);
    assert_eq!(devices_x, status_x);
    assert_eq!(devices_x, controls_x);
    assert!(status_y > devices_y);
    assert!(controls_y > status_y);
    assert!(categories_x > controls_x);
    assert!(commands_x > categories_x);
    assert!(controls_y > commands_y);
    assert_eq!(controls_y, response_y);
    assert_eq!(response_y, logs_y);
    assert!(logs_x > response_x);

    let utility_width = categories_x - devices_x;
    let category_width = commands_x - categories_x;
    let commands_width = buffer.area.width - commands_x;
    let response_width = logs_x - response_x;
    assert!(
        utility_width >= 30,
        "utility_width={utility_width}, devices_x={devices_x}, categories_x={categories_x}"
    );
    assert!(
        utility_width <= 36,
        "utility_width={utility_width}, devices_x={devices_x}, categories_x={categories_x}"
    );
    assert!(
        category_width <= 28,
        "category_width={category_width}, categories_x={categories_x}, commands_x={commands_x}"
    );
    assert!(
        commands_width > utility_width,
        "commands_width={commands_width}, utility_width={utility_width}"
    );
    assert!(
        response_width > utility_width,
        "response_width={response_width}, utility_width={utility_width}"
    );
    assert!(
        state.response_visible_height >= state.commands_visible_height,
        "response_visible_height={}, commands_visible_height={}",
        state.response_visible_height,
        state.commands_visible_height
    );

    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::Controls);
}

#[test]
fn controls_render_as_actions_not_status_table() {
    let mut state = test_state();
    state.focus = Pane::Controls;
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("AT command"));
    assert!(buffer.contains("Edit selected"));
    assert!(buffer.contains("Timeout"));
    assert!(!buffer.contains("Timeout 30s"));
    assert!(buffer.contains("Start raw export"));
    assert!(buffer.contains("Output masking on"));
    assert!(!buffer.contains("Rerun last"));
    assert!(!buffer.contains("Save response"));
    assert!(!buffer.contains("Export response"));
    assert!(!buffer.contains("Copy response"));
    assert!(!buffer.contains("Clear response"));
    assert!(!buffer.contains("Copy log path"));
    assert!(!buffer.contains("Copy response sent"));
    assert!(!buffer.contains("Save response saved"));
    assert!(!buffer.contains(" avail"));
    assert!(!buffer.contains(" no resp"));
    assert!(!buffer.contains(" sel dev"));
}

#[test]
fn controls_feedback_is_near_controls_without_changing_action_label() {
    let mut state = test_state();
    state.focus = Pane::Controls;

    set_controls_feedback(&mut state, TuiStyleRole::Status, COPY_REQUEST_SENT_FEEDBACK);
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Output masking"));
    assert!(buffer.contains(COPY_REQUEST_SENT_FEEDBACK));
    assert!(!buffer.contains("Copy response sent"));
    assert!(!buffer.contains("Copy resp sent"));
}

#[test]
fn response_enter_renders_response_action_menu() {
    let mut state = test_state();
    state.focus = Pane::Response;

    handle_key_code(&mut state, KeyCode::Enter);
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Response actions"));
    assert!(buffer.contains("Copy response"));
    assert!(buffer.contains("Export response..."));
    assert!(buffer.contains("Export format: UTF-8 text"));
    assert!(buffer.contains("Export content: masked"));
    assert!(buffer.contains("File name:"));
    assert!(!buffer.contains("Open saved responses folder"));
    assert!(!buffer.contains("Reveal in Finder"));
    assert!(buffer.contains("Clear response"));
    assert!(!buffer.contains("Copy saved response path"));
    assert!(!buffer.contains("Open saved response dir"));
}

#[test]
fn logs_enter_renders_log_action_menu() {
    let paths = test_logging_paths("log-actions");
    fs::write(
        paths.session_dir.join("2026-07-03T00-00-00Z.session.log"),
        "session",
    )
    .unwrap();
    let mut state = test_state();
    state.log_paths = paths;
    refresh_log_summaries(&mut state).unwrap();
    state.focus = Pane::History;

    handle_key_code(&mut state, KeyCode::Enter);
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Log actions"));
    assert!(buffer.contains("Open log in Response"));
    assert!(buffer.contains("Reveal in Finder"));
    assert!(buffer.contains("Log: session:"));
    assert!(buffer.contains("2026-07-03T00-00-00Z.session.log"));
    assert!(!buffer.contains("Open logs folder"));
    assert!(!buffer.contains("    Folder:"));
    assert!(!buffer.contains("Copy log path"));
    assert!(!buffer.contains("Copy log dir"));
    assert!(!buffer.contains("Open log dir"));
}

#[test]
fn response_enter_while_viewing_log_renders_log_view_actions() {
    let paths = test_logging_paths("viewed-log-actions");
    let session_log = paths.session_dir.join("test.session.log");
    fs::write(&session_log, "masked log body").unwrap();
    let mut state = test_state();
    state.log_paths = paths;
    state.focus = Pane::Response;
    state.response = ResponseState::masked("masked log body");
    state.active_command = None;
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: session_log.clone(),
        label: "test.session.log".to_owned(),
    });

    handle_key_code(&mut state, KeyCode::Enter);
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Log view actions"));
    assert!(buffer.contains("Copy displayed log"));
    assert!(buffer.contains("Reveal in Finder"));
    assert!(buffer.contains("Close log view"));
    assert!(buffer.contains("Log: test.session.log"));
    assert!(buffer.contains("test.session.log"));
    assert!(!buffer.contains("Open logs folder"));
    assert!(!buffer.contains("Save response"));
    assert!(!buffer.contains("Export response"));
    assert!(!buffer.contains("Clear response"));
}

#[test]
fn built_in_only_tui_does_not_show_preset_set_labels() {
    let mut state = test_state();

    let buffer = rendered_buffer(&mut state, 120, 32);

    assert!(!buffer.contains("[core]"));
    assert!(!buffer.contains("Built-in presets"));
    assert!(!buffer.contains("Product presets"));
    assert!(!buffer.contains("Source: Built-in presets"));
    assert!(!buffer.contains("Source: Product presets"));

    state.selected_command = 1;
    handle_key_code(&mut state, KeyCode::Enter);
    let confirmation_buffer = rendered_buffer(&mut state, 120, 32);

    assert!(confirmation_buffer.contains("Confirm command"));
    assert!(confirmation_buffer.contains("Name: disable-command-echo"));
    assert!(confirmation_buffer.contains("Command: ATE0"));
    assert!(!confirmation_buffer.contains("[core]"));
    assert!(!confirmation_buffer.contains("Built-in presets"));
    assert!(!confirmation_buffer.contains("Product presets"));
    assert!(!confirmation_buffer.contains("Source: Built-in presets"));
    assert!(!confirmation_buffer.contains("Source: Product presets"));
}

#[test]
fn mixed_preset_sets_group_commands_without_inline_badges() {
    let mut state = TuiState::new(
        vec![
            Preset::new(
                "set-soracom-apn-cid1",
                "AT+CGDCONT=1,\"IP\",\"soracom.io\"",
                vec!["pdp".to_owned(), "apn".to_owned()],
                PresetOrigin::file("SORACOM commands", "soracom.toml", None),
            ),
            Preset::built_in("modem-response", "AT", ["basic"]),
            Preset::new(
                "qccid",
                "AT+QCCID",
                vec!["sim".to_owned()],
                PresetOrigin::file("Quectel commands", "quectel.toml", None),
            ),
        ],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    let buffer = rendered_buffer(&mut state, 120, 32);

    assert!(
        state
            .response
            .contains("External definitions loaded for this TUI session.")
    );
    assert!(state.response.contains("Loaded sources:"));
    assert!(state.response.contains("Quectel commands: quectel.toml"));
    assert!(state.response.contains("SORACOM commands: soracom.toml"));
    assert!(!buffer.contains("Built-in presets"));
    assert!(!buffer.contains("Product presets"));
    assert!(buffer.contains("Quectel commands"));
    assert!(buffer.contains("SORACOM commands"));
    assert!(!buffer.contains("Add-on:"));
    assert!(!buffer.contains("[built-in]"));
    assert!(!buffer.contains("[core]"));
    assert!(
        !state
            .categories
            .iter()
            .any(|category| category == "quectel")
    );
    assert!(
        !state
            .categories
            .iter()
            .any(|category| category == "soracom")
    );

    let names = state
        .visible_commands()
        .iter()
        .map(|command| command.name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["modem-response", "qccid", "set-soracom-apn-cid1"]
    );

    let visible = state.visible_commands();
    let rows = command_list_rows(&visible);
    assert!(matches!(rows[0], CommandListRow::Command { .. }));
    assert!(matches!(rows[1], CommandListRow::BlankSeparator));
    assert!(matches!(rows[2], CommandListRow::SourceHeader(_)));
    assert!(matches!(rows[3], CommandListRow::Command { .. }));
    assert!(matches!(rows[4], CommandListRow::BlankSeparator));
    assert!(matches!(rows[5], CommandListRow::SourceHeader(_)));
    assert!(matches!(rows[6], CommandListRow::Command { .. }));

    state.selected_command = 1;
    let status_buffer = rendered_buffer(&mut state, 120, 32);

    assert!(status_buffer.contains("Source: Quectel commands"));
    assert!(!status_buffer.contains("Preset set: Quectel commands"));
}

#[test]
fn external_preset_confirmation_shows_source_file_and_review_notice() {
    let mut state = TuiState::new(
        vec![Preset::new(
            "external-write",
            "ATE0",
            vec!["basic".to_owned()],
            PresetOrigin::file("External commands", "external.toml", None),
        )],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    handle_key_code(&mut state, KeyCode::Enter);
    let buffer = rendered_buffer(&mut state, 120, 32);

    assert!(buffer.contains("Confirm command"));
    assert!(buffer.contains("Name: external-write"));
    assert!(buffer.contains("Command: ATE0"));
    assert!(buffer.contains("Source: External commands"));
    assert!(buffer.contains("File: external.toml"));
    assert!(buffer.contains("Review external definition before running."));
}

#[test]
fn preset_set_headers_are_not_command_selection_targets() {
    let mut state = TuiState::new(
        vec![
            Preset::built_in("modem-response", "AT", ["basic"]),
            Preset::new(
                "qccid",
                "AT+QCCID",
                vec!["sim".to_owned()],
                PresetOrigin::file("Quectel commands", "quectel.toml", None),
            ),
        ],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.focus = Pane::Commands;

    let visible = state.visible_commands();
    let rows = command_list_rows(&visible);
    assert_eq!(rows.len(), 4);
    assert!(matches!(rows[0], CommandListRow::Command { .. }));
    assert!(matches!(rows[1], CommandListRow::BlankSeparator));
    assert!(matches!(rows[2], CommandListRow::SourceHeader(_)));
    assert!(matches!(rows[3], CommandListRow::Command { .. }));

    handle_key_code(&mut state, KeyCode::Down);
    assert_eq!(state.selected_command, 1);
    assert_eq!(
        state.selected_command().map(|preset| preset.name()),
        Some("qccid")
    );
}

#[test]
fn mixed_commands_and_sequences_group_kind_then_non_default_source_titles() {
    let mut state = TuiState::new_with_all_usb(
        executable_items(
            vec![
                Preset::built_in("pdp-contexts", "AT+CGDCONT?", ["data"]),
                Preset::new(
                    "set-soracom-apn-cid1",
                    "AT+CGDCONT=1,\"IP\",\"soracom.io\"",
                    vec!["data".to_owned(), "apn".to_owned()],
                    PresetOrigin::file("SORACOM commands", "soracom.toml", None),
                ),
            ],
            vec![
                Sequence::built_in(
                    "sms-send-check",
                    "Send an SMS and check receive state.",
                    vec!["data".to_owned(), "sms".to_owned()],
                    Some(180),
                    Vec::new(),
                    vec![test_sequence_step("AT+CMGS=\"+12025550123\"")],
                ),
                Sequence::new(
                    "quectel-tcp-send-check",
                    "Open a Quectel TCP socket, send a payload, and read a response.",
                    vec!["data".to_owned(), "network".to_owned()],
                    SequenceOrigin::file("Quectel Sequences", "quectel.toml", None),
                    Some(180),
                    Vec::new(),
                    vec![test_sequence_step(
                        "AT+QIOPEN=1,0,\"TCP\",\"example.com\",80",
                    )],
                ),
            ],
        ),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    let visible = state.visible_commands();
    let rows = command_list_rows(&visible);
    assert!(matches!(rows[0], CommandListRow::KindHeader("Commands")));
    assert!(matches!(rows[1], CommandListRow::Command { .. }));
    assert!(matches!(rows[2], CommandListRow::BlankSeparator));
    assert!(matches!(rows[3], CommandListRow::SourceHeader(_)));
    assert!(matches!(rows[4], CommandListRow::Command { .. }));
    assert!(matches!(rows[5], CommandListRow::BlankSeparator));
    assert!(matches!(rows[6], CommandListRow::KindHeader("Sequences")));
    assert!(matches!(rows[7], CommandListRow::Command { .. }));
    assert!(matches!(rows[8], CommandListRow::BlankSeparator));
    assert!(matches!(rows[9], CommandListRow::SourceHeader(_)));
    assert!(matches!(rows[10], CommandListRow::Command { .. }));

    let buffer = rendered_buffer(&mut state, 120, 32);
    assert!(buffer.contains("Commands / Sequences"));
    assert!(buffer.contains("SORACOM commands"));
    assert!(buffer.contains("Quectel Sequences"));
    assert!(!buffer.contains("Built-in presets"));
    assert!(!buffer.contains("Product presets"));
    assert!(!buffer.contains("Product Sequences"));
    assert!(!buffer.contains("Add-on:"));

    state.selected_command = 3;
    assert_eq!(
        state.selected_command().map(|command| command.name()),
        Some("quectel-tcp-send-check")
    );
    let status_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(status_buffer.contains("Selected Sequence:"));
    assert!(!status_buffer.contains("Run: confirmation required"));
    assert!(!status_buffer.contains("Risk: effects unknown"));
    assert!(!status_buffer.contains("Sequence set: Quectel Sequences"));
}

#[test]
fn executable_discovery_adjacent_states_render_across_all_themes() {
    let repository_sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-ping-check")
    .expect("repository-managed SORACOM Sequence");

    for theme in [TuiTheme::dark(), TuiTheme::light(), TuiTheme::no_color()] {
        let mut product_commands = TuiState::new(
            vec![
                Preset::built_in("inspect", "AT", ["review"]),
                Preset::built_in("change", "AT+CMEE=2", ["review"]),
                Preset::built_in("unknown", "AT+VENDOR=1", ["review"]),
            ],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        let product_buffer = rendered_buffer(&mut product_commands, 160, 32);
        let product_compact = product_buffer
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(product_compact.contains("> inspect AT"));
        assert!(product_compact.contains("change AT+CMEE=2 [confirm]"));
        assert!(product_compact.contains("unknown AT+VENDOR=1 [confirm]"));
        assert!(!product_buffer.contains("│ AT:"));
        assert!(!product_buffer.contains("│ Purpose:"));
        assert!(!product_buffer.contains("! Confirmation required"));
        assert!(!product_buffer.contains("Product presets"));

        let product_sequence = Sequence::built_in(
            "product-inspection",
            "Inspect modem response through a product Sequence.",
            vec!["review".to_owned()],
            Some(30),
            Vec::new(),
            vec![test_sequence_step("AT")],
        );
        let mut mixed = TuiState::new_with_all_usb(
            executable_items(
                vec![
                    Preset::built_in("product-command", "AT", ["review"]),
                    Preset::new(
                        "external-command",
                        "AT+QCCID",
                        vec!["review".to_owned()],
                        PresetOrigin::file("External commands", "external-commands.toml", None),
                    ),
                ],
                vec![product_sequence, repository_sequence.clone()],
            ),
            vec![test_usb_device(1, 3, "Onyx A")],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        let mixed_buffer = rendered_buffer(&mut mixed, 180, 44);
        assert!(mixed_buffer.contains("Commands / Sequences"));
        assert!(mixed_buffer.contains("Commands"));
        assert!(mixed_buffer.contains("Sequences"));
        assert!(mixed_buffer.contains("External commands"));
        assert!(mixed_buffer.contains("SORACOM Sequences"));
        assert!(mixed_buffer.contains("product-command"));
        assert!(mixed_buffer.contains("product-inspection"));
        assert!(mixed_buffer.contains("soracom-ping-check"));
        assert!(!mixed_buffer.contains("Product presets"));
        assert!(!mixed_buffer.contains("Product Sequences"));

        mixed.selected_command = mixed
            .visible_commands()
            .iter()
            .position(|command| command.name() == "soracom-ping-check")
            .expect("repository-managed Sequence row");
        rendered_buffer(&mut mixed, 180, 44);
        handle_key_code(&mut mixed, KeyCode::Char('d'));
        let details_buffer = rendered_buffer(&mut mixed, 180, 44);
        assert!(details_buffer.contains("Type: Sequence"));
        assert!(details_buffer.contains("Name: soracom-ping-check"));
        assert!(details_buffer.contains("Source: SORACOM Sequences"));
        assert!(details_buffer.contains("File: <inline>"));
        let mut complete_details = details_buffer;
        for _ in 0..6 {
            handle_key_code(&mut mixed, KeyCode::PageDown);
            complete_details.push_str(&rendered_buffer(&mut mixed, 180, 44));
        }
        assert!(complete_details.contains("Before running:"));
        assert!(complete_details.contains("Inputs:"));
        assert!(complete_details.contains("Execution:"));
        assert!(complete_details.contains("Timeout: 120 seconds"));

        let review_sequence = Sequence::built_in(
            "review-observation",
            "Review an observation before execution.",
            vec!["review".to_owned()],
            Some(30),
            vec![SequenceParam {
                name: "note".to_owned(),
                label: "Note".to_owned(),
                required: true,
                sensitive: false,
                default_value: None,
                source: SequenceParamSource::User,
                candidate: None,
                hint: None,
            }],
            vec![test_sequence_step("AT+CSQ")],
        );
        let mut review = TuiState::new_with_all_usb(
            executable_items(Vec::new(), vec![review_sequence]),
            vec![test_usb_device(1, 3, "Onyx A")],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        handle_key_code(&mut review, KeyCode::Enter);
        for value in "observed".chars() {
            handle_key_code(&mut review, KeyCode::Char(value));
        }
        handle_key_code(&mut review, KeyCode::Enter);
        let review_buffer = rendered_buffer(&mut review, 160, 44);
        assert!(review_buffer.contains("Review Sequence: review-observation"));
        assert!(review_buffer.contains("Review 1-"));
        assert!(review_buffer.contains("Resolved values:"));
        assert!(!review_buffer.contains("Purpose: Review an observation before execution."));

        let confirmation_sequence = Sequence::built_in(
            "confirm-state-change",
            "Confirm a known modem state change.",
            vec!["review".to_owned()],
            Some(30),
            Vec::new(),
            vec![test_sequence_step("AT+CFUN=1")],
        );
        let mut confirmation = TuiState::new_with_all_usb(
            executable_items(Vec::new(), vec![confirmation_sequence]),
            vec![test_usb_device(1, 3, "Onyx A")],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        handle_key_code(&mut confirmation, KeyCode::Enter);
        let confirmation_buffer = rendered_buffer(&mut confirmation, 160, 44);
        assert!(confirmation_buffer.contains("Confirm Sequence: confirm-state-change"));
        assert!(confirmation_buffer.contains("Review 1-"));
        assert!(!confirmation_buffer.contains("Purpose: Confirm a known modem state change."));
        assert!(confirmation_buffer.contains("Type `run` to run."));

        let resize_buffer = rendered_buffer(&mut confirmation, 140, 5);
        assert!(
            resize_buffer.contains("Executable details cannot fit safely."),
            "{:?}: {resize_buffer}",
            theme.mode()
        );
        assert!(resize_buffer.contains("Resize the terminal to continue."));
    }
}

#[test]
fn file_preset_commands_preserve_entry_order_within_preset_set() {
    let state = TuiState::new(
        vec![
            Preset::new(
                "second-quectel",
                "AT+QCSQ",
                vec!["signal".to_owned()],
                PresetOrigin::file("Quectel commands", "quectel.toml", None),
            ),
            Preset::new(
                "first-quectel",
                "AT+QCCID",
                vec!["sim".to_owned()],
                PresetOrigin::file("Quectel commands", "quectel.toml", None),
            ),
        ],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    let names = state
        .visible_commands()
        .iter()
        .map(|command| command.name())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["second-quectel", "first-quectel"]);
}

#[test]
fn raw_capture_requires_path_ack_and_writes_future_command() {
    let mut state = test_state();
    state.normal_logging_enabled = false;
    let mut executor = TestExecutor::default();
    let path = unique_temp_dir("raw-capture").join("case.rawlog");

    run_control(&mut state, ControlAction::RawExport);
    for value in path.display().to_string().chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.raw_log_ack_input.is_some());
    for value in RAW_LOG_ACK.chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.raw_capture.is_some());
    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"surface\":\"tui\""));
    assert!(raw.contains("\"command_name\":\"modem-response\""));
    assert!(raw.contains("\"tx_base64\":\"QVQN\""));
    assert!(state.raw_capture.is_some());
    assert_eq!(executor.normal_logging, vec![false]);

    run_control(&mut state, ControlAction::RawExport);
    assert!(state.raw_capture.is_none());
}

#[test]
fn selection_moves_within_bounds() {
    let mut state = test_state();
    state.focus = Pane::Commands;

    handle_key_code(&mut state, KeyCode::Down);
    assert_eq!(state.selected_command, 1);
    handle_key_code(&mut state, KeyCode::Up);
    assert_eq!(state.selected_command, 0);
    handle_key_code(&mut state, KeyCode::Up);
    assert_eq!(state.selected_command, 0);
}

#[test]
fn focus_cycle_uses_interactive_panes_in_visual_workflow_order() {
    let mut state = test_state();
    state.focus = Pane::Commands;

    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::Controls);
    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::Response);
    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::History);
    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::Devices);
    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::Categories);
    handle_key_code(&mut state, KeyCode::Tab);
    assert_eq!(state.focus, Pane::Commands);

    handle_key_code(&mut state, KeyCode::Left);
    assert_eq!(state.focus, Pane::Categories);
}

#[test]
fn response_scrolls_when_response_pane_is_focused() {
    let mut state = test_state();
    state.focus = Pane::Response;
    state.response = ResponseState::masked(
        (0..20)
            .map(|index| format!("line {index}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    state.response_visible_height = 10;
    state.response_visible_width = 80;

    handle_key_code(&mut state, KeyCode::Down);
    assert_eq!(state.response_scroll, 1);
    handle_key_code(&mut state, KeyCode::PageDown);
    assert_eq!(state.response_scroll, 10);
    handle_key_code(&mut state, KeyCode::Up);
    assert_eq!(state.response_scroll, 9);

    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("line 9"));
    assert!(!buffer.contains("line 0"));
}

#[test]
fn log_view_shows_response_local_line_numbers_and_range() {
    let mut state = test_state();
    state.focus = Pane::Response;
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: PathBuf::from("/tmp/test.session.log"),
        label: "test.session.log".to_owned(),
    });
    state.response = ResponseState::masked(
        (1..=20)
            .map(|index| format!("row {index}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("Response 1-"));
    assert!(buffer.contains("top/more below"));
    assert!(buffer.contains(" 1  row 1"));
    assert!(!buffer.contains("Response scroll: line"));

    handle_key_code(&mut state, KeyCode::End);
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("bottom"));
    assert!(buffer.contains("20  row 20"));
    assert!(!buffer.contains(" 1  row 1"));
}

#[test]
fn enter_executes_known_observation_through_executor() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);

    assert!(executor.calls.is_empty());
    assert!(state.pending_execution.is_some());
    assert!(state.response.contains("Waiting for modem response"));

    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("modem-response".to_owned(), false)]);
    assert!(state.response.contains("OK"));
    assert!(!state.response.contains("Command: AT"));
    assert!(state.status.contains("status=OK"));
    assert!(state.confirmation.is_none());
}

#[test]
fn ad_hoc_input_executes_known_observation_through_executor() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    run_control(&mut state, ControlAction::AdHocCommand);
    for value in "AT+CSQ".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.ad_hoc_input.is_none());
    assert!(state.pending_execution.is_some());
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("ad-hoc".to_owned(), false)]);
    assert!(state.response.contains("AT+CSQ"));
    assert_eq!(
        state
            .active_command
            .as_ref()
            .and_then(|command| command.source_title.as_deref()),
        None
    );
}

#[test]
fn ad_hoc_material_command_uses_existing_confirmation_flow() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    run_control(&mut state, ControlAction::AdHocCommand);
    for value in "ATE0".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.confirmation.is_some());
    assert!(state.pending_execution.is_none());
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("ad-hoc".to_owned(), true)]);
}

#[test]
fn ad_hoc_input_rejects_prompt_required_sms_command() {
    let mut state = test_state();

    run_control(&mut state, ControlAction::AdHocCommand);
    for value in "AT+CMGS=\"+819012345678\"".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.ad_hoc_input.is_some());
    assert!(state.pending_execution.is_none());
    assert!(state.status.contains("Prompt-required"));
}

#[test]
fn ad_hoc_input_rejects_prompt_required_socket_data_entry() {
    let mut state = test_state();

    run_control(&mut state, ControlAction::AdHocCommand);
    for value in "AT+QISEND=0,4".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.ad_hoc_input.is_some());
    assert!(state.pending_execution.is_none());
    assert!(state.status.contains("Prompt-required"));
}

#[test]
fn command_search_filters_visible_commands() {
    let mut state = test_state();

    handle_key_code(&mut state, KeyCode::Char('/'));
    for value in "sim".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    let names = state
        .visible_commands()
        .iter()
        .map(|command| command.name())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["imsi"]);
    assert!(state.status.contains("matched 1"));
    assert!(rendered_buffer(&mut state, 100, 32).contains("Commands / Sequences search: sim"));

    handle_key_code(&mut state, KeyCode::Char('/'));
    for _ in 0.."sim".len() {
        handle_key_code(&mut state, KeyCode::Backspace);
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.search_query.is_empty());
    assert!(state.visible_commands().len() > 1);
}

#[test]
fn executable_details_is_read_only_scrollable_and_restores_discovery_state() {
    let purpose = format!(
        "Inspect a complete Sequence purpose without device access. {} terminal-end-marker",
        "Long explanatory context remains available through the scrollable Details overlay. "
            .repeat(12)
    );
    let sequence = Sequence::built_in(
        "review-flow",
        purpose,
        vec!["sms".to_owned()],
        Some(30),
        Vec::new(),
        vec![test_sequence_step("AT+CSQ")],
    );
    let mut state = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![sequence]),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        TuiTheme::dark(),
    );
    state.search_query = "sms".to_owned();
    let category = state.selected_category;
    let selected = state.selected_command;

    rendered_buffer(&mut state, 160, 14);
    handle_key_code(&mut state, KeyCode::Char('d'));

    assert!(state.executable_details.is_some());
    assert!(state.pending_execution.is_none());
    assert!(state.running_execution.is_none());
    let initial = rendered_buffer(&mut state, 160, 14);
    assert!(initial.contains("Details · Overview 1-"));
    assert!(initial.contains("Type: Sequence"));
    assert!(initial.contains("Name: review-flow"));
    assert!(initial.contains("Purpose:"));
    assert_eq!(
        state
            .executable_details
            .as_ref()
            .and_then(|details| details.search_match.as_deref()),
        Some("Category: sms")
    );
    assert_eq!(
        footer_text(&state, 160),
        "Up/Down Scroll  PgUp/PgDn Page  Esc Close"
    );

    handle_key_code(&mut state, KeyCode::Char('q'));
    assert!(state.executable_details.is_some());
    let mut detail_pages = initial;
    for _ in 0..20 {
        handle_key_code(&mut state, KeyCode::PageDown);
        detail_pages.push_str(&rendered_buffer(&mut state, 160, 14));
    }
    let _final_page = rendered_buffer(&mut state, 160, 14);
    assert!(detail_pages.contains("Overview:"));
    assert!(detail_pages.contains("Definition:"));
    assert!(detail_pages.contains("Execution:"));
    assert!(detail_pages.contains("Details · Definition"));
    assert!(detail_pages.contains("Search match: Category: sms"));
    assert!(
        state
            .executable_details
            .as_ref()
            .is_some_and(|details| details.scroll > 0)
    );

    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.executable_details.is_none());
    assert_eq!(state.selected_category, category);
    assert_eq!(state.selected_command, selected);
    assert_eq!(state.search_query, "sms");
}

#[test]
fn command_details_shows_complete_identity_and_non_default_source_without_device() {
    let preset = Preset::new(
        "external-command-with-a-long-name",
        "AT+EXTERNAL=1234567890,1234567890,1234567890",
        vec!["diagnostics".to_owned()],
        PresetOrigin::file("External command set", "/tmp/external-commands.toml", None),
    );
    let mut state = TuiState::new_with_all_usb(
        executable_items(vec![preset], Vec::new()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        TuiTheme::dark(),
    );
    state.search_query = "external command set".to_owned();

    rendered_buffer(&mut state, 160, 44);
    handle_key_code(&mut state, KeyCode::Char('d'));
    let buffer = rendered_buffer(&mut state, 160, 44);

    assert!(buffer.contains("Type: Command"));
    assert!(buffer.contains("Name: external-command-with-a-long-name"));
    assert!(buffer.contains("AT command: AT+EXTERNAL=1234567890,1234567890,1234567890"));
    assert!(buffer.contains("Source: External command set"));
    assert!(buffer.contains("File: /tmp/external-commands.toml"));
    assert!(buffer.contains("Search match: Source: External command set"));
    assert!(!buffer.contains("Run: confirmation required"));
    assert!(!buffer.contains("Risk: effects unknown"));
    assert!(buffer.contains("Categories: diagnostics"));
    assert!(buffer.contains("Timeout: 30 seconds"));
    assert!(!buffer.contains("seconds (default)"));
    assert_contains_in_order(&buffer, &["Overview:", "Definition:", "Execution:"]);
    assert!(!buffer.contains("Before running:"));
    assert!(!buffer.contains("Inputs:"));
    assert!(!buffer.contains("Execution review:"));
    assert!(buffer.contains("Assessment: unknown"));
    assert!(buffer.contains("Effects are not fully known."));
    assert!(buffer.contains("Confirmation: required"));
    assert!(state.pending_execution.is_none());
}

#[test]
fn details_timeout_matches_effective_command_and_sequence_timeout() {
    let cases = vec![
        (
            "command product default",
            ExecutableItem::Preset(Preset::new_with_timeout(
                "command-default",
                "AT",
                vec!["review".to_owned()],
                PresetOrigin::BuiltIn,
                None,
            )),
            None,
            "Timeout: 30 seconds",
        ),
        (
            "command item timeout",
            ExecutableItem::Preset(Preset::new_with_timeout(
                "command-item-timeout",
                "AT",
                vec!["review".to_owned()],
                PresetOrigin::BuiltIn,
                Some(75),
            )),
            None,
            "Timeout: 75 seconds",
        ),
        (
            "command session timeout",
            ExecutableItem::Preset(Preset::new_with_timeout(
                "command-session-timeout",
                "AT",
                vec!["review".to_owned()],
                PresetOrigin::BuiltIn,
                Some(75),
            )),
            Some(120),
            "Timeout: 120 seconds",
        ),
        (
            "Sequence product default",
            ExecutableItem::Sequence(Sequence::built_in(
                "sequence-default",
                "Inspect modem response.",
                vec!["review".to_owned()],
                None,
                Vec::new(),
                vec![test_sequence_step("AT")],
            )),
            None,
            "Timeout: 30 seconds",
        ),
        (
            "Sequence item timeout",
            ExecutableItem::Sequence(Sequence::built_in(
                "sequence-item-timeout",
                "Inspect modem response.",
                vec!["review".to_owned()],
                Some(90),
                Vec::new(),
                vec![test_sequence_step("AT")],
            )),
            None,
            "Timeout: 90 seconds",
        ),
        (
            "Sequence session timeout",
            ExecutableItem::Sequence(Sequence::built_in(
                "sequence-session-timeout",
                "Inspect modem response.",
                vec!["review".to_owned()],
                Some(90),
                Vec::new(),
                vec![test_sequence_step("AT")],
            )),
            Some(150),
            "Timeout: 150 seconds",
        ),
    ];

    for (case, item, session_timeout, expected) in cases {
        let mut state = TuiState::new_with_all_usb(
            vec![item],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            TuiTheme::dark(),
        );
        state.timeout_override_secs = session_timeout;

        rendered_buffer(&mut state, 160, 24);
        handle_key_code(&mut state, KeyCode::Char('d'));
        let buffer = rendered_buffer(&mut state, 160, 24);

        assert!(buffer.contains(expected), "{case}: {buffer}");
        assert!(!buffer.contains("seconds (default)"), "{case}: {buffer}");
    }
}

#[test]
fn built_in_source_metadata_is_not_searchable_or_disclosed() {
    let cases = vec![
        (
            ExecutableItem::Preset(Preset::built_in("product-command", "AT", ["review"])),
            "Product presets",
        ),
        (
            ExecutableItem::Sequence(Sequence::built_in(
                "product-sequence",
                "Inspect modem response.",
                vec!["review".to_owned()],
                Some(30),
                Vec::new(),
                vec![test_sequence_step("AT")],
            )),
            "Product Sequences",
        ),
    ];

    for (item, source_label) in cases {
        assert!(!command_matches_search(&item, source_label));
        assert_eq!(hidden_search_match(&item, source_label, item.name()), None);

        let mut state = TuiState::new_with_all_usb(
            vec![item],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            TuiTheme::dark(),
        );
        state.search_query = source_label.to_owned();

        assert!(state.visible_commands().is_empty());
        let empty_buffer = rendered_buffer(&mut state, 160, 24);
        assert!(
            empty_buffer.contains(&format!("No commands or Sequences match `{source_label}`."))
        );

        state.search_query.clear();
        rendered_buffer(&mut state, 160, 24);
        handle_key_code(&mut state, KeyCode::Char('d'));
        let details_buffer = rendered_buffer(&mut state, 160, 24);

        assert!(!details_buffer.contains("Source:"));
        assert!(!details_buffer.contains("Search match: Source:"));
        assert!(!details_buffer.contains(source_label));
    }
}

#[test]
fn repository_managed_sequence_source_is_searchable_and_disclosed() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-ping-check")
    .expect("repository-managed SORACOM Sequence");
    let mut state = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![sequence]),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        TuiTheme::dark(),
    );
    state.search_query = "SORACOM Sequences".to_owned();

    assert_eq!(state.visible_commands().len(), 1);
    rendered_buffer(&mut state, 160, 30);
    handle_key_code(&mut state, KeyCode::Char('d'));
    let buffer = rendered_buffer(&mut state, 160, 30);

    assert!(buffer.contains("Name: soracom-ping-check"));
    assert!(buffer.contains("Source: SORACOM Sequences"));
    assert!(buffer.contains("File: <inline>"));
    assert!(buffer.contains("Search match: Source: SORACOM Sequences"));
}

#[test]
fn executable_empty_states_explain_recovery_without_starting_work() {
    let mut search_state = test_state();
    search_state.search_query = "no-such-executable".to_owned();
    let search_buffer = rendered_buffer(&mut search_state, 160, 32);
    assert!(search_buffer.contains("No commands or Sequences match `no-such-executable`."));
    assert!(search_buffer.contains("Press / to change or clear the search."));
    assert!(search_state.pending_execution.is_none());

    let mut category_state = test_state();
    category_state.commands.clear();
    let category_buffer = rendered_buffer(&mut category_state, 160, 32);
    assert!(category_buffer.contains("No commands or Sequences are available in this category."));
    assert!(category_buffer.contains("Press Left to choose another category, or / to search."));
    assert!(category_state.pending_execution.is_none());
}

#[test]
fn sequence_rows_are_selectable_from_commands_pane() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");

    let names = state
        .visible_commands()
        .iter()
        .map(|command| command.name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "sms-send-check",
            "sms-receive-check",
            "sms-read-message",
            "sms-reply-check"
        ]
    );

    let buffer = rendered_buffer(&mut state, 120, 32);

    assert!(buffer.contains("Commands / Sequences"));
    assert!(buffer.contains("sms-send-check"));
    assert!(buffer.contains("[confirm]"));
    assert!(!buffer.contains("│ Purpose:"));
    assert!(!buffer.contains("Product Sequences"));
}

#[test]
fn selected_sequence_status_does_not_show_summary_text() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = 1;

    let buffer = rendered_status_buffer(&mut state, 80, 12);

    assert!(buffer.contains("Selected Sequence:"));
    assert!(buffer.contains("sms-receive-check"));
    assert!(!buffer.contains("Run: confirmation required"));
    assert!(!buffer.contains("Confirmation:"));
    assert!(!buffer.contains("Assessment:"));
    assert!(!buffer.contains("Risk: effects unknown"));
    assert!(!buffer.contains("Summary:"));
    assert!(!buffer.contains("List received SMS material"));
}

#[test]
fn completed_sequence_status_does_not_show_summary_text() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor::default();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let buffer = rendered_status_buffer(&mut state, 80, 12);

    assert!(buffer.contains("Sequence:"));
    assert!(buffer.contains("sms-receive-check"));
    assert!(buffer.contains("Result: OK 7ms"));
    assert!(!buffer.contains("Summary:"));
    assert!(!buffer.contains("List received SMS material"));
}

#[test]
fn failed_sequence_status_keeps_full_error_out_of_compact_status() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor {
        sequence_error: Some(AtctlError::SequenceExpectationFailed {
            sequence: "soracom-unified-endpoint-tcp-send-check".to_owned(),
            step: "activate-context".to_owned(),
            expected: "OK".to_owned(),
        }),
        ..TestExecutor::default()
    };
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-unified-endpoint-tcp-send-check")
    .unwrap();
    state.commands = executable_items(Vec::new(), vec![sequence]);
    state.categories = categories_from_commands(&state.commands);
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "data")
        .expect("data category");

    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "payload".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let status_buffer = rendered_status_buffer(&mut state, 96, 12);
    let failed_at = state
        .active_command
        .as_ref()
        .and_then(|active| active.finished_at.as_deref())
        .expect("failed timestamp")
        .to_owned();
    let failed_line = format!("Failed: {failed_at}");
    assert!(status_buffer.contains("Status: failed"));
    assert!(status_buffer.contains(&failed_line));
    assert!(!status_buffer.contains("Failed at:"));
    assert!(status_buffer.contains("Sequence:"));
    assert!(status_buffer.contains("soracom-unified-endpoint"));
    assert!(status_buffer.contains("Result: failed"));
    assert!(!status_buffer.contains("Detail:"));
    assert!(!status_buffer.contains("did not produce expected marker"));
    assert!(!status_buffer.contains("activate-context"));

    let response_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(response_buffer.contains("Result: failed"));
    assert!(
        state
            .response
            .contains("Result: failed\n\nSequence failed before response.")
    );
    assert!(response_buffer.contains("Sequence failed before response."));
    assert!(state.response.contains("activate-context"));
    assert!(state.response.contains("did not produce expected marker"));
}

#[test]
fn timeout_failure_keeps_status_concise_and_guidance_in_response() {
    let mut state = test_state();
    let item = state.selected_command().cloned().expect("selected command");
    state.running_execution = Some(RunningExecution {
        started_at: Instant::now(),
        timeout: Duration::from_secs(75),
    });

    finish_execution_result(
        &mut state,
        item,
        75,
        Err(AtctlError::Timeout { timeout_secs: 75 }),
    );

    let status = rendered_status_buffer(&mut state, 96, 12);
    assert!(status.contains("Status: failed"));
    assert!(status.contains("Result: failed"));
    assert!(!status.contains("Timed out after"));
    assert!(!status.contains("modem may still"));
    assert!(
        state
            .response
            .contains("Result: failed\n\nTimed out after 75 seconds.")
    );
    assert!(
        state
            .response
            .contains("The modem may still be processing the operation.")
    );
    assert!(
        state
            .response
            .contains("Wait for it to settle before deciding whether to retry.")
    );
}

#[test]
fn sequence_timeout_failure_identifies_step_in_response_only() {
    let mut state = test_state_with_sequences();
    let item = state
        .commands
        .iter()
        .find(|item| item.as_sequence().is_some())
        .cloned()
        .expect("Sequence");

    finish_execution_result(
        &mut state,
        item,
        120,
        Err(AtctlError::SequenceStepTimeout {
            sequence: "sms-send-check".to_owned(),
            step: "submit-message".to_owned(),
            timeout_secs: 45,
        }),
    );

    let status = rendered_status_buffer(&mut state, 96, 12);
    assert!(status.contains("Result: failed"));
    assert!(!status.contains("submit-message"));
    assert!(state.response.contains("Timed out after 45 seconds."));
    assert!(state.response.contains("Sequence step: submit-message"));
}

#[test]
fn sequence_error_status_is_failed_state_even_when_transcript_exists() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor {
        sequence_status: Some(AtStatus::Error),
        ..TestExecutor::default()
    };
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let status_buffer = rendered_status_buffer(&mut state, 80, 12);
    let failed_at = state
        .active_command
        .as_ref()
        .and_then(|active| active.finished_at.as_deref())
        .expect("failed timestamp")
        .to_owned();
    let failed_line = format!("Failed: {failed_at}");
    assert!(status_buffer.contains("Status: failed"));
    assert!(status_buffer.contains(&failed_line));
    assert!(!status_buffer.contains("Failed at:"));
    assert!(status_buffer.contains("Sequence:"));
    assert!(status_buffer.contains("sms-receive-check"));
    assert!(status_buffer.contains("Result: ERROR 7ms"));
    assert!(!status_buffer.contains("Risk:"));
    assert!(!status_buffer.contains("Confirmation:"));
    assert_contains_in_order(
        &status_buffer,
        &[
            "Status: failed",
            &failed_line,
            "Sequence:",
            "sms-receive-check",
            "Result: ERROR 7ms",
        ],
    );
    assert!(state.response.contains("Result: failed duration=7ms"));
    assert!(
        state
            .response
            .contains("Reason: test sequence status failure")
    );
}

#[test]
fn run_sequence_modal_collects_params_and_confirmation_before_execution() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor::default();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");

    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.sequence_input.is_some());
    assert!(state.pending_execution.is_none());
    let buffer = rendered_buffer(&mut state, 120, 32);
    assert!(buffer.contains("Run Sequence: sms-send-check"));
    assert!(buffer.contains("Value 1 of 2"));
    assert!(buffer.contains("Recipient"));
    assert!(!buffer.contains("Message body"));
    assert!(!buffer.contains("Details "));
    assert!(buffer.contains("Enter: Next value (2 of 2)."));

    handle_key_code(&mut state, KeyCode::Enter);
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.active_param),
        Some(0)
    );
    let validation_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(validation_buffer.contains("Value 1 of 2"));
    assert!(validation_buffer.contains("Error:"));

    for value in "+819012345678".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    let input_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(input_buffer.contains("+819012345678"));
    handle_key_code(&mut state, KeyCode::Enter);
    let second_value_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(second_value_buffer.contains("Value 2 of 2"));
    assert!(second_value_buffer.contains("Enter: Review Sequence."));
    for value in "hello from atctl".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    let confirmation_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(confirmation_buffer.contains("Review 1-"));
    assert!(confirmation_buffer.contains("Resolved values:"));
    assert!(confirmation_buffer.contains("Operation:"));
    assert!(!confirmation_buffer.contains("Value 2 of 2"));
    assert!(confirmation_buffer.contains("Destination"));
    assert!(confirmation_buffer.contains("+819012345678"));
    assert!(confirmation_buffer.contains("Message body"));
    assert!(confirmation_buffer.contains("hello from atctl"));
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.confirmation_input.as_str()),
        Some("q")
    );
    handle_key_code(&mut state, KeyCode::Backspace);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.sequence_input.is_none());
    assert!(state.pending_execution.is_some());
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("sms-send-check".to_owned(), true)]);
    assert_eq!(
        executor.sequence_params,
        vec![vec![
            ("recipient".to_owned(), "+819012345678".to_owned()),
            ("message".to_owned(), "hello from atctl".to_owned()),
        ]]
    );
    assert!(state.response.contains("Sequence sms-send-check"));
    assert_eq!(
        state.active_command.as_ref().map(|command| command.kind),
        Some(ExecutableKind::Sequence)
    );
}

#[test]
fn run_sequence_modal_shows_value_sources_defaults_and_hints() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-unified-endpoint-tcp-send-check")
    .unwrap();
    let mut state = TuiState::new_with_all_usb(
        executable_items(crate::presets::builtin::builtins(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|item| item.name() == "soracom-unified-endpoint-tcp-send-check")
        .expect("SORACOM Sequence");

    handle_key_code(&mut state, KeyCode::Enter);

    let buffer = rendered_buffer(&mut state, 140, 36);
    assert!(buffer.contains("Run Sequence: soracom-unified-endpoint-tcp-send-check"));
    assert!(buffer.contains("Value 1 of 4"));
    assert!(buffer.contains("PDP context ID"));
    assert!(buffer.contains("> 1  default"));
    assert!(buffer.contains("Confirm the PDP context"));
    assert!(!buffer.contains("Values overview:"));
    assert!(!buffer.contains("Socket connect ID: 0  default"));
    assert!(!buffer.contains("Payload (sensitive): <empty>  user"));
    assert!(!buffer.contains("Read length: 1500  default"));
    assert!(!buffer.contains("During execution"));
    assert!(!buffer.contains("Details "));
    handle_key_code(&mut state, KeyCode::PageDown);
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.detail_scroll),
        Some(0)
    );
    assert!(line_containing(&buffer, "Run active-pdp-contexts").contains("Preset"));
    assert!(line_containing(&buffer, "Run pdp-contexts").contains("Preset"));
    assert!(!buffer.contains("Load Quectel PDP contexts  AT+QIACT?"));

    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.pending_execution.is_some());
    execute_pending_command(&mut state, &mut TestExecutor::default());
    let socket_buffer = rendered_buffer(&mut state, 140, 36);
    assert!(!socket_buffer.contains("AT+QISTATE"));
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "hello".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    let confirmation_buffer = rendered_buffer(&mut state, 140, 44);
    assert!(
        confirmation_buffer.contains("Confirm Sequence: soracom-unified-endpoint-tcp-send-check")
    );
    assert!(!confirmation_buffer.contains("Value 1 of 4"));
    assert!(!confirmation_buffer.contains("Value 4 of 4"));
    assert!(confirmation_buffer.contains("Review 1-"));
    assert!(confirmation_buffer.contains("Resolved values:"));
    assert!(confirmation_buffer.contains("PDP context ID: 1  default"));
    assert!(confirmation_buffer.contains("Socket connect ID: 0  default"));
    assert!(confirmation_buffer.contains("Payload (sensitive): hello  user"));
    assert!(confirmation_buffer.contains("Read length: 1500  default"));
    assert!(confirmation_buffer.contains("Operation:"));
    assert!(confirmation_buffer.contains("Destination"));
    assert!(confirmation_buffer.contains("unified.soracom.io:23080"));
    assert!(confirmation_buffer.contains("Before running:"));
    assert!(confirmation_buffer.contains("Execution:"));
    assert!(confirmation_buffer.contains("What will happen:"));
    assert!(confirmation_buffer.contains("Type `run` to run."));
    assert!(confirmation_buffer.contains("Input: <empty>"));
    assert_contains_in_order(
        &confirmation_buffer,
        &[
            "Review 1-",
            "Resolved values:",
            "Operation:",
            "Before running:",
            "Execution:",
            "What will happen:",
            "Type `run` to run.",
            "Input: <empty>",
        ],
    );

    let compact_confirmation_buffer = rendered_buffer(&mut state, 140, 24);
    assert!(compact_confirmation_buffer.contains("Review "));
    assert!(!compact_confirmation_buffer.contains("Details "));
    assert!(!compact_confirmation_buffer.contains("detail omitted"));
    assert!(compact_confirmation_buffer.contains("Type `run` to run."));
    assert!(compact_confirmation_buffer.contains("Input: <empty>"));

    let unsafe_confirmation_buffer = rendered_buffer(&mut state, 140, 5);
    assert!(
        unsafe_confirmation_buffer.contains("Executable details cannot fit safely."),
        "{unsafe_confirmation_buffer}"
    );
    assert!(unsafe_confirmation_buffer.contains("Resize the terminal to continue."));
    assert!(!unsafe_confirmation_buffer.contains("Input: <empty>"));

    for value in "ru".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    let typed_confirmation_buffer = rendered_buffer(&mut state, 140, 24);
    assert!(typed_confirmation_buffer.contains("Type `run` to run."));
    assert!(typed_confirmation_buffer.contains("Input: ru"));
}

#[test]
fn every_sequence_modal_phase_keeps_identity_and_only_phase_relevant_content() {
    let mut params = test_state_with_sequences();
    params.selected_category = params
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    params.selected_command = params
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-send-check")
        .expect("sms-send-check");
    handle_key_code(&mut params, KeyCode::Enter);
    let params_purpose = params
        .sequence_input
        .as_ref()
        .expect("params phase")
        .sequence
        .summary
        .clone();
    params.sequence_input.as_mut().expect("params phase").error =
        Some("Keep identity visible with an input error.".to_owned());
    let params_buffer = rendered_buffer(&mut params, 160, 44);
    assert!(params_buffer.contains("Run Sequence: sms-send-check"));
    assert!(!params_buffer.contains(&format!("Purpose: {params_purpose}")));
    assert!(params_buffer.contains("Keep identity visible with an input error."));
    assert!(!params_buffer.contains("Current value:"));
    assert!(!params_buffer.contains("Value 1/2:"));
    assert!(!params_buffer.contains("Details "));
    assert!(!params_buffer.contains("Values overview:"));
    assert_contains_in_order(
        &params_buffer,
        &[
            "Run Sequence: sms-send-check",
            "Value 1 of 2",
            "Recipient",
            "Keep identity visible with an input error.",
            "Enter: Next value (2 of 2).",
        ],
    );

    let mut candidate = test_state_with_sequences();
    candidate.selected_category = candidate
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    candidate.selected_command = candidate
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message");
    handle_key_code(&mut candidate, KeyCode::Enter);
    handle_key_code(&mut candidate, KeyCode::Down);
    handle_key_code(&mut candidate, KeyCode::Enter);
    assert_eq!(
        candidate.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::CandidateActionConfirmation)
    );
    let candidate_purpose = candidate
        .sequence_input
        .as_ref()
        .expect("candidate confirmation")
        .sequence
        .summary
        .clone();
    let candidate_buffer = rendered_buffer(&mut candidate, 160, 44);
    assert!(candidate_buffer.contains("Confirm action: sms-read-message"));
    assert!(!candidate_buffer.contains(&format!("Purpose: {candidate_purpose}")));
    assert!(candidate_buffer.contains("Provider: Sequence"));
    assert!(candidate_buffer.contains("Name: sms-receive-check"));
    assert!(
        candidate_buffer.contains("Purpose: List received SMS material and decode message bodies")
    );
    assert!(!candidate_buffer.contains("AT command: AT+CMGL=\"ALL\""));
    assert!(!candidate_buffer.contains("Value 1 of 1"));
    assert!(candidate_buffer.contains("Action review 1-"));
    assert!(!candidate_buffer.contains("Details "));
    assert_contains_in_order(
        &candidate_buffer,
        &[
            "Action review 1-",
            "Provider: Sequence",
            "Name: sms-receive-check",
            "Type `run` to run.",
            "Input: <empty>",
        ],
    );

    let review_sequence = Sequence::built_in(
        "review-only",
        "Review a known observation before execution.",
        vec!["review".to_owned()],
        Some(30),
        vec![SequenceParam {
            name: "note".to_owned(),
            label: "Note".to_owned(),
            required: true,
            sensitive: false,
            default_value: None,
            source: SequenceParamSource::User,
            candidate: None,
            hint: None,
        }],
        vec![test_sequence_step("AT+CSQ")],
    );
    let mut review = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![review_sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    handle_key_code(&mut review, KeyCode::Enter);
    for value in "review".chars() {
        handle_key_code(&mut review, KeyCode::Char(value));
    }
    handle_key_code(&mut review, KeyCode::Enter);
    let review_buffer = rendered_buffer(&mut review, 160, 44);
    assert!(review_buffer.contains("Review Sequence: review-only"));
    assert!(!review_buffer.contains("Value 1 of 1"));
    assert!(!review_buffer.contains("Purpose: Review a known observation before execution."));
    assert!(review_buffer.contains("Review 1-"));
    assert!(review_buffer.contains("Resolved values:"));
    assert!(review_buffer.contains("Execution:"));
    assert!(!review_buffer.contains("Details "));
    assert_contains_in_order(
        &review_buffer,
        &[
            "Review 1-",
            "Resolved values:",
            "Execution:",
            "What will happen:",
            "Press Enter to run.",
        ],
    );

    for (name, purpose, command, title) in [
        (
            "known-confirmation",
            "Confirm a known state change.",
            "AT+CFUN=1",
            "Confirm Sequence",
        ),
        (
            "unknown-confirmation",
            "Confirm an operation whose effects are unknown.",
            "AT+VENDOR=1",
            "Confirm unknown Sequence",
        ),
    ] {
        let sequence = Sequence::built_in(
            name,
            purpose,
            vec!["review".to_owned()],
            Some(30),
            Vec::new(),
            vec![test_sequence_step(command)],
        );
        let mut state = TuiState::new_with_all_usb(
            executable_items(Vec::new(), vec![sequence]),
            vec![test_usb_device(1, 3, "Onyx A")],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            TuiTheme::dark(),
        );
        handle_key_code(&mut state, KeyCode::Enter);
        let buffer = rendered_buffer(&mut state, 160, 44);
        assert!(buffer.contains(&format!("{title}: {name}")));
        assert!(!buffer.contains(&format!("Purpose: {purpose}")));
        assert!(buffer.contains("Review 1-"));
        assert!(!buffer.contains("Details "));
        assert_contains_in_order(
            &buffer,
            &["Review 1-", "Execution:", "Type `", "Input: <empty>"],
        );
    }
}

#[test]
fn sequence_input_stays_focused_and_final_review_scrolls_independently() {
    let sequence = Sequence::built_in(
        "long-input-context",
        "Resolve values while long context remains reviewable.",
        vec!["review".to_owned()],
        Some(30),
        vec![
            SequenceParam {
                name: "first".to_owned(),
                label: "First value".to_owned(),
                required: true,
                sensitive: false,
                default_value: Some("one".to_owned()),
                source: SequenceParamSource::Default,
                candidate: None,
                hint: Some("First value hint.".to_owned()),
            },
            SequenceParam {
                name: "second".to_owned(),
                label: "Second value".to_owned(),
                required: true,
                sensitive: false,
                default_value: Some("two".to_owned()),
                source: SequenceParamSource::Default,
                candidate: None,
                hint: Some("Second value hint.".to_owned()),
            },
        ],
        vec![test_sequence_step("AT+CSQ")],
    )
    .with_before_running(
        (1..=18)
            .map(|index| format!("Context line {index} must remain available."))
            .collect(),
    );
    let mut state = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    handle_key_code(&mut state, KeyCode::Enter);

    let first = rendered_buffer(&mut state, 120, 18);
    assert!(!first.contains("Current value:"));
    assert!(first.contains("Value 1 of 2"));
    assert!(!first.contains("Value 1/2:"));
    assert!(first.contains("First value"));
    assert!(first.contains("> one  default"));
    assert!(!first.contains("Details "));
    assert!(!first.contains("Context line 1"));
    assert!(!first.contains("Purpose:"));
    assert_contains_in_order(
        &first,
        &[
            "Run Sequence: long-input-context",
            "Value 1 of 2",
            "First value",
            "> one  default",
            "Enter: Next value (2 of 2).",
        ],
    );
    assert!(footer_text(&state, 160).contains("Enter Next value"));
    assert!(footer_text(&state, 160).contains("Up/Down Values"));
    handle_key_code(&mut state, KeyCode::PageDown);
    let unchanged = rendered_buffer(&mut state, 120, 18);
    assert!(unchanged.contains("First value"));
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.detail_scroll),
        Some(0)
    );
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.active_param),
        Some(0)
    );

    handle_key_code(&mut state, KeyCode::Down);
    let second = rendered_buffer(&mut state, 120, 18);
    assert!(second.contains("Value 2 of 2"));
    assert!(second.contains("Second value"));
    assert!(second.contains("> two  default"));
    assert_contains_in_order(
        &second,
        &[
            "Run Sequence: long-input-context",
            "Value 2 of 2",
            "Second value",
            "> two  default",
            "Enter: Review Sequence.",
        ],
    );
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.active_param),
        Some(1)
    );
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.detail_scroll),
        Some(0)
    );
    assert!(footer_text(&state, 160).contains("Enter Review"));

    handle_key_code(&mut state, KeyCode::Enter);
    let review = rendered_buffer(&mut state, 120, 18);
    assert!(review.contains("Review Sequence: long-input-context"));
    assert!(review.contains("Review 1-"));
    assert!(review.contains("Resolved values:"));
    assert!(review.contains("Press Enter to run."));
    assert!(!review.contains("Details "));
    handle_key_code(&mut state, KeyCode::PageDown);
    let scrolled_review = rendered_buffer(&mut state, 120, 18);
    assert!(scrolled_review.contains("Review "));
    assert!(scrolled_review.contains("Press Enter to run."));
    assert!(
        state
            .sequence_input
            .as_ref()
            .is_some_and(|input| input.detail_scroll > 0)
    );

    let narrow = rendered_buffer(&mut state, 30, 18);
    assert!(narrow.contains("Review"));
    assert!(narrow.contains("Press Enter"));
    assert!(!narrow.contains("Executable details"));
}

#[test]
fn multi_value_position_is_literal_in_dark_light_and_no_color_themes() {
    for theme in [TuiTheme::dark(), TuiTheme::light(), TuiTheme::no_color()] {
        let mut state = test_state_with_sequences_and_theme(theme);
        state.selected_category = state
            .categories
            .iter()
            .position(|category| category == "sms")
            .expect("sms category");
        state.selected_command = state
            .visible_commands()
            .iter()
            .position(|command| command.name() == "sms-send-check")
            .expect("sms-send-check");

        handle_key_code(&mut state, KeyCode::Enter);
        let first = rendered_buffer(&mut state, 140, 36);
        assert!(first.contains("Value 1 of 2"));
        assert!(first.contains("Recipient"));
        assert!(!first.contains("Value 1/2:"));

        for value in "+819012345678".chars() {
            handle_key_code(&mut state, KeyCode::Char(value));
        }
        handle_key_code(&mut state, KeyCode::Enter);
        let second = rendered_buffer(&mut state, 140, 36);
        assert!(second.contains("Value 2 of 2"));
        assert!(second.contains("Message body (sensitive)"));
        assert!(second.contains("Enter: Review Sequence."));
    }
}

#[test]
fn run_sequence_modal_rederives_review_from_bound_values() {
    let sequence = Sequence::built_in(
        "dynamic-functionality",
        "Set the requested modem functionality.",
        vec!["modem".to_owned()],
        Some(30),
        vec![SequenceParam {
            name: "mode".to_owned(),
            label: "Mode".to_owned(),
            required: true,
            sensitive: false,
            default_value: None,
            source: SequenceParamSource::User,
            candidate: None,
            hint: None,
        }],
        vec![SequenceStep {
            id: "set-mode".to_owned(),
            label: None,
            ensure_pdp_context_active: None,
            send: Some("AT+CFUN={{mode}}".to_owned()),
            expect: Some("OK".to_owned()),
            expect_prompt: None,
            expect_urc: None,
            payload: None,
            terminator: StepTerminator::None,
            require_tcp_ack: false,
            require_ping_success: false,
            timeout_secs: None,
            evidence: None,
            cleanup_on_failure: None,
        }],
    );
    assert!(
        !sequence
            .review
            .effects
            .contains(&crate::at::review::Effect::DeleteResetOrRecover)
    );
    let mut state = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "1,1".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    let buffer = rendered_buffer(&mut state, 120, 32);
    assert!(buffer.contains("What will happen:"));
    assert!(buffer.contains("Changes modem functionality"));
    assert!(buffer.contains("restart the modem"));
    assert!(buffer.contains("Persists / Recovery:"));
    assert!(buffer.contains("restart required"));
    assert!(buffer.contains("restart"));
    assert!(buffer.contains("Type `run` to run."));
    assert!(!buffer.contains("Assessment:"));
    assert!(!buffer.contains("Confirmation: required"));
}

#[test]
fn no_confirmation_sequence_uses_focus_review_before_execution() {
    let sequence = Sequence::built_in(
        "review-only",
        "Review a known observation before execution.",
        vec!["review".to_owned()],
        Some(30),
        vec![SequenceParam {
            name: "note".to_owned(),
            label: "Note".to_owned(),
            required: true,
            sensitive: false,
            default_value: None,
            source: SequenceParamSource::User,
            candidate: None,
            hint: None,
        }],
        vec![test_sequence_step("AT+CSQ")],
    );
    let mut state = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "review".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    assert_eq!(state.status, "Sequence ready for review.");
    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rendered = format!("{buffer:?}");
    assert!(rendered.contains("Review Sequence"));
    assert!(rendered.contains("Press Enter to run. Esc cancels."));
    assert!(!rendered.contains("Type `run`"));
    assert!(!rendered.contains("Input: <empty>"));
    let position = buffer_line_position(buffer, "Review Sequence").expect("review title");
    assert_eq!(
        buffer.cell(position).map(|cell| cell.fg),
        Some(Color::Rgb(0x4d, 0xd0, 0xe1))
    );

    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.confirmation_input.as_str()),
        Some("")
    );
    state.sequence_input.as_mut().expect("review input").error =
        Some("review state must remain unchanged".to_owned());
    handle_key_code(&mut state, KeyCode::Char('q'));
    handle_key_code(&mut state, KeyCode::Backspace);
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.confirmation_input.as_str()),
        Some("")
    );
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .and_then(|input| input.error.as_deref()),
        Some("review state must remain unchanged")
    );
    assert!(state.pending_execution.is_none());

    handle_key_code(&mut state, KeyCode::Enter);
    let pending = state.pending_execution.as_ref().expect("pending Sequence");
    assert!(!pending.confirmed);
}

#[test]
fn run_sequence_modal_runs_candidate_action_and_keeps_modal_open() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-unified-endpoint-tcp-send-check")
    .unwrap();
    let mut state = TuiState::new_with_all_usb(
        executable_items(crate::presets::builtin::builtins(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|item| item.name() == "soracom-unified-endpoint-tcp-send-check")
        .expect("SORACOM Sequence");
    state.timeout_override_secs = Some(75);
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.sequence_input.is_some());
    let pending = state.pending_execution.as_ref().expect("candidate action");
    assert_eq!(pending.item.name(), "active-pdp-contexts");
    assert_eq!(pending.timeout_secs, 75);

    execute_pending_command(&mut state, &mut executor);

    assert!(state.sequence_input.is_some());
    let buffer = rendered_buffer(&mut state, 140, 36);
    assert!(buffer.contains("Candidates: last direct PDP context check result (1 total"));
    assert!(buffer.contains("> 1  active"));
    assert!(buffer.contains("No modem read is performed by this modal."));
    assert!(buffer.contains("Actions:"));
    let active_action = line_containing(&buffer, "Run active-pdp-contexts");
    assert!(active_action.contains("Preset"));
    assert!(!active_action.contains("[confirm]"));
    let definitions_action = line_containing(&buffer, "Run pdp-contexts");
    assert!(definitions_action.contains("Preset"));
    assert!(!definitions_action.contains("[confirm]"));
    handle_key_code(&mut state, KeyCode::Down);
    let selected_action_buffer = rendered_buffer(&mut state, 140, 36);
    assert!(selected_action_buffer.contains("Enter: Run action."));
}

#[test]
fn run_sequence_modal_can_refresh_loaded_pdp_candidates() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-unified-endpoint-tcp-send-check")
    .unwrap();
    let mut state = TuiState::new_with_all_usb(
        executable_items(crate::presets::builtin::builtins(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|item| item.name() == "soracom-unified-endpoint-tcp-send-check")
        .expect("SORACOM Sequence");
    state.sequence_candidate_sets = vec![TuiSequenceCandidateSet {
        candidate: SequenceCandidateSource::PdpContext,
        candidates: vec![SequenceValueCandidate {
            value: "1".to_owned(),
            raw_label: "1  active".to_owned(),
            masked_label: "1  active".to_owned(),
        }],
        source_label: "last direct PDP context check result".to_owned(),
        acquired_at: "2026-07-04T00-00-00-000000000Z".to_owned(),
    }];

    handle_key_code(&mut state, KeyCode::Enter);

    let buffer = rendered_buffer(&mut state, 140, 36);
    assert!(buffer.contains("Candidates: last direct PDP context check result (1 total"));
    assert!(buffer.contains("> 1  active"));
    assert!(buffer.contains("Actions:"));
    assert!(line_containing(&buffer, "Run active-pdp-contexts").contains("Preset"));

    handle_key_code(&mut state, KeyCode::Down);
    let action_buffer = rendered_buffer(&mut state, 140, 36);
    let selected_action = line_containing(&action_buffer, "> Run active-pdp-contexts");
    assert!(selected_action.contains("Preset"));
    assert!(action_buffer.contains("Enter: Run action."));

    handle_key_code(&mut state, KeyCode::Enter);
    let pending = state.pending_execution.as_ref().expect("candidate action");
    assert_eq!(pending.item.name(), "active-pdp-contexts");
}

#[test]
fn run_sequence_modal_requires_confirmation_for_sms_list_status_change() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message command");

    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.pending_execution.is_none());
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::CandidateActionConfirmation)
    );
    let buffer = rendered_buffer(&mut state, 150, 36);
    assert!(buffer.contains("changes received-unread messages to received-read"));
    assert!(buffer.contains("Type `run` to run."));
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .map(|input| input.confirmation_input.as_str()),
        Some("q")
    );
}

#[test]
fn failed_candidate_action_is_not_rendered_as_selected_sequence_failure() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor {
        sequence_error: Some(AtctlError::Transport(
            "simulated candidate action failure".to_owned(),
        )),
        ..TestExecutor::default()
    };
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message command");

    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert!(state.sequence_input.is_some());
    let status_buffer = rendered_status_buffer(&mut state, 64, 12);
    assert!(status_buffer.contains("Status: failed"));
    assert!(status_buffer.contains("Action: sms-receive-check"));
    assert!(status_buffer.contains("Action result: failed"));
    assert!(!status_buffer.contains("Preset:"));
    assert!(!status_buffer.contains("Sequence: sms-read-message"));
    assert!(!status_buffer.contains("load-sms"));
    assert!(!status_buffer.contains("simulated candidate action failure"));

    assert!(
        state
            .response
            .contains("Result: failed\n\nAction failed before response.")
    );
    assert!(
        state
            .response
            .contains("simulated candidate action failure")
    );
    let modal_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(modal_buffer.contains("Candidate action failed."));
}

#[test]
fn run_sequence_modal_accepts_sms_index_candidate_and_opens_review() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message command");
    state.sequence_candidate_sets = vec![sms_candidate_set(vec![
        sms_candidate(
            "3",
            "REC UNREAD",
            "901001",
            "26/06/23,06:03:02+00",
            "hello! soracom",
        ),
        sms_candidate(
            "4",
            "REC READ",
            "901001",
            "26/06/23,08:09:02+00",
            "hey! SMS",
        ),
    ])];

    handle_key_code(&mut state, KeyCode::Enter);

    let buffer = rendered_buffer(&mut state, 150, 36);
    assert!(buffer.contains("SMS storage index"));
    assert!(buffer.contains("> unresolved  select"));
    assert!(buffer.contains("Candidates: last sms-receive-check result (2 total"));
    assert!(buffer.contains("No modem read is performed by this modal."));
    assert!(buffer.contains(
        "> storage=3  REC UNREAD  ******  26/06/23,06:03:02+00  <masked sensitive body>"
    ));
    assert!(
        buffer.contains(
            "  storage=4  REC READ  ******  26/06/23,08:09:02+00  <masked sensitive body>"
        )
    );
    assert!(buffer.contains("Actions:"));
    let sms_action = line_containing(&buffer, "Run sms-receive-check");
    assert!(sms_action.contains("Sequence"));
    assert!(sms_action.contains("[confirm]"));
    assert!(buffer.contains("Enter: Use value and review Sequence."));
    assert!(footer_text(&state, 150).contains("Enter Use and review"));

    handle_key_code(&mut state, KeyCode::Down);
    let moved_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(
        moved_buffer.contains(
            "> storage=4  REC READ  ******  26/06/23,08:09:02+00  <masked sensitive body>"
        )
    );

    handle_key_code(&mut state, KeyCode::Enter);
    assert_eq!(
        state
            .sequence_input
            .as_ref()
            .and_then(|input| input.values.first())
            .map(String::as_str),
        Some("4")
    );
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    assert!(state.pending_execution.is_none());
    let confirmation_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(confirmation_buffer.contains("Confirm Sequence"));
    assert!(confirmation_buffer.contains("Review 1-"));
    assert!(confirmation_buffer.contains("Resolved values:"));
    assert!(confirmation_buffer.contains("SMS storage index"));
    assert!(confirmation_buffer.contains("4"));
}

#[test]
fn run_sequence_modal_accepts_candidate_and_advances_to_next_value() {
    let sequence = crate::sequences::loader::parse_sequences(include_str!(
        "../../examples/sequences/soracom.toml"
    ))
    .unwrap()
    .into_iter()
    .find(|sequence| sequence.name == "soracom-ping-check")
    .unwrap();
    let mut state = TuiState::new_with_all_usb(
        executable_items(crate::presets::builtin::builtins(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|item| item.name() == "soracom-ping-check")
        .expect("SORACOM ping Sequence");
    state.sequence_candidate_sets = vec![TuiSequenceCandidateSet {
        candidate: SequenceCandidateSource::PdpContext,
        candidates: vec![SequenceValueCandidate {
            value: "2".to_owned(),
            raw_label: "2  active".to_owned(),
            masked_label: "2  active".to_owned(),
        }],
        source_label: "last direct PDP context check result".to_owned(),
        acquired_at: "2026-07-04T00-00-00-000000000Z".to_owned(),
    }];

    handle_key_code(&mut state, KeyCode::Enter);

    let first_value_buffer = rendered_buffer(&mut state, 140, 36);
    assert!(first_value_buffer.contains("Value 1 of 3"));
    assert!(first_value_buffer.contains("> 2  active"));
    assert!(first_value_buffer.contains("Enter: Use value and continue to Value 2 of 3."));
    assert!(footer_text(&state, 140).contains("Enter Use and next"));

    handle_key_code(&mut state, KeyCode::Enter);

    let input = state.sequence_input.as_ref().expect("Sequence input");
    assert_eq!(input.phase, SequenceInputPhase::Params);
    assert_eq!(input.active_param, 1);
    assert_eq!(input.values.first().map(String::as_str), Some("2"));
    assert!(state.pending_execution.is_none());
    let second_value_buffer = rendered_buffer(&mut state, 140, 36);
    assert!(second_value_buffer.contains("Value 2 of 3"));
    assert!(second_value_buffer.contains("Ping timeout"));
}

#[test]
fn run_sequence_modal_can_refresh_loaded_sms_candidates_with_confirmation() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-reply-check")
        .expect("sms-reply-check command");
    state.sequence_candidate_sets = vec![sms_candidate_set(vec![
        sms_candidate(
            "3",
            "REC READ",
            "901001",
            "26/06/23,06:03:02+00",
            "hello! soracom",
        ),
        sms_candidate(
            "4",
            "REC UNREAD",
            "901001",
            "26/07/04,05:36:29+00",
            "refresh me",
        ),
    ])];

    handle_key_code(&mut state, KeyCode::Enter);

    let buffer = rendered_buffer(&mut state, 150, 36);
    assert!(buffer.contains("Candidates: last sms-receive-check result (2 total"));
    assert!(buffer.contains("Actions:"));
    let sms_action = line_containing(&buffer, "Run sms-receive-check");
    assert!(sms_action.contains("Sequence"));
    assert!(sms_action.contains("[confirm]"));

    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Down);
    let action_buffer = rendered_buffer(&mut state, 150, 36);
    let selected_action = line_containing(&action_buffer, "> Run sms-receive-check");
    assert!(selected_action.contains("Sequence"));
    assert!(action_buffer.contains("Enter: Review action."));

    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.pending_execution.is_none());
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::CandidateActionConfirmation)
    );
    let confirmation_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(confirmation_buffer.contains("Confirm action"));
    assert!(confirmation_buffer.contains("Value 1 of 2"));
    assert!(confirmation_buffer.contains("What will happen:"));
    assert!(confirmation_buffer.contains("Lists stored SMS content"));
    assert!(confirmation_buffer.contains("Sensitive data:"));
    assert!(confirmation_buffer.contains("sensitive output"));
    assert!(confirmation_buffer.contains("Type `run` to run."));
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    let pending = state.pending_execution.as_ref().expect("candidate action");
    assert_eq!(pending.item.name(), "sms-receive-check");
    assert!(pending.confirmed);

    let mut executor = TestExecutor {
        sms_candidates: vec![sms_candidate(
            "8",
            "REC UNREAD",
            "901001",
            "26/07/29,08:15:00+00",
            "new message",
        )],
        ..TestExecutor::default()
    };
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(
        executor.calls.last().map(|(name, _)| name.as_str()),
        Some("sms-receive-check")
    );
    assert!(state.sequence_input.is_some());
    let refreshed = rendered_buffer(&mut state, 150, 36);
    assert!(refreshed.contains("Candidates: last sms-receive-check result (1 total"));
    assert!(refreshed.contains("storage=8"));
}

#[test]
fn candidate_action_rows_share_confirmation_lane_and_preserve_tag_role() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message command");
    handle_key_code(&mut state, KeyCode::Enter);
    let input = state.sequence_input.as_mut().expect("Sequence input");
    input.active_candidate = 1;

    let actions = [
        SequenceCandidateAction {
            provider: SequenceCandidateProvider {
                kind: SequenceCandidateProviderKind::Preset,
                name: "inspect-modem",
            },
            item: Box::new(ExecutableItem::Preset(Preset::new(
                "inspect-modem",
                "AT",
                vec!["review".to_owned()],
                PresetOrigin::BuiltIn,
            ))),
        },
        SequenceCandidateAction {
            provider: SequenceCandidateProvider {
                kind: SequenceCandidateProviderKind::Preset,
                name: "apply-known-change",
            },
            item: Box::new(ExecutableItem::Preset(Preset::new(
                "apply-known-change",
                "AT+CMEE=2",
                vec!["review".to_owned()],
                PresetOrigin::BuiltIn,
            ))),
        },
        SequenceCandidateAction {
            provider: SequenceCandidateProvider {
                kind: SequenceCandidateProviderKind::Preset,
                name: "unknown-vendor-action",
            },
            item: Box::new(ExecutableItem::Preset(Preset::new(
                "unknown-vendor-action",
                "AT+VENDOR=1",
                vec!["review".to_owned()],
                PresetOrigin::BuiltIn,
            ))),
        },
    ];
    let theme = TuiTheme::dark();
    let mut lines = Vec::new();
    push_sequence_candidate_action_rows(&mut lines, input, &actions, 0, 64, &theme);
    let plain = lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let known_tag = plain[1].find("[confirm]").expect("known tag");
    let unknown_tag = plain[2].find("[confirm]").expect("unknown tag");
    assert_eq!(known_tag, unknown_tag);
    assert!(display_cell_width(&plain[0]) <= known_tag);

    let selected_name = lines[1].spans.first().expect("selected action name");
    assert_eq!(
        selected_name.style.fg,
        theme.style(TuiStyleRole::Selected).fg
    );
    let tag = lines[1]
        .spans
        .iter()
        .find(|span| span.content.as_ref() == "[confirm]")
        .expect("confirmation tag span");
    assert_eq!(tag.style.fg, theme.style(TuiStyleRole::ConfirmationFlow).fg);
}

#[test]
fn run_sequence_modal_keeps_selected_sms_candidate_visible_when_list_overflows() {
    let mut state = test_state_with_sequences();
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message command");
    state.sequence_candidate_sets = vec![sms_candidate_set(
        (0..8)
            .map(|index| {
                sms_candidate(
                    &index.to_string(),
                    "REC READ",
                    "901001",
                    "26/06/24,11:16:08+00",
                    &format!("message {index}"),
                )
            })
            .collect(),
    )];

    handle_key_code(&mut state, KeyCode::Enter);
    for _ in 0..7 {
        handle_key_code(&mut state, KeyCode::Down);
    }

    let buffer = rendered_buffer(&mut state, 150, 36);
    assert!(buffer.contains("Candidates: last sms-receive-check result (8 total"));
    assert!(buffer.contains("Candidate rows 4-8 of 8"));
    assert!(buffer.contains("> storage=7  REC READ  ******  26/06/24,11:16:08+00"));
    assert!(!buffer.contains("> storage=0  REC READ"));
}

#[test]
fn completed_sms_receive_sequence_updates_candidates_for_next_select_modal() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor {
        sms_candidates: vec![sms_candidate(
            "5",
            "REC UNREAD",
            "901001",
            "26/06/24,11:16:08+00",
            "new message",
        )],
        ..TestExecutor::default()
    };
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-receive-check")
        .expect("sms-receive-check command");

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(
        state
            .sequence_candidate_sets
            .iter()
            .find(|set| set.candidate == SequenceCandidateSource::SmsMessage)
            .expect("sms candidate set")
            .candidates
            .first()
            .map(|candidate| candidate.value.as_str()),
        Some("5")
    );
    assert_eq!(
        state
            .sequence_candidate_sets
            .iter()
            .find(|set| set.candidate == SequenceCandidateSource::SmsMessage)
            .expect("sms candidate set")
            .source_label,
        "last sms-receive-check result"
    );

    state.selected_command = state
        .visible_commands()
        .iter()
        .position(|command| command.name() == "sms-read-message")
        .expect("sms-read-message command");
    handle_key_code(&mut state, KeyCode::Enter);

    let buffer = rendered_buffer(&mut state, 150, 36);
    assert!(buffer.contains("Candidates: last sms-receive-check result (1 total"));
    assert!(buffer.contains(
        "> storage=5  REC UNREAD  ******  26/06/24,11:16:08+00  <masked sensitive body>"
    ));
}

#[test]
fn completed_cmgl_command_updates_sms_index_candidates() {
    let mut state = test_state_with_sequences();
    let operation = OperationIdentity::command("AT+CMGF=1");
    let review = review_direct_command("AT+CMGF=1");
    let raw_text = "AT+CMGL=\"ALL\"\r\n+CMGL: 6,\"REC READ\",\"901001\",,\"26/06/24,12:00:00+00\"\r\n00640069007200650063007400200063006F006D006D0061006E0064\r\n\r\nOK\r\n".to_owned();
    let masked_text = mask_sensitive_values(&raw_text);
    let output = ExecutionOutput::Command(Box::new(SendExecution {
        surface: ExecutionSurface::Tui,
        masked_operation: operation.masked(),
        operation,
        masked_review: review.masked(),
        review,
        status: AtStatus::Ok,
        text: masked_text.clone(),
        lines: Vec::new(),
        raw_response: Vec::new(),
        raw_text,
        masked_text,
        masked: true,
        duration: Duration::from_millis(7),
    }));

    update_sequence_candidates_from_execution(&mut state, &output);

    let candidate_set = state
        .sequence_candidate_sets
        .iter()
        .find(|set| set.candidate == SequenceCandidateSource::SmsMessage)
        .expect("sms candidate set");
    assert_eq!(candidate_set.len(), 1);
    assert_eq!(candidate_set.source_label, "last direct AT+CMGL result");
    let candidate = &candidate_set.candidates[0];
    assert_eq!(candidate.value, "6");
    assert!(candidate.raw_label.contains("direct command"));
    assert!(candidate.masked_label.contains("******"));
}

#[test]
fn tui_response_copy_uses_the_execution_short_phone_pair() {
    let mut state = test_state_with_sequences();
    let preset = Preset::new("own-number", "AT+CNUM", Vec::new(), PresetOrigin::BuiltIn);
    let item = ExecutableItem::Preset(preset);
    let operation = OperationIdentity::command("AT+CNUM");
    let review = review_direct_command("AT+CNUM");
    let raw_text = "+CNUM: \"line\",\"901001\",129\n+CMGS: 7\nOK\n".to_owned();
    let masked_text = mask_sensitive_values(&raw_text);
    let execution = SendExecution {
        surface: ExecutionSurface::Tui,
        masked_operation: operation.masked(),
        operation,
        masked_review: review.masked(),
        review,
        status: AtStatus::Ok,
        text: masked_text.clone(),
        lines: masked_text.lines().map(str::to_owned).collect(),
        raw_text: raw_text.clone(),
        masked_text,
        raw_response: raw_text.as_bytes().to_vec(),
        masked: true,
        duration: Duration::from_millis(7),
    };

    finish_execution_result(
        &mut state,
        item,
        30,
        Ok(ExecutionOutput::Command(Box::new(execution))),
    );

    let masked_copy = copyable_response_text_for_masking(&state, true).unwrap();
    assert!(masked_copy.contains("\"******\",129"));
    assert!(masked_copy.contains("+CMGS: 7"));
    assert!(!masked_copy.contains("901001"));
    let raw_copy = copyable_response_text_for_masking(&state, false).unwrap();
    assert!(raw_copy.contains("\"901001\",129"));
}

#[test]
fn raw_capture_for_tui_sequence_uses_sequence_step_events() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor::default();
    let path = unique_temp_dir("sequence-raw-capture").join("case.rawlog");
    state.raw_capture =
        Some(RawLogSink::create(RawLogConfig::new(path.clone(), ExecutionSurface::Tui)).unwrap());
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "+819012345678".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "hello from atctl".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    execute_pending_command(&mut state, &mut executor);

    assert!(state.raw_capture.is_some());
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"surface\":\"tui\""));
    assert!(raw.contains("\"command_name\":\"test-sequence-step\""));
    assert!(raw.contains("\"command\":\"AT+CMGF=1\""));
    assert!(!raw.contains("\"command\":\"SEQUENCE sms-send-check\""));
}

#[test]
fn edit_before_run_executes_edited_command_through_safety_flow() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    run_control(&mut state, ControlAction::EditCommand);
    for _ in 0.."AT".len() {
        handle_key_code(&mut state, KeyCode::Backspace);
    }
    for value in "AT+CSQ".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.edit_input.is_none());
    assert!(state.pending_execution.is_some());
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("edited".to_owned(), false)]);
    assert!(state.response.contains("AT+CSQ"));
    assert_eq!(
        state
            .active_command
            .as_ref()
            .and_then(|command| command.source_title.as_deref()),
        None
    );
}

#[test]
fn edit_before_run_material_command_requires_confirmation() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    run_control(&mut state, ControlAction::EditCommand);
    for _ in 0.."AT".len() {
        handle_key_code(&mut state, KeyCode::Backspace);
    }
    for value in "ATE0".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.confirmation.is_some());
    assert!(state.pending_execution.is_none());
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("edited".to_owned(), true)]);
}

#[test]
fn edited_command_keeps_selected_item_timeout() {
    let preset = Preset::new_with_timeout(
        "long-read",
        "AT+COPS=?",
        vec!["network".to_owned()],
        PresetOrigin::BuiltIn,
        Some(240),
    );
    let mut state = TuiState::new_with_all_usb(
        executable_items(vec![preset], Vec::new()),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    run_control(&mut state, ControlAction::EditCommand);
    assert_eq!(
        state.edit_input.as_ref().map(|input| input.timeout_secs),
        Some(240)
    );
    for _ in 0.."AT+COPS=?".len() {
        handle_key_code(&mut state, KeyCode::Backspace);
    }
    for value in "AT+CSQ".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(
        state
            .pending_execution
            .as_ref()
            .map(|pending| pending.timeout_secs),
        Some(240)
    );
}

#[test]
fn response_enter_opens_actions_without_running_selected_command() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    state.focus = Pane::Response;
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert!(state.action_menu.is_some());
    assert!(state.pending_execution.is_none());
    assert!(executor.calls.is_empty());
}

#[test]
fn categories_enter_moves_to_commands_without_running_selected_command() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    state.focus = Pane::Categories;
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(state.focus, Pane::Commands);
    assert!(state.pending_execution.is_none());
    assert!(executor.calls.is_empty());
}

#[test]
fn response_export_request_follows_visible_masking_and_names_the_response() {
    let mut state = unmasked_sensitive_state();
    state.active_command = Some(CommandStatus::new(
        CommandRunState::Completed,
        &ExecutableItem::Preset(Preset::built_in("imsi", "AT+CIMI", ["sim"])),
        DEFAULT_COMMAND_TIMEOUT_SECS,
        StatusSummary::Completed {
            status: "OK".to_owned(),
            duration_ms: 4,
        },
    ));

    state.output_masking_enabled = true;
    let masked = response_export_request(&state).expect("masked export request");
    assert!(masked.masked);
    assert!(masked.contents.contains("89811000*******"));
    assert!(!masked.contents.contains("898110001234567"));
    assert!(masked.file_name.starts_with("command-imsi-2026-"));
    assert!(masked.file_name.ends_with(".response.txt"));

    state.output_masking_enabled = false;
    let unmasked = response_export_request(&state).expect("unmasked export request");
    assert!(!unmasked.masked);
    assert!(unmasked.contents.contains("898110001234567"));

    open_response_action_menu(&mut state);
    let context = action_menu_context_lines(&state, ActionMenuKind::Response);
    assert!(context.contains(&"Export content: unmasked".to_owned()));
    assert!(!context.iter().any(|line| line.starts_with("Warning:")));
}

#[test]
fn response_export_masks_a_sensitive_command_when_response_has_no_echo() {
    let mut state = test_state();
    let item = ExecutableItem::Preset(Preset::built_in(
        "pdp-auth",
        "AT+CGAUTH=1,1,\"user\",\"password\"",
        ["pdp"],
    ));
    state.active_command = Some(CommandStatus::new(
        CommandRunState::Completed,
        &item,
        DEFAULT_COMMAND_TIMEOUT_SECS,
        StatusSummary::Completed {
            status: "OK".to_owned(),
            duration_ms: 2,
        },
    ));
    state.response = ResponseState::masked("OK");
    state.output_masking_enabled = true;

    let request = response_export_request(&state).unwrap();

    assert!(!request.contents.contains("user"));
    assert!(!request.contents.contains("password"));
    assert!(request.contents.contains("AT+CGAUTH=1,1"));

    state.output_masking_enabled = false;
    let unmasked = response_export_request(&state).unwrap();
    assert!(!unmasked.masked);
    assert!(unmasked.contents.contains("user"));
    assert!(unmasked.contents.contains("password"));

    let rows = response_action_rows(&state);
    assert!(rows.iter().any(|row| {
        row.action == ActionMenuAction::CopyResponse && row.label == "Copy unmasked response"
    }));
    assert!(rows.iter().any(|row| {
        row.action == ActionMenuAction::ExportResponse && row.label == "Export unmasked response..."
    }));
}

#[test]
fn response_actions_without_body_offer_no_response_operations() {
    let mut state = test_state();
    state.log_paths = test_logging_paths("empty-response-actions");
    state.response.clear();
    open_response_action_menu(&mut state);

    let rows = action_menu_rows(&state, ActionMenuKind::Response);

    assert!(rows.is_empty());
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::CopyResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::ExportResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::ClearResponse)
    );
}

#[test]
fn response_actions_show_export_contract_without_fixed_folder_action() {
    let mut state = test_state();
    state.response = ResponseState::masked("AT\nOK");
    open_response_action_menu(&mut state);
    let rows = action_menu_rows(&state, ActionMenuKind::Response);

    assert!(rows.iter().any(|row| {
        row.action == ActionMenuAction::ExportResponse && row.label == "Export response..."
    }));
    let context = action_menu_context_lines(&state, ActionMenuKind::Response);
    assert!(context.contains(&"Export format: UTF-8 text".to_owned()));
    assert!(context.contains(&"Export content: masked".to_owned()));
    assert!(context.iter().any(|line| line.starts_with("File name: ")));
    assert!(!context.iter().any(|line| line.contains("responses folder")));
    assert!(!rows.iter().any(|row| row.label.contains("path")));
    assert!(!rows.iter().any(|row| row.label.contains("dir")));
}

#[test]
fn material_command_opens_confirmation_without_executing() {
    let mut state = test_state();
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.confirmation.is_some());
    assert!(state.pending_execution.is_none());
    assert!(state.response.contains("Command requires confirmation"));
    assert!(state.response.contains("What will happen:"));
    assert!(!state.response.contains("Assessment:"));
    assert!(!state.response.contains("Confirmation: required"));
    assert!(!state.response.contains("Risk:"));
    assert!(!state.response.contains("CONFIRM"));
}

#[test]
fn confirmation_mismatch_does_not_execute() {
    let mut state = test_state();
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "abc".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.pending_execution.is_none());
    assert!(state.confirmation.is_none());
    assert!(state.response.contains("Command was not sent"));
}

#[test]
fn confirmation_match_executes_material_command() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "run".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(executor.calls.is_empty());
    assert!(state.pending_execution.is_some());
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(
        executor.calls,
        vec![("disable-command-echo".to_owned(), true)]
    );
    assert!(state.confirmation.is_none());
    assert!(state.status.contains("status=OK"));
}

#[test]
fn material_commands_are_visible_and_confirmation_required() {
    let state = test_state();
    let visible_names = state
        .visible_commands()
        .iter()
        .map(|preset| preset.name())
        .collect::<Vec<_>>();

    assert!(visible_names.contains(&"material-reset"));

    let material = state
        .visible_commands()
        .into_iter()
        .find(|preset| preset.name() == "material-reset")
        .expect("material command should be visible");
    assert!(material.review().requires_confirmation());
}

#[test]
fn material_commands_remain_visible_when_category_selection_is_invalid() {
    let mut state = test_state();
    state.selected_category = usize::MAX;
    let visible_names = state
        .visible_commands()
        .iter()
        .map(|preset| preset.name())
        .collect::<Vec<_>>();

    assert!(visible_names.contains(&"material-reset"));
}

#[test]
fn renders_required_panes() {
    let mut state = test_state();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    for expected in [
        "Devices",
        "Status",
        "Categories",
        "Commands",
        "Response",
        "Logs",
    ] {
        assert!(buffer.contains(expected), "{expected}");
    }
    assert!(!buffer.contains("\"History\""));
}

#[test]
fn device_detail_uses_usb_descriptor_fields_without_known_profile_label() {
    let device = test_usb_device(1, 3, "Onyx A");
    let lines = device_detail_lines(&device, &TuiTheme::dark())
        .into_iter()
        .map(|line| format!("{line:?}"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!lines.contains("Known:"));
    assert!(!lines.contains("Quectel EG25-G / SORACOM Onyx"));
    assert!(lines.contains("Manufacturer: Quectel"));
    assert!(lines.contains("Product: Onyx A"));
    assert!(lines.contains("VID: 0x2c7c"));
    assert!(lines.contains("PID: 0x0125"));
    assert!(lines.contains("Bus: 1"));
    assert!(lines.contains("Address: 3"));
}

#[test]
fn timeout_budget_label_keeps_noun_until_width_requires_fallback() {
    assert_eq!(
        timeout_budget_label(33, 180, 147, 28),
        "Timeout 33/180s left 147s"
    );
    assert_eq!(timeout_budget_label(33, 180, 147, 17), "33/180s left 147s");
    assert_eq!(timeout_budget_label(33, 180, 147, 7), "33/180s");
    assert_eq!(timeout_budget_label(33, 180, 147, 6), "");
}

#[test]
fn running_status_shows_timeout_budget_feedback() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in(
        "available-operators",
        "AT+COPS=?",
        ["network"],
    ));
    state.active_command = Some(CommandStatus::new(
        CommandRunState::Running,
        &command,
        DEFAULT_COMMAND_TIMEOUT_SECS,
        StatusSummary::None,
    ));
    state.running_execution = Some(RunningExecution {
        started_at: Instant::now() - Duration::from_secs(7),
        timeout: Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
    });
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());
    let status_buffer = rendered_status_buffer(&mut state, 80, 12);

    assert!(buffer.contains("Command:"));
    assert!(status_buffer.contains("Command: available-operators"));
    assert!(status_buffer.contains("AT command: AT+COPS=?"));
    assert!(buffer.contains("AT+COPS=?"));
    assert!(buffer.contains("30s"));
    assert!(buffer.contains("Timeout "));
    assert!(buffer.contains("/30s left "));
    assert!(!buffer.contains("Timeout: 30s"));
    assert!(buffer.contains("left"));
    assert!(!buffer.contains("Elapsed"));
    assert!(!buffer.contains("remaining"));
    assert!(buffer.contains("Timeout "));
    assert!(buffer.contains(symbols::block::FULL));
    assert!(buffer.contains(symbols::shade::LIGHT));
}

#[test]
fn running_command_blocks_new_actions_without_quitting() {
    let mut pending = test_state();
    handle_key_code(&mut pending, KeyCode::Enter);
    assert!(pending.pending_execution.is_some());
    assert_eq!(
        handle_key_code(&mut pending, KeyCode::Char('q')),
        TuiAction::Continue
    );
    assert!(pending.pending_execution.is_some());

    let mut state = test_state();
    state.running_execution = Some(RunningExecution {
        started_at: Instant::now(),
        timeout: Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
    });

    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('q')),
        TuiAction::Continue
    );
    assert!(state.status.contains("Operation is running"));
    assert!(state.running_execution.is_some());
}

#[test]
fn timeout_input_sets_session_execution_timeout() {
    let mut state = test_state();

    run_control(&mut state, ControlAction::SetTimeout);
    assert_eq!(
        state
            .timeout_input
            .as_ref()
            .map(|input| input.input.as_str()),
        Some("")
    );
    let modal = rendered_buffer(&mut state, 120, 32);
    assert!(modal.contains("Timeout"));
    assert!(
        modal.contains("Set the timeout for all Commands and Sequences until this TUI closes.")
    );
    assert!(modal.contains("Leave blank to use each Command or Sequence timeout."));
    assert!(modal.contains("Seconds: <empty>"));
    assert!(footer_text(&state, 120).contains("Enter Apply"));
    assert!(footer_text(&state, 120).contains("Esc Cancel"));
    assert!(!modal.contains("default"));
    assert!(!modal.contains("override"));
    for value in "180".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.timeout_override_secs, Some(180));
    assert_eq!(
        state
            .controls_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Timeout set to 180 seconds.")
    );
    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Enter);
    let pending = state.pending_execution.as_ref().expect("pending execution");
    assert_eq!(pending.timeout_secs, 180);
}

#[test]
fn blank_timeout_input_restores_each_item_timeout() {
    let item = ExecutableItem::Preset(Preset::new_with_timeout(
        "long-scan",
        "AT+COPS=?",
        vec!["network".to_owned()],
        PresetOrigin::BuiltIn,
        Some(240),
    ));
    let mut state = TuiState::new_with_all_usb(
        vec![item.clone()],
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.timeout_override_secs = Some(120);

    run_control(&mut state, ControlAction::SetTimeout);
    for _ in 0.."120".len() {
        handle_key_code(&mut state, KeyCode::Backspace);
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.timeout_override_secs, None);
    assert_eq!(effective_timeout_secs(&state, &item), 240);
    assert_eq!(
        state
            .controls_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Using each Command or Sequence timeout.")
    );
    let compact = rendered_buffer(&mut state, 80, 24);
    assert!(compact.contains("Using each Command or Sequence"));
    assert!(compact.contains("timeout."));
}

#[test]
fn cancelling_timeout_input_changes_neither_timeout_nor_status() {
    let mut state = test_state();
    state.timeout_override_secs = Some(75);
    state.status = "Ready before timeout input.".to_owned();
    state.status_role = TuiStyleRole::Status;

    run_control(&mut state, ControlAction::SetTimeout);
    handle_key_code(&mut state, KeyCode::Backspace);
    handle_key_code(&mut state, KeyCode::Char('9'));
    handle_key_code(&mut state, KeyCode::Esc);

    assert_eq!(state.timeout_override_secs, Some(75));
    assert_eq!(state.status, "Ready before timeout input.");
    assert!(state.timeout_input.is_none());
}

#[test]
fn single_visible_device_is_auto_selected_for_execution() {
    let mut state = TuiState::new(
        test_commands(),
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    assert_eq!(state.active_device, Some(0));
    assert_eq!(state.focus, Pane::Commands);

    handle_key_code(&mut state, KeyCode::Enter);

    let pending = state.pending_execution.as_ref().expect("pending execution");
    assert_eq!(
        pending.device_selection,
        Some(TuiDeviceSelection {
            vendor_id: 0x2c7c,
            product_id: 0x0125,
            bus: 1,
            address: 3,
        })
    );
}

#[test]
fn no_visible_device_blocks_device_dependent_actions() {
    let mut state = TuiState::new(test_commands(), Vec::new(), Vec::new(), TuiTheme::dark());

    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.pending_execution.is_none());
    assert!(state.response.contains("No matching USB device"));
    assert!(state.response.contains("Review Devices and all USB"));
    assert!(state.response.contains("restart `atctl tui` to rescan"));
    assert_eq!(state.status_role, TuiStyleRole::Warning);
    let buffer = rendered_buffer(&mut state, 120, 32);
    assert!(buffer.contains("Review Devices and all USB"), "{buffer}");
    assert!(buffer.contains("restart `atctl tui` to rescan"), "{buffer}");

    run_control(&mut state, ControlAction::AdHocCommand);

    assert!(state.ad_hoc_input.is_none());
    assert!(state.response.contains("No matching USB device"));
    assert!(state.response.contains("restart `atctl tui` to rescan"));
}

#[test]
fn multiple_visible_devices_require_explicit_selection_before_execution() {
    let mut state = test_state();
    state.devices = vec![
        test_usb_device(1, 3, "Onyx A"),
        test_usb_device(2, 4, "Onyx B"),
    ];
    state.active_device = None;
    state.highlighted_device = 0;
    state.focus = Pane::Devices;

    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.pending_execution.is_none());
    assert!(state.status.contains("Select a USB device"));
    assert_eq!(state.focus, Pane::Devices);
    assert!(!state.response.contains("restart `atctl tui` to rescan"));
    assert!(!rendered_buffer(&mut state, 120, 32).contains("restart `atctl tui` to rescan"));

    handle_key_code(&mut state, KeyCode::Down);
    assert_eq!(state.highlighted_device, 1);
    handle_key_code(&mut state, KeyCode::Enter);
    assert_eq!(state.active_device, Some(1));
    assert_eq!(state.focus, Pane::Commands);

    handle_key_code(&mut state, KeyCode::Enter);

    let pending = state.pending_execution.as_ref().expect("pending execution");
    assert_eq!(
        pending.device_selection,
        Some(TuiDeviceSelection {
            vendor_id: 0x2c7c,
            product_id: 0x0125,
            bus: 2,
            address: 4,
        })
    );
}

#[test]
fn all_usb_troubleshooting_view_keeps_non_targets_diagnostic_only() {
    for theme in [TuiTheme::dark(), TuiTheme::light(), TuiTheme::no_color()] {
        let mut state = TuiState::new_with_all_usb(
            executable_items(test_commands(), Vec::new()),
            Vec::new(),
            vec![test_diagnostic_usb_device(1, 2, "USB Hub")],
            Vec::new(),
            theme,
        );

        state.focus = Pane::Devices;
        state.highlighted_device = state.devices.len();
        handle_key_code(&mut state, KeyCode::Enter);

        assert_eq!(state.device_view, DeviceView::AllUsbTroubleshooting);
        let lines = device_lines(&state, &state.theme)
            .into_iter()
            .map(|line| format!("{line:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(lines.contains("All USB: 1"));
        assert!(lines.contains("[diagnostic-only]"));

        handle_key_code(&mut state, KeyCode::Enter);

        assert_eq!(state.active_device, None);
        assert!(state.pending_execution.is_none());
        assert!(state.status.contains("Diagnostic-only USB device"));
        assert!(state.response.contains("not an atctl operation target"));
        assert!(
            !state
                .response
                .contains("Known default-detection limitation")
        );
        assert!(!state.response.contains("atctl inspect --bus"));

        let compact = rendered_buffer(&mut state, 120, 32);
        assert!(compact.contains("Diagnostic-only USB device"));
        assert!(!compact.contains("Known default-detection limitation"));

        let expanded = rendered_buffer(&mut state, 200, 48);
        assert!(expanded.contains("not an atctl operation target"));
        assert!(!expanded.contains("Known default-detection limitation"));
    }
}

#[test]
fn class_zero_diagnostic_only_device_shows_known_limit_in_response() {
    for theme in [TuiTheme::dark(), TuiTheme::light(), TuiTheme::no_color()] {
        let mut state = TuiState::new_with_all_usb(
            executable_items(test_commands(), Vec::new()),
            Vec::new(),
            vec![test_class_zero_diagnostic_usb_device(
                1,
                2,
                "Composite USB device",
            )],
            Vec::new(),
            theme,
        );

        state.focus = Pane::Devices;
        state.highlighted_device = state.devices.len();
        handle_key_code(&mut state, KeyCode::Enter);

        let lines = device_lines(&state, &state.theme)
            .into_iter()
            .map(|line| format!("{line:?}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(lines.contains("[diagnostic-only]"));

        handle_key_code(&mut state, KeyCode::Enter);

        assert_eq!(state.active_device, None);
        assert!(state.pending_execution.is_none());
        assert_eq!(
            state.status,
            "Diagnostic-only USB device; AT sending is disabled."
        );
        assert!(
            !state
                .status
                .contains("AT support for this device is unconfirmed")
        );
        assert!(
            state
                .response
                .contains("Known default-detection limitation")
        );
        assert!(
            state
                .response
                .contains("AT support for this device is unconfirmed")
        );
        assert!(state.response.contains(
            "Do not send AT unless vendor documentation or an independently validated device configuration identifies the AT interface and bulk endpoints."
        ));
        assert!(state.response.contains("atctl inspect --bus 1 --address 2"));
        assert!(
            state
                .response
                .contains("https://github.com/uchimanajet7/atctl/issues")
        );
        assert!(!state.response.contains("atctl send AT"));

        let compact = rendered_buffer(&mut state, 120, 32);
        assert!(compact.contains("Known default-detection limitation"));

        let expanded = rendered_buffer(&mut state, 160, 48);
        assert!(expanded.contains("Known default-detection limitation"));
        assert!(expanded.contains("atctl inspect --bus 1 --address 2"));
    }
}

#[test]
fn all_usb_troubleshooting_view_can_select_visible_operation_target() {
    for theme in [TuiTheme::dark(), TuiTheme::light(), TuiTheme::no_color()] {
        let target = test_usb_device(1, 3, "Onyx A");
        let mut state = TuiState::new_with_all_usb(
            executable_items(test_commands(), Vec::new()),
            vec![target.clone()],
            vec![test_diagnostic_usb_device(1, 2, "USB Hub"), target],
            Vec::new(),
            theme,
        );
        state.active_device = None;

        state.focus = Pane::Devices;
        state.highlighted_device = state.devices.len();
        handle_key_code(&mut state, KeyCode::Enter);
        handle_key_code(&mut state, KeyCode::Down);
        handle_key_code(&mut state, KeyCode::Enter);

        assert_eq!(state.active_device, Some(0));
        assert_eq!(state.device_view, DeviceView::OperationTargets);
        assert_eq!(state.focus, Pane::Commands);
        assert!(state.status.contains("Selected USB device"));

        for (width, height) in [(120, 32), (160, 48)] {
            let buffer = rendered_buffer(&mut state, width, height);
            assert!(buffer.contains("Onyx A"));
            assert!(!buffer.contains("Known default-detection limitation"));
        }
    }
}

#[test]
fn device_list_keeps_lower_highlighted_item_visible() {
    let devices = (0..12)
        .map(|index| test_usb_device(1, index as u8, &format!("Target {index:02}")))
        .collect::<Vec<_>>();
    let mut state = TuiState::new(test_commands(), devices, Vec::new(), TuiTheme::dark());
    state.focus = Pane::Devices;
    state.highlighted_device = 11;

    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Target 11"));
    assert!(!buffer.contains("Target 00"));
}

#[test]
fn category_list_keeps_lower_selected_item_visible() {
    let commands = (0..18)
        .map(|index| {
            Preset::built_in(
                format!("cmd-{index:02}"),
                "AT",
                [format!("category-{index:02}")],
            )
        })
        .collect::<Vec<_>>();
    let mut state = TuiState::new(
        commands,
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.focus = Pane::Categories;
    state.selected_category = state
        .categories
        .iter()
        .position(|category| category == "category-17")
        .expect("category-17 category");

    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("category-17"));
    assert!(!buffer.contains("category-00"));
}

#[test]
fn command_list_keeps_lower_selected_item_visible() {
    let commands = (0..18)
        .map(|index| Preset::built_in(format!("cmd-{index:02}"), "AT", ["bulk"]))
        .collect::<Vec<_>>();
    let mut state = TuiState::new(
        commands,
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.focus = Pane::Commands;
    state.selected_command = 17;

    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("cmd-17"));
    assert!(!buffer.contains("cmd-00"));
}

#[test]
fn logs_list_keeps_lower_selected_item_visible() {
    let logs = (0..12)
        .map(|index| {
            test_log_entry(
                LogListingKind::Session,
                format!("/tmp/{index:02}.session.log"),
            )
        })
        .collect::<Vec<_>>();
    let mut state = TuiState::new(
        test_commands(),
        vec![test_usb_device(1, 3, "Onyx A")],
        logs,
        TuiTheme::dark(),
    );
    state.focus = Pane::History;
    state.selected_log = 11;

    let buffer = rendered_buffer(&mut state, 100, 12);

    assert!(buffer.contains("11.session.log"));
    assert!(!buffer.contains("00.session.log"));
}

#[test]
fn logs_pane_labels_mixed_history_and_sessions_as_saved_logs() {
    let mut state = test_state();
    state.logs = vec![
        test_log_entry(LogListingKind::History, "/tmp/history.jsonl"),
        test_log_entry(LogListingKind::Session, "/tmp/new.session.log"),
    ];
    state.focus = Pane::History;

    let buffer = rendered_buffer(&mut state, 100, 20);

    assert!(buffer.contains("Saved logs:"));
    assert!(!buffer.contains("Recent logs:"));
}

#[test]
fn log_entries_use_resolved_session_log_directory() {
    let dir = unique_temp_dir("resolved-log-dir");
    let state_dir = dir.join("state");
    let resolved_log_dir = dir.join("resolved-logs");
    let default_log_dir = state_dir.join("logs");
    fs::create_dir_all(&state_dir).unwrap();
    fs::create_dir_all(&resolved_log_dir).unwrap();
    fs::create_dir_all(&default_log_dir).unwrap();
    fs::write(state_dir.join("history.jsonl"), "{}\n").unwrap();
    fs::write(
        resolved_log_dir.join("2026-07-03T00-00-00Z.session.log"),
        "resolved",
    )
    .unwrap();
    fs::write(
        default_log_dir.join("2026-07-03T01-00-00Z.session.log"),
        "default",
    )
    .unwrap();

    let entries = log_entries_from_paths(&LoggingPaths {
        state_dir,
        session_dir: resolved_log_dir,
    })
    .unwrap();
    let labels = entries
        .iter()
        .map(|entry| entry.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "history: history.jsonl",
            "session: 2026-07-03T00-00-00Z.session.log"
        ]
    );
}

#[test]
fn refresh_log_summaries_adds_new_session_logs_without_restart() {
    let dir = unique_temp_dir("refresh-log-list");
    let state_dir = dir.join("state");
    let session_dir = dir.join("logs");
    fs::create_dir_all(&state_dir).unwrap();
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(state_dir.join("history.jsonl"), "{}\n").unwrap();
    let paths = LoggingPaths {
        state_dir,
        session_dir,
    };
    let mut state = test_state();
    refresh_log_summaries_from_paths(&mut state, &paths).unwrap();
    assert_eq!(state.logs.len(), 1);

    fs::write(
        paths.session_dir.join("2026-07-03T02-00-00Z.session.log"),
        "session",
    )
    .unwrap();
    state.selected_log = 99;
    refresh_log_summaries_from_paths(&mut state, &paths).unwrap();

    assert_eq!(state.logs_error, None);
    assert_eq!(state.selected_log, 1);
    assert_eq!(state.logs.len(), 2);
    assert_eq!(
        state.logs[1].label,
        "session: 2026-07-03T02-00-00Z.session.log"
    );
}

#[test]
fn opening_log_actions_refreshes_externally_deleted_logs() {
    let paths = test_logging_paths("log-actions-deleted-before-menu");
    let session_log = paths.session_dir.join("2026-07-03T02-30-00Z.session.log");
    fs::write(&session_log, "session").unwrap();
    let mut state = test_state();
    state.log_paths = paths.clone();
    refresh_log_summaries(&mut state).unwrap();
    assert_eq!(state.logs.len(), 1);

    fs::remove_file(&session_log).unwrap();
    state.focus = Pane::History;
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.action_menu.is_some());
    assert!(state.logs.is_empty());
    assert_eq!(state.logs_error, None);
    let rows = action_menu_rows(&state, ActionMenuKind::Log);
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    assert!(rows.is_empty());
    assert!(
        state
            .action_menu
            .as_ref()
            .and_then(|menu| menu.feedback.as_ref())
            .map(|feedback| feedback.message.contains("Selected log no longer exists"))
            .unwrap_or(false)
    );
}

#[test]
fn opening_log_actions_does_not_retarget_deleted_session_to_history() {
    let paths = test_logging_paths("log-actions-deleted-session-with-history");
    let history_log = paths.state_dir.join("history.jsonl");
    let session_log = paths.session_dir.join("2026-07-03T02-45-00Z.session.log");
    fs::write(&history_log, "history").unwrap();
    fs::write(&session_log, "session").unwrap();
    let mut state = test_state();
    state.log_paths = paths.clone();
    refresh_log_summaries(&mut state).unwrap();
    state.selected_log = state
        .logs
        .iter()
        .position(|entry| entry.path == session_log)
        .expect("session log row");

    fs::remove_file(&session_log).unwrap();
    state.focus = Pane::History;
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.action_menu.is_some());
    assert_eq!(state.logs.len(), 1);
    assert_eq!(state.logs[0].path, history_log);
    let rows = action_menu_rows(&state, ActionMenuKind::Log);
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    assert!(rows.is_empty());
    let buffer = rendered_buffer(&mut state, 100, 24);
    assert!(buffer.contains("Selected log no longer exists"));
    assert!(buffer.contains("2026-07-03T02-45-00Z.session.log"));
}

#[test]
fn deleted_selected_log_feedback_persists_during_log_action_navigation() {
    let paths = test_logging_paths("log-actions-deleted-feedback-persistence");
    let history_log = paths.state_dir.join("history.jsonl");
    let session_log = paths.session_dir.join("2026-07-03T02-50-00Z.session.log");
    fs::write(&history_log, "history").unwrap();
    fs::write(&session_log, "session").unwrap();
    let mut state = test_state();
    state.log_paths = paths.clone();
    refresh_log_summaries(&mut state).unwrap();
    state.selected_log = state
        .logs
        .iter()
        .position(|entry| entry.path == session_log)
        .expect("session log row");

    fs::remove_file(&session_log).unwrap();
    state.focus = Pane::History;
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Home);
    handle_key_code(&mut state, KeyCode::End);

    let rows = action_menu_rows(&state, ActionMenuKind::Log);
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    assert!(rows.is_empty());
    let buffer = rendered_buffer(&mut state, 100, 24);
    assert!(buffer.contains("Selected log no longer exists"));
    assert!(buffer.contains("2026-07-03T02-50-00Z.session.log"));
    assert!(!buffer.contains("Open log in Response"));
    assert!(!buffer.contains("Open logs folder"));
    assert!(buffer.contains("Esc closes."));
    assert!(!buffer.contains("Enter selects."));
    assert_eq!(footer_text(&state, 80), "Esc Close");
    handle_key_code(&mut state, KeyCode::Char('q'));
    assert!(state.action_menu.is_some());
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.action_menu.is_none());
}

#[test]
fn missing_selected_log_refreshes_list_after_open_failure() {
    let paths = test_logging_paths("log-open-deleted-after-menu");
    let session_log = paths.session_dir.join("2026-07-03T03-00-00Z.session.log");
    fs::write(&session_log, "session").unwrap();
    let mut state = test_state();
    state.log_paths = paths.clone();
    refresh_log_summaries(&mut state).unwrap();
    assert_eq!(state.logs.len(), 1);

    state.focus = Pane::History;
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.action_menu.is_some());
    fs::remove_file(&session_log).unwrap();
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.status, "Failed to open log.");
    assert!(state.response.contains("Failed to read selected log."));
    assert!(state.response.contains("2026-07-03T03-00-00Z.session.log"));
    assert!(state.response.contains("Reason: "));
    assert!(state.response.contains("Logs list refreshed."));
    assert!(state.logs.is_empty());
    assert_eq!(state.logs_error, None);
    assert!(state.viewed_log.is_none());
}

#[test]
fn missing_selected_log_after_menu_open_does_not_open_history() {
    let paths = test_logging_paths("log-open-deleted-session-with-history");
    let history_log = paths.state_dir.join("history.jsonl");
    let session_log = paths.session_dir.join("2026-07-03T03-15-00Z.session.log");
    fs::write(&history_log, "history-body").unwrap();
    fs::write(&session_log, "session-body").unwrap();
    let mut state = test_state();
    state.log_paths = paths.clone();
    refresh_log_summaries(&mut state).unwrap();
    state.selected_log = state
        .logs
        .iter()
        .position(|entry| entry.path == session_log)
        .expect("session log row");

    state.focus = Pane::History;
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.action_menu.is_some());
    fs::remove_file(&session_log).unwrap();
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.status, "Failed to open log.");
    assert!(state.response.contains("Failed to read selected log."));
    assert!(state.response.contains("2026-07-03T03-15-00Z.session.log"));
    assert!(state.response.contains("Logs list refreshed."));
    assert!(!state.response.contains("history-body"));
    assert!(state.logs.iter().all(|entry| entry.path != session_log));
    assert!(state.logs.iter().any(|entry| entry.path == history_log));
    assert!(state.viewed_log.is_none());
}

#[test]
fn logs_pane_shows_refresh_error_without_replacing_existing_list() {
    let mut state = test_state();
    state.logs_error = Some("Refresh failed: denied".to_owned());
    state.focus = Pane::History;

    let buffer = rendered_buffer(&mut state, 100, 20);

    assert!(buffer.contains("Refresh failed: denied"));
    assert!(buffer.contains("history: history.jsonl"));
}

#[test]
fn page_and_boundary_keys_apply_to_focused_list_pane() {
    let commands = (0..20)
        .map(|index| Preset::built_in(format!("cmd-{index:02}"), "AT", ["bulk"]))
        .collect::<Vec<_>>();
    let mut state = TuiState::new(
        commands,
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.focus = Pane::Commands;
    state.commands_visible_height = 4;

    handle_key_code(&mut state, KeyCode::PageDown);
    assert_eq!(state.selected_command, 4);
    handle_key_code(&mut state, KeyCode::End);
    assert_eq!(state.selected_command, 19);
    handle_key_code(&mut state, KeyCode::Home);
    assert_eq!(state.selected_command, 0);
}

#[test]
fn page_and_boundary_keys_select_only_items_in_grouped_executable_list() {
    let mut state = TuiState::new_with_all_usb(
        executable_items(
            vec![
                Preset::built_in("command-00", "AT", ["bulk"]),
                Preset::built_in("command-01", "AT+CSQ", ["bulk"]),
                Preset::new(
                    "command-02",
                    "AT+QCCID",
                    vec!["bulk".to_owned()],
                    PresetOrigin::file("External commands", "external.toml", None),
                ),
                Preset::new(
                    "command-03",
                    "AT+QCSQ",
                    vec!["bulk".to_owned()],
                    PresetOrigin::file("External commands", "external.toml", None),
                ),
            ],
            vec![
                Sequence::built_in(
                    "sequence-00",
                    "Inspect modem response.",
                    vec!["bulk".to_owned()],
                    Some(30),
                    Vec::new(),
                    vec![test_sequence_step("AT")],
                ),
                Sequence::new(
                    "sequence-01",
                    "Inspect vendor signal state.",
                    vec!["bulk".to_owned()],
                    SequenceOrigin::file("External Sequences", "external-sequences.toml", None),
                    Some(30),
                    Vec::new(),
                    vec![test_sequence_step("AT+QCSQ")],
                ),
            ],
        ),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.focus = Pane::Commands;
    state.commands_visible_height = 3;

    let visible = state.visible_commands();
    let rows = command_list_rows(&visible);
    assert!(
        rows.iter()
            .any(|row| matches!(row, CommandListRow::KindHeader(_)))
    );
    assert!(
        rows.iter()
            .any(|row| matches!(row, CommandListRow::SourceHeader(_)))
    );
    assert!(
        rows.iter()
            .any(|row| matches!(row, CommandListRow::BlankSeparator))
    );
    drop(visible);

    for (key, expected) in [
        (KeyCode::PageDown, 3),
        (KeyCode::PageUp, 0),
        (KeyCode::End, 5),
        (KeyCode::PageUp, 2),
        (KeyCode::Home, 0),
    ] {
        handle_key_code(&mut state, key);
        assert_eq!(state.selected_command, expected);

        let visible = state.visible_commands();
        let rows = command_list_rows(&visible);
        let selected_row = selected_command_row_index(&rows, state.selected_command);
        assert!(matches!(
            rows[selected_row],
            CommandListRow::Command { command_index, .. }
                if command_index == state.selected_command
        ));
    }
}

#[test]
fn selected_device_can_be_reselected_after_command_completion() {
    let mut state = TuiState::new(
        test_commands(),
        vec![
            test_usb_device(1, 3, "Onyx A"),
            test_usb_device(2, 4, "Onyx B"),
        ],
        Vec::new(),
        TuiTheme::dark(),
    );
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    state.focus = Pane::Devices;
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(
        executor.devices,
        vec![
            Some(TuiDeviceSelection {
                vendor_id: 0x2c7c,
                product_id: 0x0125,
                bus: 1,
                address: 3,
            }),
            Some(TuiDeviceSelection {
                vendor_id: 0x2c7c,
                product_id: 0x0125,
                bus: 2,
                address: 4,
            }),
        ]
    );
    assert!(state.response.contains("OK"));
}

#[test]
fn history_focus_selects_and_opens_masked_log_content() {
    let paths = test_logging_paths("session-log");
    let session_log = paths.session_dir.join("2026-06-18T00-00-00Z.session.log");
    fs::write(
        &session_log,
        r#"{
  "command": "AT+QCCID",
  "response": "+QCCID: 89423100************"
}"#,
    )
    .unwrap();
    let mut state = test_state();
    state.log_paths = paths;
    refresh_log_summaries(&mut state).unwrap();
    state.focus = Pane::History;

    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.action_menu.is_some());
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.focus, Pane::Response);
    assert!(!state.response.contains("Masked session log"));
    assert!(state.response.contains("+QCCID: 89423100************"));
    assert!(!state.response.contains("8942310020003626445F"));
    assert!(state.viewed_log.is_some());
    assert_eq!(state.status, "Opened masked log.");
    let log_line_count = response_lines(&state).len();
    assert!(log_line_count >= 3);
    assert_eq!(
        response_range_title(
            log_line_count,
            0,
            log_line_count.saturating_sub(1),
            true,
            true,
        ),
        format!("Response 1-{log_line_count}/{log_line_count} all")
    );

    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("Response 1-"));
    assert!(buffer.contains("1  {"));
    assert!(!buffer.contains("Response scroll: line"));
}

#[test]
fn history_selection_moves_between_logs() {
    let mut state = test_state();
    state.logs = vec![
        test_log_entry(LogListingKind::History, "/tmp/history.jsonl"),
        test_log_entry(LogListingKind::Session, "/tmp/first.session.log"),
    ];
    state.focus = Pane::History;

    handle_key_code(&mut state, KeyCode::Down);
    assert_eq!(state.selected_log, 1);

    handle_key_code(&mut state, KeyCode::Up);
    assert_eq!(state.selected_log, 0);
}

#[test]
fn renders_running_command_context_before_transport_executes() {
    let mut state = test_state();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    handle_key_code(&mut state, KeyCode::Enter);
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("Waiting for modem response"));
    assert!(buffer.contains("AT command: AT"));
    assert!(!buffer.contains("Risk:"));
}

#[test]
fn clear_response_removes_previous_rendered_content() {
    let mut state = test_state();
    state.log_paths = test_logging_paths("clear-response-actions");
    let command = ExecutableItem::Preset(Preset::built_in("signal-quality", "AT+CSQ", ["signal"]));
    state.active_command = Some(
        CommandStatus::new(
            CommandRunState::Completed,
            &command,
            DEFAULT_COMMAND_TIMEOUT_SECS,
            StatusSummary::Completed {
                status: "OK".to_owned(),
                duration_ms: 7,
            },
        )
        .with_finished_at("2026-07-02T11:41:12Z".to_owned()),
    );

    state.response = ResponseState::masked("leftover-fragment");
    assert!(rendered_buffer(&mut state, 100, 32).contains("leftover-fragment"));

    run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::ClearResponse,
    );
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(!buffer.contains("leftover-fragment"));
    assert!(state.response.is_empty());
    let cleared_at = state.response_cleared_at.as_deref().expect("cleared at");
    assert_eq!(cleared_at.len(), "2026-07-02T11:41:12Z".len());
    assert!(cleared_at.contains('T'));
    assert!(cleared_at.ends_with('Z'));
    assert!(buffer.contains("Response body cleared."));
    assert!(buffer.contains("Cleared: "));
    assert!(!buffer.contains("No response."));
    assert!(buffer.contains("Status: completed"));
    assert!(buffer.contains("Command:"));
    assert!(buffer.contains("Result: OK 7ms"));
    assert!(buffer.contains("Completed: 2026-07-02T11:41:12Z"));
    assert!(!buffer.contains("Completed at:"));
    assert!(buffer.contains("2026-07-02T11:41:12Z"));
    assert!(!buffer.contains("Risk:"));
    let status_buffer = rendered_status_buffer(&mut state, 80, 12);
    assert_contains_in_order(
        &status_buffer,
        &[
            "Status: completed",
            "Completed: 2026-07-02T11:41:12Z",
            "Command: signal-quality",
            "AT command: AT+CSQ",
            "Result: OK 7ms",
        ],
    );
    assert!(status_buffer.contains("Command: signal-quality"));
    assert!(status_buffer.contains("AT command: AT+CSQ"));
    assert_eq!(state.status, "Response body cleared.");
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Response body cleared.")
    );

    open_response_action_menu(&mut state);
    let rows = action_menu_rows(&state, ActionMenuKind::Response);
    assert!(rows.is_empty());
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::CopyResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::ExportResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::ClearResponse)
    );
}

#[test]
fn completed_execution_status_shows_completed_event_timestamp() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();
    state.output_masking_enabled = false;

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let completed_at = state
        .active_command
        .as_ref()
        .and_then(|active| active.finished_at.as_deref())
        .expect("completed timestamp");
    assert_eq!(completed_at.len(), "2026-07-02T11:41:12Z".len());
    assert!(completed_at.contains('T'));
    assert!(completed_at.ends_with('Z'));
    let completed_at = completed_at.to_owned();
    let completed_line = format!("Completed: {completed_at}");

    let buffer = rendered_status_buffer(&mut state, 80, 12);
    assert!(buffer.contains("Status: completed"));
    assert!(buffer.contains("Result: OK 7ms"));
    assert!(buffer.contains(&completed_line));
    assert!(!buffer.contains("Completed at:"));
    assert!(!buffer.contains("Result: OK 7ms Completed:"));
    assert!(buffer.contains(&completed_at));
    assert!(buffer.contains("AT command: AT"));
    assert!(!buffer.contains("Risk:"));
    assert!(buffer.contains("Output masking: off"));
    assert_contains_in_order(
        &buffer,
        &[
            "Status: completed",
            &completed_line,
            "Command: modem-response",
            "AT command: AT",
            "Result: OK 7ms",
            "Output masking: off",
        ],
    );
}

#[test]
fn cancelled_execution_status_shows_cancelled_event_timestamp() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in("signal-quality", "AT+CSQ", ["signal"]));
    state.active_command = Some(
        CommandStatus::new(
            CommandRunState::Cancelled,
            &command,
            DEFAULT_COMMAND_TIMEOUT_SECS,
            StatusSummary::None,
        )
        .with_finished_at("2026-07-02T11:42:13Z".to_owned()),
    );

    let buffer = rendered_status_buffer(&mut state, 80, 12);

    assert!(buffer.contains("Status: cancelled"));
    assert!(buffer.contains("Cancelled: 2026-07-02T11:42:13Z"));
    assert!(!buffer.contains("Cancelled at:"));
    assert_contains_in_order(
        &buffer,
        &[
            "Status: cancelled",
            "Cancelled: 2026-07-02T11:42:13Z",
            "Command: signal-quality",
            "AT command: AT+CSQ",
        ],
    );
}

#[test]
fn completed_status_stays_execution_context_after_selection_moves() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);
    state.selected_command = state
        .commands
        .iter()
        .position(|command| command.name() == "imsi")
        .expect("imsi command");

    let status_buffer = rendered_status_buffer(&mut state, 80, 12);

    assert!(status_buffer.contains("Status: completed"));
    assert!(status_buffer.contains("Command: modem-response"));
    assert!(status_buffer.contains("AT command: AT"));
    assert!(status_buffer.contains("Result: OK 7ms"));
    assert!(!status_buffer.contains("Selected Command:"));
    assert!(!status_buffer.contains("imsi"));
}

#[test]
fn renders_non_color_state_affordances() {
    let mut state = test_state();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());
    let compact = buffer.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(buffer.contains("> modem-response"));
    assert!(compact.contains("> modem-response AT"));
    assert!(!buffer.contains("Confirmation: not required"));
    assert!(buffer.contains("[confirm]"));
    assert!(!buffer.contains("! Confirmation required"));
}

#[test]
fn command_rows_prioritize_identity_and_show_only_confirmation_flow_tags() {
    let mut state = TuiState::new(
        vec![
            Preset::built_in("inspect", "AT", ["review"]),
            Preset::built_in("change", "AT+CMEE=2", ["review"]),
            Preset::built_in("unknown", "AT+VENDOR=1", ["review"]),
        ],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    let backend = TestBackend::new(160, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());
    let compact = buffer.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(compact.contains("> inspect AT"));
    assert!(!buffer.contains("Confirmation: not required"));
    assert!(compact.contains("change AT+CMEE=2 [confirm]"));
    assert!(compact.contains("unknown AT+VENDOR=1 [confirm]"));
    assert!(!buffer.contains("│ AT:"));
    assert!(!buffer.contains("│ Purpose:"));
    assert!(!buffer.contains("! Confirmation required"));
    for removed in [
        "[safe]",
        "[sensitive]",
        "[write]",
        "[persistent]",
        "[dangerous]",
        "[unknown]",
    ] {
        assert!(
            !buffer.contains(removed),
            "unexpected {removed} in {buffer}"
        );
    }
}

#[test]
fn executable_selection_keeps_row_height_and_confirmation_tag_emphasis_independent() {
    let mut state = TuiState::new(
        vec![
            Preset::built_in("inspect", "AT", ["review"]),
            Preset::built_in("unknown", "AT+VENDOR=1", ["review"]),
        ],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    state.selected_command = 1;
    let backend = TestBackend::new(160, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let inspect = buffer_line_position(buffer, "inspect").expect("inspect row");
    let unknown = buffer_line_position(buffer, "> unknown").expect("unknown row");
    assert_eq!(unknown.1, inspect.1 + 1);

    let name_cell = buffer.cell(unknown).expect("selected name cell");
    let confirmation_tag = buffer_line_position(buffer, "[confirm]").expect("confirmation tag");
    let confirmation_tag_cell = buffer
        .cell(confirmation_tag)
        .expect("confirmation tag cell");
    assert_eq!(
        name_cell.fg,
        TuiTheme::dark()
            .style(TuiStyleRole::Selected)
            .fg
            .expect("selected color")
    );
    assert_eq!(
        confirmation_tag_cell.fg,
        TuiTheme::dark()
            .style(TuiStyleRole::ConfirmationFlow)
            .fg
            .expect("confirmation-flow color")
    );
    assert_ne!(name_cell.fg, confirmation_tag_cell.fg);
}

#[test]
fn narrow_command_rows_preserve_complete_confirmation_tag_before_detail() {
    let item = ExecutableItem::Preset(Preset::built_in(
        "vendor-command-with-a-long-name",
        "AT+VENDORCOMMANDWITHLONGARGUMENT=1234567890",
        ["review"],
    ));

    assert_eq!(
        command_row_plain_text(false, &item, &[&item], 12),
        "Executable details cannot fit safely. Resize the terminal to continue."
    );
    assert_eq!(
        command_row_plain_text(false, &item, &[&item], 13),
        "  … [confirm]"
    );
}

#[test]
fn executable_name_alignment_uses_terminal_display_cells() {
    let ascii = ExecutableItem::Preset(Preset::built_in("modem", "AT", ["review"]));
    let wide = ExecutableItem::Preset(Preset::built_in("通信確認", "AT+CSQ", ["review"]));
    let commands = [&ascii, &wide];
    let name_width = executable_name_column_width(&commands, 120).unwrap();
    let confirmation_lane_width = executable_confirmation_lane_width(&commands);
    let ascii_row =
        executable_row_layout(&ascii, 120, name_width, confirmation_lane_width).unwrap();
    let wide_row = executable_row_layout(&wide, 120, name_width, confirmation_lane_width).unwrap();

    assert_eq!(name_width, display_cell_width("通信確認"));
    assert_eq!(display_cell_width(&ascii_row.name), name_width);
    assert_eq!(display_cell_width(&wide_row.name), name_width);
    assert_eq!(ascii_row.detail.as_deref(), Some("AT"));
    assert_eq!(wide_row.detail.as_deref(), Some("AT+CSQ"));
}

#[test]
fn long_executable_name_sets_one_uncapped_shared_name_column() {
    let long_name = "soracom-unified-endpoint-tcp-send-check";
    let long = ExecutableItem::Sequence(Sequence::built_in(
        long_name,
        "Send a payload to SORACOM Unified Endpoint.",
        vec!["data".to_owned()],
        Some(180),
        Vec::new(),
        vec![test_sequence_step("AT+CMEE=2")],
    ));
    let short = ExecutableItem::Sequence(Sequence::built_in(
        "sms-send-check",
        "Send a standard SMS and check receive state.",
        vec!["sms".to_owned()],
        Some(180),
        Vec::new(),
        vec![test_sequence_step("AT+CMEE=2")],
    ));
    let commands = [&long, &short];
    let confirmation_lane_width = executable_confirmation_lane_width(&commands);
    let name_width = executable_name_column_width(&commands, 80).unwrap();

    assert_eq!(name_width, display_cell_width(long_name));
    assert!(name_width > 28);
    let normal_long =
        executable_row_layout(&long, 80, name_width, confirmation_lane_width).unwrap();
    let normal_short =
        executable_row_layout(&short, 80, name_width, confirmation_lane_width).unwrap();
    assert_eq!(normal_long.name, long_name);
    assert_eq!(normal_short.name.trim_end(), "sms-send-check");
    assert_eq!(display_cell_width(&normal_long.name), name_width);
    assert_eq!(display_cell_width(&normal_short.name), name_width);
    assert!(normal_long.detail.is_some());
    assert!(normal_short.detail.is_some());

    let long_text = command_row_plain_text(false, &long, &commands, 80);
    let short_text = command_row_plain_text(false, &short, &commands, 80);
    assert_eq!(
        long_text.find("Send a payload"),
        short_text.find("Send a standard")
    );

    let constrained_name_width = executable_name_column_width(&commands, 45).unwrap();
    let constrained_long =
        executable_row_layout(&long, 45, constrained_name_width, confirmation_lane_width).unwrap();
    let constrained_short =
        executable_row_layout(&short, 45, constrained_name_width, confirmation_lane_width).unwrap();
    assert!(constrained_long.name.trim_end().ends_with('…'));
    assert!(constrained_long.detail.is_none());
    assert_eq!(constrained_short.name.trim_end(), "sms-send-check");
    assert!(constrained_short.detail.is_none());
    assert_eq!(
        display_cell_width(&constrained_long.name),
        constrained_name_width
    );
    assert_eq!(
        display_cell_width(&constrained_short.name),
        constrained_name_width
    );

    let constrained_long_text = command_row_plain_text(false, &long, &commands, 45);
    let constrained_short_text = command_row_plain_text(false, &short, &commands, 45);
    assert_eq!(
        display_cell_width(
            constrained_long_text
                .split_once("[confirm]")
                .expect("long confirmation tag")
                .0,
        ),
        display_cell_width(
            constrained_short_text
                .split_once("[confirm]")
                .expect("short confirmation tag")
                .0,
        )
    );
}

#[test]
fn commands_pane_keeps_long_and_short_sequence_previews_aligned() {
    let long_name = "soracom-unified-endpoint-tcp-send-check";
    let long = Sequence::built_in(
        long_name,
        "Send a payload to SORACOM Unified Endpoint.",
        vec!["data".to_owned()],
        Some(180),
        Vec::new(),
        vec![test_sequence_step("AT+CMEE=2")],
    );
    let short = Sequence::built_in(
        "sms-send-check",
        "Send a standard SMS and check receive state.",
        vec!["sms".to_owned()],
        Some(180),
        Vec::new(),
        vec![test_sequence_step("AT+CMEE=2")],
    );
    let state = TuiState::new_with_all_usb(
        executable_items(Vec::new(), vec![long, short]),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        TuiTheme::dark(),
    );

    let normal = rendered_commands_buffer(&state, 80, 6);
    let normal_rows = buffer_rows(&normal);
    let normal_long = normal_rows
        .iter()
        .find(|row| row.contains(long_name))
        .expect("complete long Sequence row");
    let normal_short = normal_rows
        .iter()
        .find(|row| row.contains("sms-send-check"))
        .expect("complete short Sequence row");
    assert!(normal_long.contains("Send a payload"));
    assert!(normal_short.contains("Send a standard"));
    assert_eq!(
        normal_long.find("Send a payload"),
        normal_short.find("Send a standard")
    );

    let constrained = rendered_commands_buffer(&state, 45, 6);
    let constrained_rows = buffer_rows(&constrained);
    let constrained_long = constrained_rows
        .iter()
        .find(|row| row.contains("soracom-unified"))
        .expect("constrained long Sequence row");
    let constrained_short = constrained_rows
        .iter()
        .find(|row| row.contains("sms-send-check"))
        .expect("constrained short Sequence row");
    assert!(constrained_long.contains('…'));
    assert!(!constrained_long.contains("Send a payload"));
    assert!(!constrained_short.contains("Send a standard"));
    assert_eq!(
        display_cell_width(
            constrained_long
                .split_once("[confirm]")
                .expect("long confirmation tag")
                .0,
        ),
        display_cell_width(
            constrained_short
                .split_once("[confirm]")
                .expect("short confirmation tag")
                .0,
        )
    );
}

#[test]
fn executable_rows_share_confirmation_lane_and_reclaim_it_when_unused() {
    let observation = ExecutableItem::Preset(Preset::built_in(
        "observation",
        "AT+THISISALONGOBSERVATIONCOMMAND=1234567890,ABCDEFGHIJKLMNOPQRSTUVWXYZ,1234567890",
        ["review"],
    ));
    let known = ExecutableItem::Preset(Preset::built_in("known-change", "AT+CMEE=2", ["review"]));
    let unknown = ExecutableItem::Preset(Preset::built_in(
        "unknown-change",
        "AT+VENDOR=1",
        ["review"],
    ));
    let mixed = [&observation, &known, &unknown];
    let mixed_name_width = executable_name_column_width(&mixed, 80).unwrap();
    let lane_width = executable_confirmation_lane_width(&mixed);
    assert_eq!(lane_width, display_cell_width(" [confirm]"));

    let known_text = command_row_plain_text(false, &known, &mixed, 80);
    let unknown_text = command_row_plain_text(false, &unknown, &mixed, 80);
    assert_eq!(known_text.find("[confirm]"), unknown_text.find("[confirm]"));

    let mixed_observation =
        executable_row_layout(&observation, 80, mixed_name_width, lane_width).unwrap();
    let only_observation = [&observation];
    let only_name_width = executable_name_column_width(&only_observation, 80).unwrap();
    let reclaimed = executable_row_layout(&observation, 80, only_name_width, 0).unwrap();
    assert!(
        reclaimed
            .detail
            .as_ref()
            .map_or(0, |value| display_cell_width(value))
            > mixed_observation
                .detail
                .as_ref()
                .map_or(0, |value| display_cell_width(value))
    );
}

#[test]
fn compact_status_does_not_duplicate_execution_review() {
    let mut state = test_state();
    let status = rendered_status_buffer(&mut state, 80, 12);

    assert!(status.contains("Selected Command:"));
    assert!(!status.contains("Run: confirmation required"));
    assert!(!status.contains("Risk: effects unknown"));
    assert!(!status.contains("Confirmation:"));
    assert!(!status.contains("Assessment:"));
    assert!(!status.contains("Effects:"));
    assert!(!status.contains("Consequences:"));
}

#[test]
fn selected_unknown_item_status_does_not_duplicate_execution_review() {
    let mut state = TuiState::new(
        vec![Preset::built_in("unknown", "AT+VENDOR=1", ["review"])],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    let status = rendered_status_buffer(&mut state, 80, 12);

    assert!(status.contains("Selected Command: unknown"));
    assert!(!status.contains("Run: confirmation required"));
    assert!(!status.contains("Risk: effects unknown"));
    assert!(!status.contains("Assessment:"));
    assert!(!status.contains("Effects:"));
    assert!(!status.contains("Data handling:"));
    assert!(!status.contains("Reversibility:"));
    assert!(!status.contains("Consequences:"));
}

#[test]
fn confirmation_behavior_separates_known_and_unknown_consequences() {
    for (name, command) in [("inspect", "AT"), ("sensitive-read", "AT+CIMI")] {
        let mut state = TuiState::new(
            vec![Preset::built_in(name, command, ["review"])],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            TuiTheme::dark(),
        );

        handle_key_code(&mut state, KeyCode::Enter);

        assert!(
            state.confirmation.is_none(),
            "unexpected confirmation for {command}"
        );
        assert!(
            state.pending_execution.is_some(),
            "missing execution for {command}"
        );
    }

    for (command, token, unknown) in [
        ("AT+CMEE=2", "run", false),
        ("AT+VENDOR=1", "unknown", true),
    ] {
        let mut state = TuiState::new(
            vec![Preset::built_in("confirm", command, ["review"])],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            TuiTheme::dark(),
        );

        handle_key_code(&mut state, KeyCode::Enter);

        assert!(
            state.pending_execution.is_none(),
            "unexpected execution for {command}"
        );
        assert!(
            state.confirmation.is_some(),
            "missing confirmation for {command}"
        );
        let buffer = rendered_buffer(&mut state, 100, 32);
        if unknown {
            assert!(buffer.contains("Confirm unknown command"));
            assert!(buffer.contains("Effects are not fully known."));
            assert!(!buffer.contains("! Confirm unknown command"));
            assert!(!buffer.contains("! Effects are not fully known"));
            assert!(buffer.contains("Known:"));
            assert!(buffer.contains("Possible consequences:"));
            assert!(buffer.contains("Not established:"));
            assert!(!buffer.contains("What will happen:"));
        } else {
            assert!(buffer.contains("Confirm command"));
            assert!(buffer.contains("What will happen:"));
            assert!(!buffer.contains("Effects are not fully known."));
            assert!(!buffer.contains("Not established:"));
        }
        assert!(!buffer.contains("Assessment:"));
        assert!(!buffer.contains("Confirmation: required"));
        assert!(buffer.contains(&format!("Type `{token}` to send")));
    }
}

#[test]
fn output_masking_off_requires_exact_acknowledgement_for_session() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();
    state.selected_command = 2;

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert!(state.response.contains("89811000*******"));
    assert!(!state.response.contains("898110001234567"));
    assert_eq!(
        state
            .response
            .output_masking_label(state.output_masking_enabled),
        Some("on")
    );

    run_control(&mut state, ControlAction::ToggleOutputMasking);
    for value in "abc".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.output_masking_enabled);
    assert!(state.response.contains("89811000*******"));
    assert!(!state.response.contains("898110001234567"));

    run_control(&mut state, ControlAction::ToggleOutputMasking);
    for value in OUTPUT_UNMASK_ACK.chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(!state.output_masking_enabled);
    assert_eq!(
        state
            .response
            .output_masking_label(state.output_masking_enabled),
        Some("off")
    );
    assert!(
        state
            .response
            .contains_visible(state.output_masking_enabled, "898110001234567")
    );
}

#[test]
fn response_without_masked_values_does_not_show_mask_state() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(
        state
            .response
            .output_masking_label(state.output_masking_enabled),
        None
    );
    assert!(state.response.contains("AT"));
    assert!(state.response.contains("OK"));

    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("Command:"));
    assert!(buffer.contains("modem-response"));
    assert!(buffer.contains("AT"));
    assert!(!buffer.contains("Risk:"));
    assert!(!buffer.contains("Mask: masked"));
    assert!(!buffer.contains("raw visible"));
    assert!(!buffer.contains("Output masking: on"));
}

#[test]
fn response_copy_uses_body_without_duplicate_echo_or_ui_chrome() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::CopyResponse,
    );
    let TuiAction::CopyToClipboard(text) = action else {
        panic!("expected clipboard action");
    };

    assert_eq!(text, "AT\nOK");
    assert!(!text.contains("Response"));
    assert!(!text.contains("Status"));
    assert!(state.response_action_confirmation.is_none());
}

#[test]
fn unmasked_response_copy_requires_explicit_confirmation_and_can_be_cancelled() {
    let mut state = unmasked_sensitive_state();

    let rows = response_action_rows(&state);
    assert!(rows.iter().any(|row| {
        row.action == ActionMenuAction::CopyResponse && row.label == "Copy unmasked response"
    }));
    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::CopyResponse,
    );
    assert_eq!(action, TuiAction::Continue);
    assert!(state.response_action_confirmation.is_some());

    let buffer = rendered_buffer(&mut state, 120, 32);
    assert!(buffer.contains("Copy unmasked response?"));
    assert!(buffer.contains("Warning:"));
    assert!(buffer.contains("Type `copy` to continue"));

    assert_eq!(
        handle_key_code(&mut state, KeyCode::Enter),
        TuiAction::Continue
    );
    assert!(state.response_action_confirmation.is_some());
    assert!(rendered_buffer(&mut state, 120, 32).contains("Type `copy` exactly."));

    for value in RESPONSE_COPY_ACK.chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    let action = handle_key_code(&mut state, KeyCode::Enter);
    let TuiAction::CopyToClipboard(contents) = action else {
        panic!("expected clipboard action after exact confirmation");
    };
    assert!(contents.contains("898110001234567"));
    assert!(state.response_action_confirmation.is_none());

    let mut cancelled = unmasked_sensitive_state();
    run_action_menu(
        &mut cancelled,
        ActionMenuKind::Response,
        ActionMenuAction::CopyResponse,
    );
    assert_eq!(
        handle_key_code(&mut cancelled, KeyCode::Char('q')),
        TuiAction::Continue
    );
    assert_eq!(
        cancelled
            .response_action_confirmation
            .as_ref()
            .map(|confirmation| confirmation.input.as_str()),
        Some("q")
    );
    handle_key_code(&mut cancelled, KeyCode::Esc);
    assert!(cancelled.response_action_confirmation.is_none());
    assert_eq!(cancelled.status, "Response copy cancelled.");
    assert_eq!(
        cancelled
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Response copy cancelled.")
    );
}

#[test]
fn response_copy_reports_clipboard_request_result() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::CopyResponse,
    );
    assert!(matches!(action, TuiAction::CopyToClipboard(_)));
    assert!(state.action_menu.is_none());

    finish_clipboard_copy(&mut state, Ok(()));
    assert_eq!(state.status, "Response copy requested.");
    assert_eq!(state.status_role, TuiStyleRole::Status);
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some(COPY_REQUEST_SENT_FEEDBACK)
    );
    let copied_response = copyable_response_text(&state).expect("copyable response");
    assert!(!copied_response.contains(COPY_REQUEST_SENT_FEEDBACK));
    let buffer = rendered_buffer(&mut state, 100, 24);
    assert!(buffer.contains(COPY_REQUEST_SENT_FEEDBACK));
    assert!(buffer.contains("AT"));
    let copy_row = action_menu_rows(&state, ActionMenuKind::Response)
        .into_iter()
        .find(|row| row.action == ActionMenuAction::CopyResponse)
        .expect("copy row");
    assert_eq!(copy_row.label, "Copy response");

    finish_clipboard_copy(&mut state, Err(AtctlError::Transport("blocked".to_owned())));
    assert_eq!(state.status, "Response copy failed.");
    assert_eq!(state.status_role, TuiStyleRole::Error);
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Copy request failed: transport error: blocked")
    );
    let copy_row = action_menu_rows(&state, ActionMenuKind::Response)
        .into_iter()
        .find(|row| row.action == ActionMenuAction::CopyResponse)
        .expect("copy row");
    assert_eq!(copy_row.label, "Copy response");
}

#[test]
fn log_actions_offer_open_and_reveal_for_selected_log() {
    let mut state = test_state();
    let log_path = PathBuf::from("/tmp/atctl/logs/2026-06-18T04-02-06Z.session.log");
    state.logs = vec![test_log_entry(LogListingKind::Session, log_path.clone())];
    state.selected_log = 0;

    let rows = action_menu_rows(&state, ActionMenuKind::Log);

    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    let context = action_menu_context_lines(&state, ActionMenuKind::Log);
    assert!(context[0].starts_with("Log: session: "));
    assert!(context[0].contains("2026-06-18T04-02-06Z.session.log"));
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::RevealInFinder)
    );
    assert!(!rows.iter().any(|row| row.label == "Copy log path"));
    assert!(!rows.iter().any(|row| row.label == "Copy log folder"));
}

#[test]
fn log_actions_without_selected_log_are_empty() {
    let mut state = test_state();
    state.logs.clear();

    let rows = action_menu_rows(&state, ActionMenuKind::Log);

    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    assert!(rows.is_empty());
    assert!(action_menu_context_lines(&state, ActionMenuKind::Log).is_empty());
}

#[test]
fn logs_enter_without_saved_logs_does_not_open_empty_menu() {
    let mut state = test_state();
    state.log_paths = test_logging_paths("no-saved-logs");
    state.logs.clear();
    state.focus = Pane::History;

    let action = handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(action, TuiAction::Continue);
    assert!(state.action_menu.is_none());
    assert_eq!(state.status, "No logs are available.");
}

#[test]
fn selected_history_and_session_logs_reveal_exact_file_directly_from_logs() {
    let paths = test_logging_paths("direct-log-reveal");
    let history_log = paths.state_dir.join("history.jsonl");
    let session_log = paths.session_dir.join("2026-07-03T04-00-00Z.session.log");
    fs::write(&history_log, "history").unwrap();
    fs::write(&session_log, "session").unwrap();

    for (kind, path) in [
        (LogListingKind::History, history_log),
        (LogListingKind::Session, session_log),
    ] {
        let mut state = test_state();
        state.log_paths = paths.clone();
        state.logs = vec![test_log_entry(kind, path.clone())];
        state.selected_log = 0;
        state.focus = Pane::History;

        handle_key_code(&mut state, KeyCode::Enter);
        handle_key_code(&mut state, KeyCode::Down);
        let action = handle_key_code(&mut state, KeyCode::Enter);

        assert_eq!(
            action,
            TuiAction::RevealPath {
                path,
                origin: ActionMenuKind::Log,
            }
        );
        assert!(state.viewed_log.is_none());
        assert_eq!(state.focus, Pane::History);
        assert!(state.action_menu.is_none());
    }
}

#[test]
fn viewed_history_and_session_logs_reveal_the_exact_opened_file() {
    let paths = test_logging_paths("reveal-exact-log");
    let history_log = paths.state_dir.join("history.jsonl");
    let session_log = paths.session_dir.join("2026-07-03T04-00-00Z.session.log");
    fs::write(&history_log, "history").unwrap();
    fs::write(&session_log, "session").unwrap();

    for (kind, path) in [
        (LogListingKind::History, history_log),
        (LogListingKind::Session, session_log),
    ] {
        let mut state = test_state();
        state.response = ResponseState::masked("masked log body");
        state.viewed_log = Some(ViewedLog {
            kind,
            path: path.clone(),
            label: compact_path_label(&path),
        });

        let action = run_action_menu(
            &mut state,
            ActionMenuKind::Response,
            ActionMenuAction::RevealInFinder,
        );

        assert_eq!(
            action,
            TuiAction::RevealPath {
                path,
                origin: ActionMenuKind::Response,
            }
        );
        assert!(state.action_menu.is_none());
    }
}

#[test]
fn export_response_uses_selected_folder_and_retains_exact_file_context() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in("signal-quality", "AT+CSQ", ["signal"]));
    state.active_command = Some(
        CommandStatus::new(
            CommandRunState::Completed,
            &command,
            DEFAULT_COMMAND_TIMEOUT_SECS,
            StatusSummary::Completed {
                status: "OK".to_owned(),
                duration_ms: 7,
            },
        )
        .with_finished_at("2026-07-12T09:00:00Z".to_owned()),
    );
    state.response = ResponseState::masked("AT+CSQ\n+CSQ: 20,99\nOK");

    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::ExportResponse,
    );
    let TuiAction::ChooseResponseExportDirectory(request) = action else {
        panic!("expected folder chooser request");
    };
    assert_eq!(request.response_label, "Command: signal-quality");
    assert!(request.masked);
    assert!(request.file_name.starts_with("command-signal-quality-"));
    let directory = unique_temp_dir("exported-response-context");
    finish_response_export(&mut state, request, Ok(Some(directory.clone())));
    assert!(state.response_action_confirmation.is_none());

    let exported = state.exported_response.as_ref().expect("exported response");
    assert!(exported.path.is_file());
    assert_eq!(exported.response_label, "Command: signal-quality");
    assert_eq!(
        exported.finished_at.as_deref(),
        Some("2026-07-12T09:00:00Z")
    );
    assert_eq!(state.status, "Response export completed.");
    let export_feedback = state
        .response_action_feedback
        .as_ref()
        .expect("response export feedback");
    assert!(
        export_feedback
            .message
            .starts_with("Exported response: command-signal-quality-")
    );
    assert!(export_feedback.message.ends_with(".response.txt."));
    assert_eq!(
        fs::read_to_string(&exported.path).unwrap(),
        "AT+CSQ\n+CSQ: 20,99\nOK\n"
    );
    let expected_path = exported.path.clone();

    open_response_action_menu(&mut state);
    let context = action_menu_context_lines(&state, ActionMenuKind::Response);
    assert!(context.contains(&"Response: Command: signal-quality".to_owned()));
    assert!(context.contains(&"Completed: 2026-07-12T09:00:00Z".to_owned()));
    assert!(context.contains(&"Export format: UTF-8 text".to_owned()));
    assert!(context.contains(&"Export content: masked".to_owned()));
    assert!(
        context
            .iter()
            .any(|line| line.starts_with("File name: command-signal-quality-"))
    );
    assert!(context.contains(&"Last exported response: Command: signal-quality".to_owned()));
    assert!(context.contains(&"Last export content: masked".to_owned()));

    state.focus = Pane::Response;
    let buffer = rendered_buffer(&mut state, 80, 24);
    assert!(
        buffer.contains("Response: Command: signal-quality"),
        "{buffer}"
    );
    assert!(buffer.contains("Completed: 2026-07-12T09:00:00Z"));
    assert!(buffer.contains("Export content: masked"));
    assert!(buffer.contains("File name:"), "{buffer}");
    assert!(buffer.contains("command-signal-quality-"), "{buffer}");
    assert!(buffer.contains("Reveal in Finder"));
    state.action_menu = None;

    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::RevealInFinder,
    );
    assert_eq!(
        action,
        TuiAction::RevealPath {
            path: expected_path,
            origin: ActionMenuKind::Response,
        }
    );
}

#[test]
fn unmasked_response_export_requires_exact_path_confirmation_before_file_creation() {
    let mut state = unmasked_sensitive_state();
    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::ExportResponse,
    );
    let TuiAction::ChooseResponseExportDirectory(request) = action else {
        panic!("expected folder chooser request");
    };
    assert!(!request.masked);
    let directory = unique_temp_dir("unmasked-response-export-confirmation");
    let expected_path = directory.join(&request.file_name);

    finish_response_export(&mut state, request, Ok(Some(directory)));

    assert!(!expected_path.exists());
    assert!(state.response_action_confirmation.is_some());
    let Some(ResponseActionConfirmationState {
        action: ResponseActionConfirmation::Export { path, .. },
        ..
    }) = state.response_action_confirmation.as_ref()
    else {
        panic!("expected unmasked export confirmation");
    };
    assert_eq!(path, &expected_path);
    let buffer = rendered_buffer(&mut state, 140, 32);
    assert!(buffer.contains("Export unmasked response?"));
    assert!(buffer.contains("Warning:"));
    assert!(buffer.contains("File:"));
    assert!(buffer.contains(expected_path.file_name().unwrap().to_str().unwrap()));
    assert!(buffer.contains("Type `export` to continue"));

    handle_key_code(&mut state, KeyCode::Enter);
    assert!(!expected_path.exists());
    assert!(state.response_action_confirmation.is_some());
    assert!(rendered_buffer(&mut state, 140, 32).contains("Type `export` exactly."));

    for value in RESPONSE_EXPORT_ACK.chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Enter),
        TuiAction::Continue
    );
    assert!(state.response_action_confirmation.is_none());
    assert!(expected_path.is_file());
    assert!(
        fs::read_to_string(&expected_path)
            .unwrap()
            .contains("898110001234567")
    );

    let mut cancelled = unmasked_sensitive_state();
    let action = run_action_menu(
        &mut cancelled,
        ActionMenuKind::Response,
        ActionMenuAction::ExportResponse,
    );
    let TuiAction::ChooseResponseExportDirectory(request) = action else {
        panic!("expected folder chooser request");
    };
    let directory = unique_temp_dir("unmasked-response-export-cancel");
    let cancelled_path = directory.join(&request.file_name);
    finish_response_export(&mut cancelled, request, Ok(Some(directory)));
    handle_key_code(&mut cancelled, KeyCode::Esc);
    assert!(!cancelled_path.exists());
    assert!(cancelled.response_action_confirmation.is_none());
    assert_eq!(cancelled.status, "Response export cancelled.");
    assert_eq!(
        cancelled
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Response export cancelled.")
    );
}

#[test]
fn response_export_context_identifies_initial_notice_sequence_and_candidate_action() {
    let mut state = test_state();
    assert_eq!(response_export_target_label(&state), "Initial notice");

    let sequence = crate::sequences::builtin::builtins()
        .into_iter()
        .next()
        .expect("built-in sequence");
    let item = ExecutableItem::Sequence(sequence.clone());
    state.active_command = Some(CommandStatus::new(
        CommandRunState::Completed,
        &item,
        DEFAULT_COMMAND_TIMEOUT_SECS,
        StatusSummary::Completed {
            status: "OK".to_owned(),
            duration_ms: 4,
        },
    ));

    assert_eq!(
        response_export_target_label(&state),
        format!("Sequence: {}", sequence.name)
    );

    let candidate_action = state.active_command.as_mut().unwrap();
    candidate_action.kind = ExecutableKind::CandidateAction;
    candidate_action.name = "Load candidates".to_owned();
    assert_eq!(
        response_export_target_label(&state),
        "Candidate action: Load candidates"
    );
}

#[test]
fn response_export_cancellation_and_folder_failure_keep_no_association() {
    let mut state = test_state();
    state.response = ResponseState::masked("AT\nOK");
    let request = response_export_request(&state).unwrap();

    finish_response_export(&mut state, request.clone(), Ok(None));
    assert!(state.exported_response.is_none());
    assert_eq!(state.status, "Response export cancelled.");
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Response export cancelled.")
    );

    finish_response_export(
        &mut state,
        request,
        Ok(Some(unique_temp_dir("missing-folder").join("missing"))),
    );
    assert!(state.exported_response.is_none());
    assert_eq!(state.status, "Response export failed.");
    let feedback = state
        .response_action_feedback
        .as_ref()
        .expect("response export failure feedback");
    assert!(
        feedback
            .message
            .contains("Destination folder does not exist")
    );
    assert!(
        feedback
            .message
            .starts_with("Choose an existing folder and export again.")
    );
    assert_eq!(state.status_role, TuiStyleRole::Error);
    let buffer = rendered_buffer(&mut state, 80, 24);
    assert!(buffer.contains("Choose an existing"), "{buffer}");
    assert!(buffer.contains("folder and export again."), "{buffer}");
}

#[test]
fn repeated_export_creates_new_files_without_deleting_prior_export() {
    let mut state = test_state();
    state.response = ResponseState::masked("AT\nOK");
    let directory = unique_temp_dir("repeated-response-export");

    let first_request = response_export_request(&state).unwrap();
    finish_response_export(&mut state, first_request, Ok(Some(directory.clone())));
    let first = state
        .exported_response
        .as_ref()
        .expect("first exported response")
        .path
        .clone();
    std::thread::sleep(Duration::from_millis(1));
    let second_request = response_export_request(&state).unwrap();
    finish_response_export(&mut state, second_request, Ok(Some(directory)));
    let second = state
        .exported_response
        .as_ref()
        .expect("second exported response")
        .path
        .clone();

    assert_ne!(first, second);
    assert!(first.is_file());
    assert!(second.is_file());
}

#[test]
fn replacing_or_clearing_response_removes_export_association_only() {
    let mut state = test_state();
    state.response = ResponseState::masked("AT\nOK");
    let directory = unique_temp_dir("response-export-replacement");
    let first_request = response_export_request(&state).unwrap();
    finish_response_export(&mut state, first_request, Ok(Some(directory.clone())));
    let first = state.exported_response.as_ref().unwrap().path.clone();
    assert!(state.response_action_feedback.is_some());

    set_response(&mut state, ResponseState::masked("ATI\nOK"));
    assert!(state.exported_response.is_none());
    assert!(state.response_action_feedback.is_none());
    assert!(first.is_file());

    let second_request = response_export_request(&state).unwrap();
    finish_response_export(&mut state, second_request, Ok(Some(directory)));
    let second = state.exported_response.as_ref().unwrap().path.clone();
    clear_response(&mut state);
    assert!(state.exported_response.is_none());
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Response body cleared.")
    );
    assert!(second.is_file());
}

#[test]
fn deleted_export_disables_reveal_without_disabling_another_export() {
    let mut state = test_state();
    state.response = ResponseState::masked("AT\nOK");
    let directory = unique_temp_dir("deleted-response-export");
    let request = response_export_request(&state).unwrap();
    finish_response_export(&mut state, request, Ok(Some(directory)));
    let path = state.exported_response.as_ref().unwrap().path.clone();
    fs::remove_file(path).unwrap();

    open_response_action_menu(&mut state);
    let rows = action_menu_rows(&state, ActionMenuKind::Response);
    let reveal = rows
        .iter()
        .find(|row| row.action == ActionMenuAction::RevealInFinder)
        .expect("exported response reveal row");
    assert!(!reveal.enabled);
    assert_eq!(
        reveal.unavailable_message,
        "Exported response no longer exists."
    );
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::ExportResponse && row.enabled)
    );
}

#[test]
fn response_export_refuses_an_existing_file_without_overwriting_it() {
    let mut state = test_state();
    state.response = ResponseState::masked("AT\nOK");
    let request = response_export_request(&state).unwrap();
    let directory = unique_temp_dir("existing-response-export");
    let path = directory.join(&request.file_name);
    fs::write(&path, "existing").unwrap();

    finish_response_export(&mut state, request, Ok(Some(directory)));
    assert!(state.exported_response.is_none());
    assert_eq!(state.status, "Response export failed.");
    let feedback = state
        .response_action_feedback
        .as_ref()
        .expect("response export failure feedback");
    assert!(feedback.message.contains("File already exists"));
    assert!(
        feedback
            .message
            .starts_with("Choose another folder and export again.")
    );
    let buffer = rendered_buffer(&mut state, 80, 24);
    assert!(buffer.contains("Choose another folder"), "{buffer}");
    assert_eq!(fs::read_to_string(path).unwrap(), "existing");
}

#[test]
fn deleted_viewed_log_disables_reveal_but_keeps_copy_and_close() {
    let missing_log = unique_temp_dir("deleted-viewed-log").join("missing.session.log");
    let mut state = test_state();
    state.response = ResponseState::masked("cached masked log body");
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: missing_log,
        label: "missing.session.log".to_owned(),
    });

    let rows = action_menu_rows(&state, ActionMenuKind::Response);
    let reveal = rows
        .iter()
        .find(|row| row.action == ActionMenuAction::RevealInFinder)
        .expect("reveal row");

    assert!(!reveal.enabled);
    assert_eq!(reveal.unavailable_message, "Saved log no longer exists.");
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::CopyDisplayedLog)
    );
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::CloseLogView)
    );

    let action = run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::RevealInFinder,
    );
    assert_eq!(action, TuiAction::Continue);
    assert!(state.action_menu.is_some());
    assert!(
        state
            .action_menu
            .as_ref()
            .and_then(|menu| menu.feedback.as_ref())
            .is_some_and(|feedback| feedback.message == "Saved log no longer exists.")
    );
}

#[test]
fn selected_log_deleted_after_menu_open_cannot_reveal_or_retarget() {
    let paths = test_logging_paths("deleted-log-before-reveal");
    let selected = paths.session_dir.join("2026-07-03T05-00-00Z.session.log");
    let history = paths.state_dir.join("history.jsonl");
    fs::write(&selected, "session").unwrap();
    fs::write(&history, "history").unwrap();
    let mut state = test_state();
    state.log_paths = paths;
    refresh_log_summaries(&mut state).unwrap();
    state.selected_log = state
        .logs
        .iter()
        .position(|entry| entry.path == selected)
        .expect("selected session");
    state.focus = Pane::History;

    handle_key_code(&mut state, KeyCode::Enter);
    fs::remove_file(&selected).unwrap();
    handle_key_code(&mut state, KeyCode::Down);
    let action = handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(action, TuiAction::Continue);
    assert!(state.action_menu.is_some());
    assert!(
        state
            .action_menu
            .as_ref()
            .and_then(|menu| menu.feedback.as_ref())
            .is_some_and(|feedback| feedback.message == "Saved log no longer exists.")
    );
    assert!(state.viewed_log.is_none());
}

#[test]
fn reveal_feedback_reports_request_without_claiming_finder_state() {
    let mut state = test_state();
    let path = PathBuf::from("/tmp/history.jsonl");
    state.response = ResponseState::masked("unchanged response body");

    finish_path_reveal(&mut state, &path, ActionMenuKind::Response, Ok(()));
    assert_eq!(state.status, "Reveal request sent.");
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Reveal in Finder request sent: history.jsonl.")
    );

    finish_path_reveal(
        &mut state,
        &path,
        ActionMenuKind::Response,
        Err(AtctlError::Transport("blocked".to_owned())),
    );
    assert_eq!(state.status, "Reveal request failed.");
    assert_eq!(
        state
            .response_action_feedback
            .as_ref()
            .map(|feedback| feedback.message.as_str()),
        Some("Reveal in Finder request failed: transport error: blocked")
    );

    let buffer = rendered_buffer(&mut state, 100, 24);
    assert!(buffer.contains("Reveal in Finder request failed"));
    assert!(buffer.contains("unchanged response body"));
}

#[test]
fn log_view_close_clears_opened_log_without_status_clear_action() {
    let mut state = test_state();
    state.focus = Pane::Response;
    state.response = ResponseState::masked("persisted log body");
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: PathBuf::from("/tmp/test.session.log"),
        label: "test.session.log".to_owned(),
    });
    set_response_action_result(
        &mut state,
        TuiStyleRole::Status,
        "Response copy requested.",
        COPY_REQUEST_SENT_FEEDBACK,
    );

    run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::CloseLogView,
    );

    assert!(state.action_menu.is_none());
    assert!(state.response.is_empty());
    assert!(state.viewed_log.is_none());
    assert!(state.response_action_feedback.is_none());
    assert_eq!(state.status, "Log view closed.");
}

#[test]
fn response_copy_prefixes_command_when_modem_does_not_echo() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in("signal-quality", "AT+CSQ", ["signal"]));
    state.active_command = Some(CommandStatus::new(
        CommandRunState::Completed,
        &command,
        DEFAULT_COMMAND_TIMEOUT_SECS,
        StatusSummary::Completed {
            status: "OK".to_owned(),
            duration_ms: 1,
        },
    ));
    state.response = ResponseState::masked("+CSQ: 20,99\nOK\n");

    assert_eq!(
        copyable_response_text(&state),
        Some("AT+CSQ\n+CSQ: 20,99\nOK".to_owned())
    );
}

#[test]
fn response_copy_uses_unmasked_text_only_while_output_masking_is_off() {
    let mut state = unmasked_sensitive_state();

    assert_eq!(
        copyable_response_text(&state),
        Some("AT+CIMI\n898110001234567\nOK".to_owned())
    );

    state.output_masking_enabled = true;

    assert_eq!(
        copyable_response_text(&state),
        Some("AT+CIMI\n89811000*******\nOK".to_owned())
    );
}

#[test]
fn response_copy_for_log_view_omits_line_number_ui() {
    let mut state = test_state();
    state.status = "Opened masked log.".to_owned();
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: PathBuf::from("/tmp/test.session.log"),
        label: "test.session.log".to_owned(),
    });
    state.response = ResponseState::masked("{\n  \"response\": \"OK\"\n}");

    assert_eq!(
        copyable_response_text(&state),
        Some("{\n  \"response\": \"OK\"\n}".to_owned())
    );
}

#[test]
fn osc52_sequence_base64_encodes_clipboard_text() {
    assert_eq!(osc52_clipboard_sequence("AT\nOK"), "\x1b]52;c;QVQKT0s=\x07");
}

#[test]
fn output_masking_change_can_be_cancelled_and_enabled_again() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();
    state.selected_command = 2;

    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);
    run_control(&mut state, ControlAction::ToggleOutputMasking);
    handle_key_code(&mut state, KeyCode::Esc);

    assert!(state.output_masking_ack_input.is_none());
    assert!(state.output_masking_enabled);

    run_control(&mut state, ControlAction::ToggleOutputMasking);
    handle_key_code(&mut state, KeyCode::Char('q'));

    assert_eq!(state.output_masking_ack_input.as_deref(), Some("q"));
    assert!(state.output_masking_enabled);
    handle_key_code(&mut state, KeyCode::Esc);
    assert!(state.output_masking_ack_input.is_none());

    run_control(&mut state, ControlAction::ToggleOutputMasking);
    for value in OUTPUT_UNMASK_ACK.chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(!state.output_masking_enabled);

    run_control(&mut state, ControlAction::ToggleOutputMasking);
    assert!(state.output_masking_enabled);
    assert!(state.response.contains("89811000*******"));
}

#[test]
fn output_masking_off_persists_until_enabled_or_tui_exit() {
    let mut state = unmasked_sensitive_state();

    run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::ClearResponse,
    );
    assert!(!state.output_masking_enabled);
    assert!(!state.response.has_raw_text());
    assert!(state.response.is_empty());

    state = unmasked_sensitive_state();
    state.selected_command = 0;
    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(!state.output_masking_enabled);
    assert!(!state.response.has_raw_text());

    state = unmasked_sensitive_state();
    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Up);
    assert!(!state.output_masking_enabled);
    assert!(state.response.has_raw_text());

    state = unmasked_sensitive_state();
    state.focus = Pane::Categories;
    handle_key_code(&mut state, KeyCode::Down);
    assert!(!state.output_masking_enabled);
    assert!(state.response.has_raw_text());
}

#[test]
fn unmasked_response_pane_prioritizes_response_body_without_duplicate_context() {
    let mut state = unmasked_sensitive_state();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("Output masking: off"));
    assert!(buffer.contains("898110001234567"));
    assert!(!buffer.contains("Completed command"));
    assert!(!buffer.contains("Effects: reads sensitive"));
}

#[test]
fn status_does_not_render_copy_behavior_explanation_for_unmasked_response() {
    let mut state = unmasked_sensitive_state();

    let buffer = rendered_status_buffer(&mut state, 44, 12);

    assert!(buffer.contains("Output masking: off"));
    assert!(!buffer.contains("Copy:"));
    assert!(!buffer.contains("Copy response uses"));
    assert!(!buffer.contains("visible Response body"));
}

#[test]
fn status_does_not_render_copy_behavior_explanation_for_viewed_log() {
    let mut state = test_state();
    state.status = "Opened masked log.".to_owned();
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: PathBuf::from("/tmp/test.session.log"),
        label: "test.session.log".to_owned(),
    });
    state.response = ResponseState::masked("{\n  \"response\": \"OK\"\n}");

    let buffer = rendered_status_buffer(&mut state, 44, 12);

    assert!(buffer.contains("Status: viewing log"));
    assert!(!buffer.contains("State: viewing masked log"));
    assert!(buffer.contains("Raw values: not shown"));
    assert!(!buffer.contains("Copy:"));
    assert!(!buffer.contains("Copy response uses"));
    assert!(!buffer.contains("displayed masked log body"));
}

#[test]
fn response_state_removes_terminal_control_sequences_before_rendering() {
    let response = ResponseState::with_raw(
        "AT+QCCID\r\r\n+QCCID: 89423100************\r\n\x1b[31mOK\x1b[0m\r\n",
        "AT+QCCID\r\r\n+QCCID: 8942310020003626445F\r\n\x1b[31mOK\x1b[0m\r\n",
    );

    assert!(!response.visible_text(true).contains('\r'));
    assert!(!response.visible_text(true).contains('\x1b'));
    assert!(!response.visible_text(true).contains("[31m"));
    assert!(response.visible_text(true).contains("89423100************"));

    assert!(!response.visible_text(false).contains('\r'));
    assert!(!response.visible_text(false).contains('\x1b'));
    assert!(!response.visible_text(false).contains("[31m"));
    assert!(
        response
            .visible_text(false)
            .contains("+QCCID: 8942310020003626445F")
    );
    assert!(response.visible_text(false).contains("OK"));
}

#[test]
fn response_pane_renders_sanitized_raw_response_body() {
    let mut state = test_state();
    state.response = ResponseState::with_raw(
        "AT+QCCID\r\r\n+QCCID: 89423100************\r\nOK\r\n",
        "AT+QCCID\r\r\n+QCCID: 8942310020003626445F\r\nOK\r\n",
    );
    state.output_masking_enabled = false;

    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("Output masking: off"));
    assert!(buffer.contains("+QCCID: 8942310020003626445F"));
    assert!(buffer.contains("OK"));
    assert!(!state.response.visible_text(false).contains('\r'));
}

#[test]
fn semantic_style_roles_are_declared() {
    for role in [
        TuiStyleRole::Background,
        TuiStyleRole::Text,
        TuiStyleRole::Emphasis,
        TuiStyleRole::ConfirmationFlow,
        TuiStyleRole::Focus,
        TuiStyleRole::Selected,
        TuiStyleRole::Status,
        TuiStyleRole::Muted,
        TuiStyleRole::Warning,
        TuiStyleRole::Error,
    ] {
        assert!(REQUIRED_STYLE_ROLES.contains(&role), "{role:?}");
    }
}

#[test]
fn approved_colored_accents_are_retained() {
    let theme = TuiTheme::colored();
    let status_style = theme.style(TuiStyleRole::Status);
    let pane_style = theme.style(TuiStyleRole::Focus);
    let selected_style = theme.style(TuiStyleRole::Selected);

    assert_eq!(status_style.fg, Some(Color::Rgb(0x4d, 0xd0, 0xe1)));
    assert_eq!(pane_style.fg, Some(Color::Rgb(0x4d, 0xd0, 0xe1)));
    assert!(pane_style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(selected_style.fg, Some(Color::Rgb(0xff, 0xd5, 0x4f)));
    assert!(!selected_style.add_modifier.contains(Modifier::REVERSED));
}

#[test]
fn confirmation_roles_use_approved_dark_and_light_palettes() {
    let dark = TuiTheme::dark();
    assert_eq!(
        dark.style(TuiStyleRole::ConfirmationFlow).fg,
        Some(Color::Rgb(0xbe, 0x95, 0xff))
    );
    assert!(
        dark.style(TuiStyleRole::ConfirmationFlow)
            .add_modifier
            .contains(Modifier::BOLD)
    );

    let light = TuiTheme::light();
    assert_eq!(
        light.style(TuiStyleRole::ConfirmationFlow).fg,
        Some(Color::Rgb(0x69, 0x29, 0xc4))
    );
    assert!(
        contrast_ratio((0xbe, 0x95, 0xff), (0x26, 0x32, 0x38)) >= 4.5,
        "dark confirmation-flow contrast"
    );
    assert!(
        contrast_ratio((0x69, 0x29, 0xc4), (0xfa, 0xfa, 0xfa)) >= 4.5,
        "light confirmation-flow contrast"
    );

    let no_color = TuiTheme::no_color().style(TuiStyleRole::ConfirmationFlow);
    assert_eq!(no_color.fg, None);
    assert!(no_color.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn modal_control_copy_is_consistent_across_all_states_and_themes() {
    for theme in [TuiTheme::dark(), TuiTheme::light(), TuiTheme::no_color()] {
        let mut help = test_state_with_theme(theme);
        help.show_help = true;
        let help_buffer = rendered_buffer(&mut help, 160, 44);
        assert!(help_buffer.contains(ModalDismissal::Help.instruction()));
        assert!(help_buffer.contains("q             Quit (idle base screen)"));
        assert_eq!(
            footer_text(&help, 160),
            "Up/Down Scroll  PgUp/PgDn Page  Esc Close  ? Close"
        );
        assert!(!help_buffer.contains("q Close"));

        let mut empty_menu = test_state_with_theme(theme);
        empty_menu.action_menu = Some(ActionMenuState {
            kind: ActionMenuKind::Log,
            selected: 0,
            feedback: None,
            feedback_scope: ActionMenuFeedbackScope::ModalState,
            log_target: None,
            response_export: None,
        });
        let empty_menu_buffer = rendered_buffer(&mut empty_menu, 160, 44);
        assert!(empty_menu_buffer.contains(ModalDismissal::Close.instruction()));
        assert_eq!(footer_text(&empty_menu, 160), "Esc Close");
        assert!(!empty_menu_buffer.contains("Enter selects."));

        let mut cancel_states = Vec::new();

        let mut action_menu = test_state_with_theme(theme);
        action_menu.response = ResponseState::masked("OK");
        action_menu.focus = Pane::Response;
        handle_key_code(&mut action_menu, KeyCode::Enter);
        cancel_states.push(action_menu);

        let mut confirmation = test_state_with_theme(theme);
        confirmation.confirmation = Some(ConfirmationState::new(Preset::built_in(
            "change",
            "AT+CMEE=2",
            ["review"],
        )));
        cancel_states.push(confirmation);

        let mut masking = test_state_with_theme(theme);
        masking.output_masking_ack_input = Some(String::new());
        cancel_states.push(masking);

        let mut response_copy = test_state_with_theme(theme);
        response_copy.response_action_confirmation = Some(ResponseActionConfirmationState {
            action: ResponseActionConfirmation::Copy {
                contents: "response".to_owned(),
                response_label: "Response".to_owned(),
            },
            input: String::new(),
            error: None,
        });
        cancel_states.push(response_copy);

        let mut raw_path = test_state_with_theme(theme);
        raw_path.raw_log_path_input = Some(RawLogPathInputState::default());
        cancel_states.push(raw_path);

        let mut raw_ack = test_state_with_theme(theme);
        raw_ack.raw_log_ack_input = Some(RawLogAckInputState {
            path: PathBuf::from("case.rawlog"),
            input: String::new(),
            error: None,
        });
        cancel_states.push(raw_ack);

        let mut search = test_state_with_theme(theme);
        search.search_input = Some(SearchInputState::default());
        cancel_states.push(search);

        let mut edit = test_state_with_theme(theme);
        edit.edit_input = Some(EditInputState {
            input: "AT+QCFG?".to_owned(),
            error: None,
            timeout_secs: DEFAULT_COMMAND_TIMEOUT_SECS,
        });
        cancel_states.push(edit);

        let mut timeout = test_state_with_theme(theme);
        timeout.timeout_input = Some(TimeoutInputState {
            input: "120".to_owned(),
            error: None,
        });
        cancel_states.push(timeout);

        let mut ad_hoc = test_state_with_theme(theme);
        ad_hoc.ad_hoc_input = Some(AdHocInputState::default());
        cancel_states.push(ad_hoc);

        let mut sequence_params = test_state_with_sequences_and_theme(theme);
        sequence_params.selected_category = sequence_params
            .categories
            .iter()
            .position(|category| category == "sms")
            .expect("sms category");
        handle_key_code(&mut sequence_params, KeyCode::Enter);
        cancel_states.push(sequence_params);

        let required_sequence = Sequence::built_in(
            "required-review",
            "Review a state change before execution.",
            vec!["review".to_owned()],
            Some(30),
            Vec::new(),
            vec![test_sequence_step("AT+CFUN=1")],
        );
        let mut required_review = TuiState::new_with_all_usb(
            executable_items(Vec::new(), vec![required_sequence]),
            vec![test_usb_device(1, 3, "Onyx A")],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        handle_key_code(&mut required_review, KeyCode::Enter);
        cancel_states.push(required_review);

        let review_sequence = Sequence::built_in(
            "review-only",
            "Review an observation before execution.",
            vec!["review".to_owned()],
            Some(30),
            vec![SequenceParam {
                name: "note".to_owned(),
                label: "Note".to_owned(),
                required: true,
                sensitive: false,
                default_value: None,
                source: SequenceParamSource::User,
                candidate: None,
                hint: None,
            }],
            vec![test_sequence_step("AT+CSQ")],
        );
        let mut review_only = TuiState::new_with_all_usb(
            executable_items(Vec::new(), vec![review_sequence]),
            vec![test_usb_device(1, 3, "Onyx A")],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        handle_key_code(&mut review_only, KeyCode::Enter);
        for value in "review".chars() {
            handle_key_code(&mut review_only, KeyCode::Char(value));
        }
        handle_key_code(&mut review_only, KeyCode::Enter);
        cancel_states.push(review_only);

        let mut candidate_confirmation = test_state_with_sequences_and_theme(theme);
        candidate_confirmation.selected_category = candidate_confirmation
            .categories
            .iter()
            .position(|category| category == "sms")
            .expect("sms category");
        candidate_confirmation.selected_command = candidate_confirmation
            .visible_commands()
            .iter()
            .position(|command| command.name() == "sms-read-message")
            .expect("sms-read-message command");
        handle_key_code(&mut candidate_confirmation, KeyCode::Enter);
        handle_key_code(&mut candidate_confirmation, KeyCode::Down);
        handle_key_code(&mut candidate_confirmation, KeyCode::Enter);
        cancel_states.push(candidate_confirmation);

        for (index, state) in cancel_states.iter_mut().enumerate() {
            let buffer = rendered_buffer(state, 160, 44);
            if state.timeout_input.is_none() {
                assert!(
                    buffer.contains(ModalDismissal::Cancel.instruction()),
                    "missing cancel instruction in {:?} modal state {index}",
                    theme.mode(),
                );
            }
            assert!(footer_text(state, 160).contains("Esc Cancel"));
            assert!(!buffer.contains("Esc/q"));
            assert!(!buffer.contains("Esc or q"));
            assert!(!buffer.contains("q Close"));
        }
    }
}

#[test]
fn modal_geometry_uses_wrapped_content_height_and_existing_maximum() {
    let screen = Rect::new(0, 0, 100, 40);
    let short = vec![
        Line::from("first"),
        Line::from("second"),
        Line::from("third"),
    ];
    let short_geometry = modal_geometry_for_lines(screen, 64, 32, &short);
    assert_eq!(short_geometry.area.height, 5);
    assert!(short_geometry.content_fits);

    let long = (1..=40)
        .map(|index| Line::from(format!("line {index}")))
        .collect::<Vec<_>>();
    let long_geometry = modal_geometry_for_lines(screen, 64, 32, &long);
    assert_eq!(
        long_geometry.area.height,
        centered_rect(64, 32, screen).height
    );
    assert!(!long_geometry.content_fits);
}

#[test]
fn canonical_wrapped_layout_uses_word_boundaries_for_measurement_and_rendering() {
    let layout = WrappedTextLayout::new(vec![Line::from("records transmitted and received")]);
    let line_count = layout.line_count(12);
    assert_eq!(line_count, 3);

    let backend = TestBackend::new(12, line_count as u16);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| frame.render_widget(layout.into_paragraph(), frame.area()))
        .unwrap();
    let rows = buffer_rows(terminal.backend().buffer());
    assert_eq!(rows, vec!["records", "transmitted", "and received"]);
}

#[test]
fn wrapped_layout_and_ellipsis_preserve_extended_grapheme_clusters() {
    let combined = "e\u{301}e\u{301}";
    let layout = WrappedTextLayout::new(vec![Line::from(combined)]);
    assert_eq!(layout.line_count(1), 2);

    let backend = TestBackend::new(1, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| frame.render_widget(layout.into_paragraph(), frame.area()))
        .unwrap();
    assert_eq!(
        buffer_rows(terminal.backend().buffer()),
        vec!["e\u{301}", "e\u{301}"]
    );

    assert_eq!(truncate_with_ellipsis("e\u{301}cho", 2), "e\u{301}…");
    assert_eq!(truncate_with_ellipsis("👨‍👩‍👧‍👦 family", 3), "👨‍👩‍👧‍👦…");
    assert_eq!(truncate_with_ellipsis("🇯🇵abc", 3), "🇯🇵…");
    assert_eq!(truncate_with_ellipsis("a\u{200b}b", 2), "a\u{200b}b");

    let cjk = WrappedTextLayout::new(vec![Line::from("通信")]);
    assert_eq!(cjk.line_count(2), 2);
}

#[test]
fn long_unbroken_text_remains_reachable_through_visual_row_scroll() {
    let layout = WrappedTextLayout::new(vec![Line::from("AT+PAYLOAD=ABCDEFGHIJKLMNOPQRSTUVWXYZ")]);
    let index = layout.line_index(10);
    assert!(index.total_rows > 2);
    let scroll = index.max_scroll(2);

    let backend = TestBackend::new(10, 2);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| frame.render_widget(layout.paragraph_at(&index, scroll), frame.area()))
        .unwrap();
    let rendered = buffer_rows(terminal.backend().buffer()).join("");
    assert!(rendered.contains("Z"), "{rendered}");
}

#[test]
fn wrapped_line_index_maps_visual_rows_back_to_source_lines() {
    let layout = WrappedTextLayout::new(vec![
        Line::from("short"),
        Line::from("this source line wraps across several visual rows"),
        Line::from("end"),
    ]);
    let index = layout.line_index(10);
    assert!(index.total_rows > layout.lines.len());

    let inside_second_line = index.line_start(1).saturating_add(1);
    assert_eq!(
        index.visible_logical_range(inside_second_line, 1),
        Some((1, 1))
    );
    assert_eq!(
        response_range_title(3, 1, 1, false, false),
        "Response 2-2/3 more above/below"
    );
}

#[test]
fn wrapped_log_scroll_reaches_bottom_and_retains_source_line_identity() {
    let mut state = test_state();
    state.focus = Pane::Response;
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        path: PathBuf::from("/tmp/wrapped.session.log"),
        label: "wrapped.session.log".to_owned(),
    });
    state.response = ResponseState::masked(format!("{}\ntail", "word ".repeat(18)));
    let theme = state.theme;

    let backend = TestBackend::new(24, 5);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_response(frame, frame.area(), &mut state, &theme))
        .unwrap();
    assert!(state.response_scroll == 0);
    assert!(response_max_scroll(&state) > 0);

    handle_key_code(&mut state, KeyCode::End);
    terminal
        .draw(|frame| render_response(frame, frame.area(), &mut state, &theme))
        .unwrap();
    let rendered = format!("{:?}", terminal.backend().buffer());
    assert!(rendered.contains("bottom"), "{rendered}");
    assert!(rendered.contains("2  tail"), "{rendered}");
}

#[test]
fn saved_log_rows_remain_one_row_and_use_unicode_safe_ellipsis() {
    let mut state = test_state();
    state.logs = vec![
        test_log_entry(
            LogListingKind::Session,
            "/tmp/👨‍👩‍👧‍👦-a-very-long-session-log-name.session.log",
        ),
        test_log_entry(LogListingKind::Session, "/tmp/second.session.log"),
    ];
    state.selected_log = 0;
    state.focus = Pane::History;
    let theme = state.theme;

    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_history(frame, frame.area(), &mut state, &theme))
        .unwrap();
    let rows = buffer_rows(terminal.backend().buffer());
    assert!(
        rows.iter()
            .any(|row| row.contains("> session:") && row.contains('…'))
    );
    assert!(rows.iter().any(|row| row.contains("  session:")));
    assert_eq!(
        rows.iter().filter(|row| row.contains("session:")).count(),
        2
    );
    assert_eq!(state.logs_visible_height, 5);
}

#[test]
fn representative_modal_states_end_after_their_last_visible_control() {
    let mut search = test_state();
    handle_key_code(&mut search, KeyCode::Char('/'));
    assert_content_fit_modal_bottom(
        &mut search,
        "Commands / Sequences search",
        "Enter applies. Esc cancels.",
        160,
        44,
    );

    let mut confirmation = test_state();
    confirmation.selected_command = 1;
    handle_key_code(&mut confirmation, KeyCode::Enter);
    assert_content_fit_modal_bottom(
        &mut confirmation,
        "Confirm command",
        "Input: <empty>",
        160,
        44,
    );

    let mut action_menu = test_state();
    action_menu.response = ResponseState::masked("OK");
    action_menu.focus = Pane::Response;
    handle_key_code(&mut action_menu, KeyCode::Enter);
    assert_content_fit_modal_bottom(
        &mut action_menu,
        "Response actions",
        "Enter selects. Esc cancels.",
        160,
        44,
    );

    let mut sequence = test_state_with_sequences();
    sequence.selected_category = sequence
        .categories
        .iter()
        .position(|category| category == "sms")
        .expect("sms category");
    sequence.selected_command = sequence
        .visible_commands()
        .iter()
        .position(|item| item.name() == "sms-send-check")
        .expect("sms-send-check");
    handle_key_code(&mut sequence, KeyCode::Enter);
    assert_content_fit_modal_bottom(
        &mut sequence,
        "Run Sequence: sms-send-check",
        "Enter: Next value (2 of 2).",
        160,
        44,
    );

    let mut details = test_state();
    handle_key_code(&mut details, KeyCode::Char('d'));
    assert_content_fit_modal_bottom(
        &mut details,
        "Details ·",
        "Confirmation: not required",
        160,
        44,
    );

    let mut help = test_state();
    handle_key_code(&mut help, KeyCode::Char('?'));
    assert_content_fit_modal_bottom(
        &mut help,
        "Help",
        "Unmasked Response: copy / export confirmation",
        160,
        44,
    );
}

#[test]
fn representative_modal_states_paint_their_complete_rectangle_in_colored_themes() {
    for (theme, expected_background) in [
        (TuiTheme::dark(), Color::Rgb(0x26, 0x32, 0x38)),
        (TuiTheme::light(), Color::Rgb(0xfa, 0xfa, 0xfa)),
    ] {
        let mut search = test_state_with_theme(theme);
        handle_key_code(&mut search, KeyCode::Char('/'));
        assert_modal_background(
            &mut search,
            "Commands / Sequences search",
            expected_background,
            160,
            44,
        );

        let mut confirmation = test_state_with_theme(theme);
        confirmation.selected_command = 1;
        handle_key_code(&mut confirmation, KeyCode::Enter);
        assert_modal_background(
            &mut confirmation,
            "Confirm command",
            expected_background,
            160,
            44,
        );

        let mut action_menu = test_state_with_theme(theme);
        action_menu.response = ResponseState::masked("OK");
        action_menu.focus = Pane::Response;
        handle_key_code(&mut action_menu, KeyCode::Enter);
        assert_modal_background(
            &mut action_menu,
            "Response actions",
            expected_background,
            160,
            44,
        );

        let mut sequence = test_state_with_sequences_and_theme(theme);
        sequence.selected_category = sequence
            .categories
            .iter()
            .position(|category| category == "sms")
            .expect("sms category");
        sequence.selected_command = sequence
            .visible_commands()
            .iter()
            .position(|item| item.name() == "sms-send-check")
            .expect("sms-send-check");
        handle_key_code(&mut sequence, KeyCode::Enter);
        assert_modal_background(
            &mut sequence,
            "Run Sequence: sms-send-check",
            expected_background,
            160,
            44,
        );
        for value in "+819012345678".chars() {
            handle_key_code(&mut sequence, KeyCode::Char(value));
        }
        handle_key_code(&mut sequence, KeyCode::Enter);
        for value in "hello".chars() {
            handle_key_code(&mut sequence, KeyCode::Char(value));
        }
        handle_key_code(&mut sequence, KeyCode::Enter);
        assert_modal_background(
            &mut sequence,
            "Confirm Sequence: sms-send-check",
            expected_background,
            160,
            44,
        );

        let mut details = test_state_with_theme(theme);
        handle_key_code(&mut details, KeyCode::Char('d'));
        assert_modal_background(&mut details, "Details ·", expected_background, 160, 44);

        let mut help = test_state_with_theme(theme);
        handle_key_code(&mut help, KeyCode::Char('?'));
        assert_modal_background(&mut help, "Help", expected_background, 160, 44);
    }
}

#[test]
fn help_uses_a_bounded_working_viewport_when_content_is_long() {
    let mut state = test_state();
    handle_key_code(&mut state, KeyCode::Char('?'));

    let first = rendered_buffer(&mut state, 100, 12);
    assert!(first.contains("Help 1-"));
    assert!(first.contains("atctl TUI"));
    assert!(footer_text(&state, 100).contains("PgUp/PgDn Page"));

    handle_key_code(&mut state, KeyCode::PageDown);
    let next = rendered_buffer(&mut state, 100, 12);
    assert!(state.help_scroll > 0);
    assert!(next.contains("Help "));
    assert!(!next.contains("atctl TUI"));

    handle_key_code(&mut state, KeyCode::Home);
    rendered_buffer(&mut state, 100, 12);
    assert_eq!(state.help_scroll, 0);

    handle_key_code(&mut state, KeyCode::End);
    let last = rendered_buffer(&mut state, 100, 12);
    assert!(state.help_scroll > 0);
    assert!(last.contains("Unmasked Response: copy / export confirmation"));
}

#[test]
fn confirmation_render_uses_warning_for_decision_and_focus_for_input() {
    for (theme, warning, focus) in [
        (
            TuiTheme::dark(),
            Color::Rgb(0xff, 0xd1, 0x66),
            Color::Rgb(0x4d, 0xd0, 0xe1),
        ),
        (
            TuiTheme::light(),
            Color::Rgb(0x7a, 0x5a, 0x00),
            Color::Rgb(0x00, 0x7c, 0x89),
        ),
        (TuiTheme::no_color(), Color::Reset, Color::Reset),
    ] {
        let mut state = TuiState::new(
            vec![Preset::built_in("change", "AT+CMEE=2", ["review"])],
            vec![test_usb_device(1, 3, "Onyx A")],
            Vec::new(),
            theme,
        );
        handle_key_code(&mut state, KeyCode::Enter);

        let backend = TestBackend::new(120, 32);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_frame(frame, &mut state))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let instruction = buffer_line_position(buffer, "Type `run` to send").expect("instruction");
        let input = buffer_line_position(buffer, "Input: <empty>").expect("input");
        let instruction_cell = buffer.cell(instruction).expect("instruction cell");
        let input_cell = buffer.cell(input).expect("input cell");

        assert_eq!(instruction_cell.fg, warning);
        assert_eq!(input_cell.fg, focus);
        assert!(instruction_cell.modifier.contains(Modifier::BOLD));
        assert!(input_cell.modifier.contains(Modifier::BOLD));
    }
}

#[test]
fn raw_export_path_is_focus_and_acknowledgement_is_warning() {
    let mut state = test_state();
    run_control(&mut state, ControlAction::RawExport);

    assert_eq!(state.status_role, TuiStyleRole::Status);
    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let path_buffer = terminal.backend().buffer();
    let path_title =
        buffer_line_position(path_buffer, "Raw diagnostic export path").expect("path title");
    assert_eq!(
        path_buffer.cell(path_title).map(|cell| cell.fg),
        Some(Color::Rgb(0x4d, 0xd0, 0xe1))
    );

    let path = unique_temp_dir("raw-review-style").join("case.rawlog");
    for value in path.to_string_lossy().chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.raw_log_path_input.is_none());
    assert!(state.raw_log_ack_input.is_some());
    let backend = TestBackend::new(120, 32);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let ack_buffer = terminal.backend().buffer();
    let ack_bounds =
        modal_bounds(ack_buffer, "Start raw diagnostic export?").expect("acknowledgement modal");
    let ack_text = normalized_rect_text(ack_buffer, ack_bounds);
    assert!(
        ack_text.contains(RAW_LOG_WARNING),
        "missing complete canonical raw warning in acknowledgement modal:\n{ack_text}"
    );
    let ack_title = buffer_line_position(ack_buffer, "Start raw diagnostic export?")
        .expect("acknowledgement title");
    let ack_input = buffer_line_position(ack_buffer, "Input: <empty>").expect("ack input");
    assert_eq!(
        ack_buffer.cell(ack_title).map(|cell| cell.fg),
        Some(Color::Rgb(0xff, 0xd1, 0x66))
    );
    assert_eq!(
        ack_buffer.cell(ack_input).map(|cell| cell.fg),
        Some(Color::Rgb(0x4d, 0xd0, 0xe1))
    );

    let compact = rendered_buffer_cells(&mut state, 80, 24);
    assert!(buffer_line_position(&compact, "Start raw diagnostic export?").is_some());
    assert!(buffer_line_position(&compact, "Input: <empty>").is_some());
    let compact_bounds = modal_bounds(&compact, "Start raw diagnostic export?")
        .expect("compact acknowledgement modal");
    assert!(normalized_rect_text(&compact, compact_bounds).contains(RAW_LOG_WARNING));
}

#[test]
fn no_color_disables_foreground_colors_but_keeps_non_color_emphasis() {
    let theme = TuiTheme::no_color();

    for role in REQUIRED_STYLE_ROLES {
        assert_eq!(theme.style(role).fg, None, "{role:?}");
    }
    assert!(
        theme
            .style(TuiStyleRole::Focus)
            .add_modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        theme
            .style(TuiStyleRole::Selected)
            .add_modifier
            .contains(Modifier::BOLD)
    );
    assert!(
        theme
            .style(TuiStyleRole::Warning)
            .add_modifier
            .contains(Modifier::BOLD)
    );
}

#[test]
fn no_color_environment_value_controls_color_mode() {
    assert_eq!(
        TuiTheme::from_choice_and_no_color_value(None, None).mode(),
        TuiThemeMode::Dark
    );
    assert_eq!(
        TuiTheme::from_choice_and_no_color_value(None, Some("")).mode(),
        TuiThemeMode::NoColor
    );
    assert_eq!(
        TuiTheme::from_choice_and_no_color_value(None, Some("1")).mode(),
        TuiThemeMode::NoColor
    );
    assert_eq!(
        TuiTheme::from_choice_and_no_color_value(Some(TuiThemeChoice::Dark), Some("1")).mode(),
        TuiThemeMode::Dark
    );
    assert_eq!(
        TuiTheme::from_choice_and_no_color_value(Some(TuiThemeChoice::Light), Some("1")).mode(),
        TuiThemeMode::Light
    );
}

fn test_state() -> TuiState {
    test_state_with_theme(TuiTheme::dark())
}

fn test_state_with_theme(theme: TuiTheme) -> TuiState {
    TuiState::new(
        test_commands(),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_log_entry(
            LogListingKind::History,
            "/tmp/history.jsonl",
        )],
        theme,
    )
}

fn test_state_with_sequences() -> TuiState {
    test_state_with_sequences_and_theme(TuiTheme::dark())
}

fn test_state_with_sequences_and_theme(theme: TuiTheme) -> TuiState {
    TuiState::new_with_all_usb(
        executable_items(test_commands(), crate::sequences::builtin::builtins()),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_log_entry(
            LogListingKind::History,
            "/tmp/history.jsonl",
        )],
        theme,
    )
}

fn test_sequence_step(send: &str) -> SequenceStep {
    SequenceStep {
        id: "send".to_owned(),
        label: None,
        ensure_pdp_context_active: None,
        send: Some(send.to_owned()),
        expect: Some("OK".to_owned()),
        expect_prompt: None,
        expect_urc: None,
        payload: None,
        terminator: StepTerminator::None,
        require_tcp_ack: false,
        require_ping_success: false,
        timeout_secs: Some(30),
        evidence: None,
        cleanup_on_failure: None,
    }
}

fn sms_candidate(
    index: &str,
    status: &str,
    sender: &str,
    timestamp: &str,
    body_preview: &str,
) -> SmsMessageCandidate {
    SmsMessageCandidate {
        index: index.to_owned(),
        status: Some(status.to_owned()),
        sender: Some(sender.to_owned()),
        masked_sender: Some("******".to_owned()),
        timestamp: Some(timestamp.to_owned()),
        raw_body_preview: Some(body_preview.to_owned()),
        masked_body_preview: Some("<masked sensitive body>".to_owned()),
    }
}

fn sms_candidate_set(candidates: Vec<SmsMessageCandidate>) -> TuiSequenceCandidateSet {
    TuiSequenceCandidateSet {
        candidate: SequenceCandidateSource::SmsMessage,
        candidates: candidates.into_iter().map(sms_value_candidate).collect(),
        source_label: "last sms-receive-check result".to_owned(),
        acquired_at: "2026-06-25T00-00-00-000000000Z".to_owned(),
    }
}

fn sms_value_candidate(candidate: SmsMessageCandidate) -> SequenceValueCandidate {
    let status = candidate.status.as_deref().unwrap_or("-");
    let sender = candidate.sender.as_deref().unwrap_or("-");
    let masked_sender = candidate.masked_sender.as_deref().unwrap_or("-");
    let timestamp = candidate.timestamp.as_deref().unwrap_or("-");
    let raw_preview = candidate.raw_body_preview.as_deref().unwrap_or("-");
    let masked_preview = candidate.masked_body_preview.as_deref().unwrap_or("-");
    SequenceValueCandidate {
        value: candidate.index.clone(),
        raw_label: format!(
            "storage={}  {}  {}  {}  {}",
            candidate.index, status, sender, timestamp, raw_preview
        ),
        masked_label: format!(
            "storage={}  {}  {}  {}  {}",
            candidate.index, status, masked_sender, timestamp, masked_preview
        ),
    }
}

fn rendered_buffer(state: &mut TuiState, width: u16, height: u16) -> String {
    format!("{:?}", rendered_buffer_cells(state, width, height))
}

fn rendered_buffer_cells(state: &mut TuiState, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render_frame(frame, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn rendered_commands_buffer(state: &TuiState, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_commands(frame, frame.area(), state, &state.theme))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn normalized_rect_text(buffer: &Buffer, area: Rect) -> String {
    let mut text = String::new();
    let left = area.x.saturating_add(1);
    let right = area.x.saturating_add(area.width).saturating_sub(1);
    let top = area.y.saturating_add(1);
    let bottom = area.y.saturating_add(area.height).saturating_sub(1);
    for y in top..bottom {
        for x in left..right {
            if let Some(cell) = buffer.cell((x, y)) {
                text.push_str(cell.symbol());
            }
        }
        text.push('\n');
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn buffer_rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buffer.area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    row.push_str(cell.symbol());
                }
            }
            row.trim_end().to_owned()
        })
        .filter(|row| !row.is_empty())
        .collect()
}

fn assert_content_fit_modal_bottom(
    state: &mut TuiState,
    title: &str,
    final_line: &str,
    width: u16,
    height: u16,
) {
    let buffer = rendered_buffer_cells(state, width, height);
    let bounds = modal_bounds(&buffer, title).unwrap_or_else(|| panic!("missing modal: {title}"));
    let (_, final_y) = buffer_line_position(&buffer, final_line)
        .unwrap_or_else(|| panic!("missing final modal line: {final_line}"));
    assert_eq!(
        bounds.y.saturating_add(bounds.height).saturating_sub(1),
        final_y.saturating_add(1),
        "{title} retained rows below its final visible control"
    );
}

fn assert_modal_background(
    state: &mut TuiState,
    title: &str,
    expected_background: Color,
    width: u16,
    height: u16,
) {
    let theme = state.theme;
    let buffer = rendered_buffer_cells(state, width, height);
    let bounds = modal_bounds(&buffer, title).unwrap_or_else(|| panic!("missing modal: {title}"));
    for y in bounds.y..bounds.y.saturating_add(bounds.height) {
        for x in bounds.x..bounds.x.saturating_add(bounds.width) {
            assert_eq!(
                buffer.cell((x, y)).map(|cell| cell.bg),
                Some(expected_background),
                "unexpected modal background at ({x}, {y}) for {:?}",
                theme.mode()
            );
        }
    }
}

fn modal_bounds(buffer: &Buffer, title: &str) -> Option<Rect> {
    for top_y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            line.push_str(buffer.cell((x, top_y))?.symbol());
        }
        let Some(byte_index) = line.find(title) else {
            continue;
        };
        let title_x = line[..byte_index].chars().count() as u16;
        let Some(left_x) = (0..=title_x).rev().find(|x| {
            buffer
                .cell((*x, top_y))
                .is_some_and(|cell| cell.symbol() == "┌")
        }) else {
            continue;
        };
        let Some(right_x) = (title_x..buffer.area.width).find(|x| {
            buffer
                .cell((*x, top_y))
                .is_some_and(|cell| cell.symbol() == "┐")
        }) else {
            continue;
        };
        let Some(bottom_y) = (top_y..buffer.area.height).find(|y| {
            buffer
                .cell((left_x, *y))
                .is_some_and(|cell| cell.symbol() == "└")
                && buffer
                    .cell((right_x, *y))
                    .is_some_and(|cell| cell.symbol() == "┘")
        }) else {
            continue;
        };
        return Some(Rect::new(
            left_x,
            top_y,
            right_x.saturating_sub(left_x).saturating_add(1),
            bottom_y.saturating_sub(top_y).saturating_add(1),
        ));
    }
    None
}

fn rendered_status_buffer(state: &mut TuiState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let theme = state.theme;
    terminal
        .draw(|frame| render_status(frame, frame.area(), state, &theme))
        .unwrap();
    format!("{:?}", terminal.backend().buffer())
}

fn line_containing<'a>(buffer: &'a str, needle: &str) -> &'a str {
    buffer
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("missing {needle:?} in buffer:\n{buffer}"))
}

fn assert_contains_in_order(haystack: &str, needles: &[&str]) {
    let mut previous_index = 0;
    for needle in needles {
        let relative_index = haystack[previous_index..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in buffer:\n{haystack}"));
        previous_index += relative_index + needle.len();
    }
}

fn buffer_line_position(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                line.push_str(cell.symbol());
            }
        }
        if let Some(byte_index) = line.find(needle) {
            return Some((line[..byte_index].chars().count() as u16, y));
        }
    }
    None
}

fn contrast_ratio(foreground: (u8, u8, u8), background: (u8, u8, u8)) -> f64 {
    fn luminance(color: (u8, u8, u8)) -> f64 {
        fn channel(value: u8) -> f64 {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * channel(color.0) + 0.7152 * channel(color.1) + 0.0722 * channel(color.2)
    }

    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn test_commands() -> Vec<Preset> {
    vec![
        Preset::built_in("modem-response", "AT", ["basic"]),
        Preset::built_in("disable-command-echo", "ATE0", ["basic"]),
        Preset::built_in("imsi", "AT+CIMI", ["sim"]),
        Preset::built_in("material-reset", "AT+CFUN=0", ["modem"]),
    ]
}

fn run_control(state: &mut TuiState, action: ControlAction) -> TuiAction {
    state.focus = Pane::Controls;
    state.selected_control = control_rows(state)
        .iter()
        .position(|row| row.action == action)
        .expect("control action");
    handle_key_code(state, KeyCode::Enter)
}

fn run_action_menu(
    state: &mut TuiState,
    kind: ActionMenuKind,
    action: ActionMenuAction,
) -> TuiAction {
    let response_export = if kind == ActionMenuKind::Response && state.viewed_log.is_none() {
        response_export_request(state)
    } else {
        None
    };
    let selected = action_menu_rows(state, kind)
        .iter()
        .position(|row| row.action == action)
        .expect("action menu row");
    state.action_menu = Some(ActionMenuState {
        kind,
        selected,
        feedback: None,
        feedback_scope: ActionMenuFeedbackScope::Action,
        log_target: if kind == ActionMenuKind::Log {
            state.selected_log().cloned()
        } else {
            None
        },
        response_export,
    });
    handle_key_code(state, KeyCode::Enter)
}

fn test_log_entry(kind: LogListingKind, path: impl Into<PathBuf>) -> LogEntry {
    let path = path.into();
    let prefix = match kind {
        LogListingKind::History => "history",
        LogListingKind::Session => "session",
    };
    LogEntry {
        kind,
        label: format!("{prefix}: {}", compact_path_label(&path)),
        path,
    }
}

fn test_usb_device(bus: u8, address: u8, product: &str) -> UsbDeviceInfo {
    UsbDeviceInfo {
        bus,
        address,
        vendor_id: 0x2c7c,
        product_id: 0x0125,
        class_code: 0xef,
        sub_class_code: 0x02,
        protocol_code: 0x01,
        num_configurations: 1,
        manufacturer: Some("Quectel".to_owned()),
        product: Some(product.to_owned()),
        serial_number: None,
    }
}

fn test_diagnostic_usb_device(bus: u8, address: u8, product: &str) -> UsbDeviceInfo {
    UsbDeviceInfo {
        bus,
        address,
        vendor_id: 0x0bda,
        product_id: 0x0411,
        class_code: 0x09,
        sub_class_code: 0x00,
        protocol_code: 0x03,
        num_configurations: 1,
        manufacturer: Some("Generic".to_owned()),
        product: Some(product.to_owned()),
        serial_number: None,
    }
}

fn test_class_zero_diagnostic_usb_device(bus: u8, address: u8, product: &str) -> UsbDeviceInfo {
    UsbDeviceInfo {
        bus,
        address,
        vendor_id: 0x1234,
        product_id: 0x5678,
        class_code: 0x00,
        sub_class_code: 0x00,
        protocol_code: 0x00,
        num_configurations: 1,
        manufacturer: Some("Example".to_owned()),
        product: Some(product.to_owned()),
        serial_number: None,
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("atctl-tui-{name}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_logging_paths(name: &str) -> LoggingPaths {
    let dir = unique_temp_dir(name);
    let state_dir = dir.join("state");
    let session_dir = dir.join("logs");
    fs::create_dir_all(&state_dir).unwrap();
    fs::create_dir_all(&session_dir).unwrap();
    LoggingPaths {
        state_dir,
        session_dir,
    }
}

fn unmasked_sensitive_state() -> TuiState {
    let mut state = test_state();
    let mut executor = TestExecutor::default();
    state.selected_command = 2;
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);
    run_control(&mut state, ControlAction::ToggleOutputMasking);
    for value in OUTPUT_UNMASK_ACK.chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    assert!(!state.output_masking_enabled);
    state
}

#[derive(Default)]
struct TestExecutor {
    calls: Vec<(String, bool)>,
    timeouts: Vec<u64>,
    devices: Vec<Option<TuiDeviceSelection>>,
    sequence_params: Vec<Vec<(String, String)>>,
    normal_logging: Vec<bool>,
    sms_candidates: Vec<SmsMessageCandidate>,
    value_candidate_sets: Vec<SequenceValueCandidateSet>,
    preset_error: Option<AtctlError>,
    sequence_error: Option<AtctlError>,
    sequence_status: Option<AtStatus>,
}

impl TuiCommandExecutor for TestExecutor {
    fn execute_item(
        &mut self,
        pending: &PendingExecution,
        raw_log: Option<&mut RawLogSink>,
    ) -> Result<ExecutionOutput> {
        self.calls
            .push((pending.item.name().to_owned(), pending.confirmed));
        self.timeouts.push(pending.timeout_secs);
        self.devices.push(pending.device_selection);
        self.normal_logging.push(pending.normal_logging_enabled);
        let item = match &pending.item {
            ExecutableItem::CandidateAction(action) => action.item.as_ref(),
            item => item,
        };
        match item {
            ExecutableItem::Preset(preset) => {
                if let Some(error) = self.preset_error.take() {
                    return Err(error);
                }
                let (text, raw_text) = if preset.command == "AT+CGACT?" {
                    (
                        "AT+CGACT?\n+CGACT: 1,1\nOK\n".to_owned(),
                        "AT+CGACT?\r\n+CGACT: 1,1\r\nOK\r\n".to_owned(),
                    )
                } else if preset.command == "AT+QISTATE" {
                    (
                            "AT+QISTATE\n+QISTATE: 0,\"TCP\",\"example.com\",80,0,2,1,0,0,\"uart1\"\nOK\n".to_owned(),
                            "AT+QISTATE\r\n+QISTATE: 0,\"TCP\",\"example.com\",80,0,2,1,0,0,\"uart1\"\r\nOK\r\n".to_owned(),
                        )
                } else if preset.review.data_handling.contains(&DataHandling::Output) {
                    (
                        "89811000*******\nOK\n".to_owned(),
                        "898110001234567\nOK\n".to_owned(),
                    )
                } else {
                    (
                        format!("{}\nOK\n", preset.command),
                        format!("{}\nOK\n", preset.command),
                    )
                };
                let operation = OperationIdentity::preset(preset);
                let review = preset.review.clone();
                let masked_text = mask_sensitive_values(&raw_text);
                Ok(ExecutionOutput::Command(Box::new(SendExecution {
                    surface: ExecutionSurface::Tui,
                    masked_operation: operation.masked(),
                    operation,
                    masked_review: review.masked(),
                    review,
                    status: AtStatus::Ok,
                    text: if text.is_empty() {
                        masked_text.clone()
                    } else {
                        text
                    },
                    lines: masked_text.lines().map(str::to_owned).collect(),
                    raw_response: raw_text.as_bytes().to_vec(),
                    raw_text,
                    masked_text,
                    masked: true,
                    duration: Duration::from_millis(7),
                })))
            }
            ExecutableItem::Sequence(sequence) => {
                self.sequence_params.push(
                    pending
                        .sequence_params
                        .iter()
                        .map(|param| (param.name.clone(), param.value.clone()))
                        .collect(),
                );
                if let Some(error) = self.sequence_error.take() {
                    return Err(error);
                }
                if let Some(raw_log) = raw_log {
                    let operation = OperationIdentity::sequence(sequence);
                    raw_log.append_exchange(RawLogExchange {
                        operation: &operation,
                        command_name: Some("test-sequence-step"),
                        command: "AT+CMGF=1",
                        review: &sequence.review,
                        status: &AtStatus::Ok,
                        duration: Duration::from_millis(1),
                        tx_bytes: b"AT+CMGF=1\r",
                        rx_bytes: b"\r\nOK\r\n",
                    })?;
                }
                let value_candidate_sets =
                    if self.value_candidate_sets.is_empty() && !self.sms_candidates.is_empty() {
                        vec![SequenceValueCandidateSet {
                            candidate: SequenceCandidateSource::SmsMessage,
                            candidates: self
                                .sms_candidates
                                .clone()
                                .into_iter()
                                .map(sms_value_candidate)
                                .collect(),
                        }]
                    } else {
                        self.value_candidate_sets.clone()
                    };
                let status = self.sequence_status.clone().unwrap_or(AtStatus::Ok);
                let transcript = if status.is_success() {
                    format!("Sequence {}\nResult: OK", sequence.name)
                } else {
                    format!(
                        "Sequence {}\nResult: failed duration=7ms\nReason: test sequence status failure",
                        sequence.name
                    )
                };
                let review = sequence.review.clone();
                Ok(ExecutionOutput::Sequence(Box::new(SequenceExecution {
                    name: sequence.name.clone(),
                    masked_review: review.masked(),
                    review,
                    status,
                    steps: Vec::new(),
                    masked_notes: Vec::new(),
                    raw_notes: Vec::new(),
                    value_candidate_sets,
                    masked_transcript: transcript.clone(),
                    raw_transcript: transcript,
                    duration: Duration::from_millis(7),
                })))
            }
            ExecutableItem::CandidateAction(_) => unreachable!("candidate actions are unwrapped"),
        }
    }
}
