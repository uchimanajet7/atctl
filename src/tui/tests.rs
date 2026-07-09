use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::prelude::{Color, Modifier};
use ratatui::symbols;

use crate::at::response::AtStatus;
use crate::at::risk::RiskLevel;
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
        handle_key_code(&mut state, KeyCode::Char('?')),
        TuiAction::Continue
    );
    assert!(state.show_help);
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('q')),
        TuiAction::Continue
    );
    assert!(!state.show_help);
    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('q')),
        TuiAction::Quit
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
    assert!(buffer.contains("Timeout 30s"));
    assert!(buffer.contains("Start raw export"));
    assert!(buffer.contains("Output masking on"));
    assert!(!buffer.contains("Rerun last"));
    assert!(!buffer.contains("Save response"));
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
    assert!(buffer.contains("Save response"));
    assert!(buffer.contains("Open response folder"));
    assert!(buffer.contains("Response folder:"));
    assert!(!buffer.contains("Saves to:"));
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
    assert!(buffer.contains("Open logs folder"));
    assert!(buffer.contains("Logs folder:"));
    assert!(!buffer.contains("    Folder:"));
    assert!(!buffer.contains("Copy log path"));
    assert!(!buffer.contains("Copy log dir"));
    assert!(!buffer.contains("Open log dir"));
}

#[test]
fn response_enter_while_viewing_log_renders_log_view_actions() {
    let mut state = test_state();
    state.focus = Pane::Response;
    state.response = ResponseState::masked("masked log body");
    state.active_command = None;
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        label: "test.session.log".to_owned(),
    });

    handle_key_code(&mut state, KeyCode::Enter);
    let buffer = rendered_buffer(&mut state, 100, 32);

    assert!(buffer.contains("Log view actions"));
    assert!(buffer.contains("Copy displayed log"));
    assert!(buffer.contains("Open logs folder"));
    assert!(buffer.contains("Close log view"));
    assert!(buffer.contains("Logs folder:"));
    assert!(!buffer.contains("Save response"));
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

    assert!(confirmation_buffer.contains("Command requires confirmation"));
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
                RiskLevel::Write,
                vec!["pdp".to_owned(), "apn".to_owned()],
                PresetOrigin::file("SORACOM commands", "soracom.toml", None),
            ),
            Preset::built_in("modem-response", "AT", RiskLevel::Safe, ["basic"]),
            Preset::new(
                "qccid",
                "AT+QCCID",
                RiskLevel::Sensitive,
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
            RiskLevel::Write,
            vec!["basic".to_owned()],
            PresetOrigin::file("External commands", "external.toml", None),
        )],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    handle_key_code(&mut state, KeyCode::Enter);
    let buffer = rendered_buffer(&mut state, 120, 32);

    assert!(buffer.contains("Command requires confirmation"));
    assert!(buffer.contains("Source: External commands"));
    assert!(buffer.contains("File: external.toml"));
    assert!(buffer.contains("Review external definition before running."));
}

#[test]
fn preset_set_headers_are_not_command_selection_targets() {
    let mut state = TuiState::new(
        vec![
            Preset::built_in("modem-response", "AT", RiskLevel::Safe, ["basic"]),
            Preset::new(
                "qccid",
                "AT+QCCID",
                RiskLevel::Sensitive,
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
                Preset::built_in("pdp-contexts", "AT+CGDCONT?", RiskLevel::Safe, ["data"]),
                Preset::new(
                    "set-soracom-apn-cid1",
                    "AT+CGDCONT=1,\"IP\",\"soracom.io\"",
                    RiskLevel::Write,
                    vec!["data".to_owned(), "apn".to_owned()],
                    PresetOrigin::file("SORACOM commands", "soracom.toml", None),
                ),
            ],
            vec![
                Sequence::built_in(
                    "sms-send-check",
                    "Send an SMS and check receive state.",
                    RiskLevel::Write,
                    vec!["data".to_owned(), "sms".to_owned()],
                    Some(180),
                    Vec::new(),
                    vec![test_sequence_step("AT+CMGS=\"+12025550123\"")],
                ),
                Sequence::new(
                    "quectel-tcp-send-check",
                    "Open a Quectel TCP socket, send a payload, and read a response.",
                    RiskLevel::Write,
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
    assert!(status_buffer.contains("Source: Quectel"));
    assert!(!status_buffer.contains("Sequence set: Quectel Sequences"));
}

#[test]
fn file_preset_commands_preserve_entry_order_within_preset_set() {
    let state = TuiState::new(
        vec![
            Preset::new(
                "second-quectel",
                "AT+QCSQ",
                RiskLevel::Safe,
                vec!["signal".to_owned()],
                PresetOrigin::file("Quectel commands", "quectel.toml", None),
            ),
            Preset::new(
                "first-quectel",
                "AT+QCCID",
                RiskLevel::Sensitive,
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
fn enter_executes_safe_command_through_executor() {
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
fn ad_hoc_input_executes_safe_command_through_executor() {
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
fn ad_hoc_write_command_uses_existing_confirmation_flow() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();

    run_control(&mut state, ControlAction::AdHocCommand);
    for value in "ATE0".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.confirmation.is_some());
    assert!(state.pending_execution.is_none());
    for value in "write".chars() {
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
    assert!(buffer.contains("Send a standard SMS"));
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
    assert!(buffer.contains("Risk:"));
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
    for value in "write".chars() {
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
    for value in "write".chars() {
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
    for value in "write".chars() {
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
    assert!(status_buffer.contains("Risk:"));
    assert_contains_in_order(
        &status_buffer,
        &[
            "Status: failed",
            &failed_line,
            "Sequence:",
            "sms-receive-check",
            "Result: ERROR 7ms",
            "Risk:",
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
    assert!(buffer.contains("Run Sequence"));
    assert!(buffer.contains("Recipient"));
    assert!(buffer.contains("Message body"));

    for value in "+819012345678".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    let input_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(input_buffer.contains("+819012345678"));
    handle_key_code(&mut state, KeyCode::Enter);
    for value in "hello from atctl".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    let confirmation_buffer = rendered_buffer(&mut state, 120, 32);
    assert!(confirmation_buffer.contains("Review:"));
    assert!(confirmation_buffer.contains("Destination"));
    assert!(confirmation_buffer.contains("+819012345678"));
    assert!(confirmation_buffer.contains("Message body"));
    assert!(confirmation_buffer.contains("hello from atctl"));
    for value in "write".chars() {
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
        executable_items(Vec::new(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );

    handle_key_code(&mut state, KeyCode::Enter);

    let buffer = rendered_buffer(&mut state, 140, 36);
    assert!(buffer.contains("Values:"));
    assert!(buffer.contains("PDP context ID: 1  default"));
    assert!(buffer.contains("Socket connect ID: 0  default"));
    assert!(buffer.contains("Payload (sensitive): <empty>  user"));
    assert!(buffer.contains("Read length: 1500  default"));
    assert!(buffer.contains("Confirm the PDP context"));
    assert!(buffer.contains("During execution"));
    assert!(buffer.contains("AT+QIACT?"));
    assert!(buffer.contains("Load active PDP contexts  AT+CGACT?"));
    assert!(buffer.contains("Load PDP context definitions  AT+CGDCONT?"));
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
    let confirmation_buffer = rendered_buffer(&mut state, 140, 36);
    assert!(confirmation_buffer.contains("Values:"));
    assert!(confirmation_buffer.contains("PDP context ID: 1  default"));
    assert!(confirmation_buffer.contains("Socket connect ID: 0  default"));
    assert!(confirmation_buffer.contains("Payload (sensitive): hello  user"));
    assert!(confirmation_buffer.contains("Read length: 1500  default"));
    assert!(confirmation_buffer.contains("Destination"));
    assert!(confirmation_buffer.contains("unified.soracom.io:23080"));
    assert!(confirmation_buffer.contains("Expected effect:"));
    assert!(confirmation_buffer.contains("Type `write` to run."));
    assert!(confirmation_buffer.contains("Input: <empty>"));

    let compact_confirmation_buffer = rendered_buffer(&mut state, 140, 24);
    assert!(compact_confirmation_buffer.contains("detail omitted"));
    assert!(compact_confirmation_buffer.contains("Type `write` to run."));
    assert!(compact_confirmation_buffer.contains("Input: <empty>"));

    for value in "wr".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    let typed_confirmation_buffer = rendered_buffer(&mut state, 140, 24);
    assert!(typed_confirmation_buffer.contains("Type `write` to run."));
    assert!(typed_confirmation_buffer.contains("Input: wr"));
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
        executable_items(Vec::new(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
    let mut executor = TestExecutor::default();

    handle_key_code(&mut state, KeyCode::Enter);
    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.sequence_input.is_some());
    let pending = state.pending_execution.as_ref().expect("candidate action");
    assert_eq!(pending.item.name(), "Load active PDP contexts");

    execute_pending_command(&mut state, &mut executor);

    assert!(state.sequence_input.is_some());
    let buffer = rendered_buffer(&mut state, 140, 36);
    assert!(buffer.contains("Candidates: last direct PDP context check result (1 total"));
    assert!(buffer.contains("> 1  active"));
    assert!(buffer.contains("No modem read is performed by this modal."));
    assert!(buffer.contains("Actions:"));
    assert!(buffer.contains("Load active PDP contexts  AT+CGACT?"));
    assert!(buffer.contains("Load PDP context definitions  AT+CGDCONT?"));
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
        executable_items(Vec::new(), vec![sequence]),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        Vec::new(),
        TuiTheme::dark(),
    );
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
    assert!(buffer.contains("Load active PDP contexts  AT+CGACT?"));

    handle_key_code(&mut state, KeyCode::Down);
    let action_buffer = rendered_buffer(&mut state, 140, 36);
    assert!(action_buffer.contains("> Load active PDP contexts  AT+CGACT?"));

    handle_key_code(&mut state, KeyCode::Enter);
    let pending = state.pending_execution.as_ref().expect("candidate action");
    assert_eq!(pending.item.name(), "Load active PDP contexts");
}

#[test]
fn run_sequence_modal_confirms_write_risk_candidate_action_before_execution() {
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
    let confirmation_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(confirmation_buffer.contains("Confirm action."));
    assert!(confirmation_buffer.contains("Action: Load received SMS list"));
    assert!(confirmation_buffer.contains("Command: AT+CMGL=\"ALL\""));
    assert!(confirmation_buffer.contains("Risk:"));
    assert!(confirmation_buffer.contains("Type `write` to run."));
    assert!(confirmation_buffer.contains("Input: <empty>"));

    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.pending_execution.is_none());
    assert!(state.sequence_input.as_ref().is_some_and(|input| {
        input
            .error
            .as_deref()
            .is_some_and(|error| error.contains("Type `write` exactly"))
    }));

    for value in "write".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    let pending = state.pending_execution.as_ref().expect("candidate action");
    assert_eq!(pending.item.name(), "Load received SMS list");
    assert!(pending.confirmed);
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Params)
    );
}

#[test]
fn failed_candidate_action_is_not_rendered_as_selected_sequence_failure() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor {
        preset_error: Some(AtctlError::Transport(
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
    for value in "write".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert!(state.sequence_input.is_some());
    let status_buffer = rendered_status_buffer(&mut state, 64, 12);
    assert!(status_buffer.contains("Status: failed"));
    assert!(status_buffer.contains("Action: Load received SMS list"));
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
fn run_sequence_modal_selects_sms_index_candidate_without_leaving_modal() {
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
    assert!(buffer.contains("SMS storage index: unresolved  select"));
    assert!(buffer.contains("Candidates: last sms-receive-check result (2 total"));
    assert!(buffer.contains("No modem read is performed by this modal."));
    assert!(buffer.contains(
        "> storage=3  REC UNREAD  90****  26/06/23,06:03:02+00  <masked sensitive body>"
    ));
    assert!(
        buffer.contains(
            "  storage=4  REC READ  90****  26/06/23,08:09:02+00  <masked sensitive body>"
        )
    );
    assert!(buffer.contains("Actions:"));
    assert!(buffer.contains("Load received SMS list  AT+CMGL=\"ALL\""));
    assert!(buffer.contains("Enter selects or runs action"));

    handle_key_code(&mut state, KeyCode::Down);
    let moved_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(
        moved_buffer.contains(
            "> storage=4  REC READ  90****  26/06/23,08:09:02+00  <masked sensitive body>"
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
        Some(SequenceInputPhase::Params)
    );
    let selected_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(selected_buffer.contains("SMS storage index: 4  select"));

    handle_key_code(&mut state, KeyCode::Enter);
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::Confirmation)
    );
    let confirmation_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(confirmation_buffer.contains("Review:"));
    assert!(confirmation_buffer.contains("SMS storage index"));
    assert!(confirmation_buffer.contains("4"));
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
    assert!(buffer.contains("Load received SMS list  AT+CMGL=\"ALL\""));

    handle_key_code(&mut state, KeyCode::Down);
    handle_key_code(&mut state, KeyCode::Down);
    let action_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(action_buffer.contains("> Load received SMS list  AT+CMGL=\"ALL\""));

    handle_key_code(&mut state, KeyCode::Enter);
    assert!(state.pending_execution.is_none());
    assert_eq!(
        state.sequence_input.as_ref().map(|input| input.phase),
        Some(SequenceInputPhase::CandidateActionConfirmation)
    );
    let confirmation_buffer = rendered_buffer(&mut state, 150, 36);
    assert!(confirmation_buffer.contains("Action: Load received SMS list"));
    assert!(confirmation_buffer.contains("Type `write` to run."));
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
    assert!(buffer.contains("> storage=7  REC READ  90****  26/06/24,11:16:08+00"));
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
    for value in "write".chars() {
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
        "> storage=5  REC UNREAD  90****  26/06/24,11:16:08+00  <masked sensitive body>"
    ));
}

#[test]
fn completed_cmgl_command_updates_sms_index_candidates() {
    let mut state = test_state_with_sequences();
    let output = ExecutionOutput::Command(SendExecution {
            risk: RiskLevel::Write,
            status: AtStatus::Ok,
            text: "+CMGL: 6,\"REC READ\",\"901001\",,\"26/06/24,12:00:00+00\"\nSMS body (ucs2): direct command\nOK\n".to_owned(),
            lines: Vec::new(),
            raw_response: Vec::new(),
            raw_text: "AT+CMGL=\"ALL\"\r\n+CMGL: 6,\"REC READ\",\"901001\",,\"26/06/24,12:00:00+00\"\r\n00640069007200650063007400200063006F006D006D0061006E0064\r\n\r\nOK\r\n".to_owned(),
            masked: false,
            duration: Duration::from_millis(7),
        });

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
    assert!(candidate.masked_label.contains("90****"));
}

#[test]
fn raw_capture_for_tui_sequence_uses_sequence_step_events() {
    let mut state = test_state_with_sequences();
    let mut executor = TestExecutor::default();
    let path = unique_temp_dir("sequence-raw-capture").join("case.rawlog");
    state.raw_capture =
        Some(RawLogSink::create(RawLogConfig::new(path.clone(), "tui", "tui")).unwrap());
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
    for value in "write".chars() {
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
fn edit_before_run_write_command_requires_confirmation() {
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
    for value in "write".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);
    execute_pending_command(&mut state, &mut executor);

    assert_eq!(executor.calls, vec![("edited".to_owned(), true)]);
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
fn save_current_response_writes_masked_response_file() {
    let state = unmasked_sensitive_state();
    let dir = unique_temp_dir("saved-response");

    let path = write_current_response(&dir, &state).unwrap();
    let contents = fs::read_to_string(&path).unwrap();

    assert!(
        path.parent()
            .is_some_and(|parent| parent.ends_with("responses"))
    );
    assert!(path.display().to_string().ends_with(".response.txt"));
    assert!(contents.contains("AT+CIMI"));
    assert!(contents.contains("89811000*******"));
    assert!(!contents.contains("898110001234567"));
}

#[test]
fn response_actions_without_body_only_offer_response_folder() {
    let mut state = test_state();
    state.response.clear();

    let rows = action_menu_rows(&state, ActionMenuKind::Response);

    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenResponseDirectory)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::CopyResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::SaveResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::ClearResponse)
    );
}

#[test]
fn response_actions_show_response_folder_without_path_copy_actions() {
    let state = test_state();
    let rows = action_menu_rows(&state, ActionMenuKind::Response);

    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenResponseDirectory)
    );
    let context = action_menu_context_line(&state, ActionMenuKind::Response).expect("context");
    assert!(context.starts_with("Response folder: "));
    assert!(context.contains("responses"));
    assert!(!rows.iter().any(|row| row.label.contains("path")));
    assert!(!rows.iter().any(|row| row.label.contains("dir")));
}

#[test]
fn write_command_opens_confirmation_without_executing() {
    let mut state = test_state();
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);

    assert!(state.confirmation.is_some());
    assert!(state.pending_execution.is_none());
    assert!(state.response.contains("Command requires confirmation"));
    assert!(state.response.contains("Risk: [write] CONFIRM"));
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
fn confirmation_match_executes_write_command() {
    let mut state = test_state();
    let mut executor = TestExecutor::default();
    state.selected_command = 1;

    handle_key_code(&mut state, KeyCode::Enter);
    for value in "write".chars() {
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
fn dangerous_commands_are_visible_and_confirmation_required() {
    let state = test_state();
    let visible_names = state
        .visible_commands()
        .iter()
        .map(|preset| preset.name())
        .collect::<Vec<_>>();

    assert!(visible_names.contains(&"danger-reset"));

    let dangerous = state
        .visible_commands()
        .into_iter()
        .find(|preset| preset.name() == "danger-reset")
        .expect("dangerous preset should be visible");
    assert!(dangerous.risk().requires_confirmation());
}

#[test]
fn dangerous_commands_remain_visible_when_category_selection_is_invalid() {
    let mut state = test_state();
    state.selected_category = usize::MAX;
    let visible_names = state
        .visible_commands()
        .iter()
        .map(|preset| preset.name())
        .collect::<Vec<_>>();

    assert!(visible_names.contains(&"danger-reset"));
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
        RiskLevel::Safe,
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
    let mut state = test_state();
    state.running_execution = Some(RunningExecution {
        started_at: Instant::now(),
        timeout: Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
    });

    assert_eq!(
        handle_key_code(&mut state, KeyCode::Char('q')),
        TuiAction::Continue
    );
    assert!(state.status.contains("Command is running"));
    assert!(state.running_execution.is_some());
}

#[test]
fn timeout_input_sets_temporary_execution_timeout() {
    let mut state = test_state();

    run_control(&mut state, ControlAction::SetTimeout);
    handle_key_code(&mut state, KeyCode::Backspace);
    handle_key_code(&mut state, KeyCode::Backspace);
    for value in "180".chars() {
        handle_key_code(&mut state, KeyCode::Char(value));
    }
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.timeout_override_secs, Some(180));
    state.focus = Pane::Commands;
    handle_key_code(&mut state, KeyCode::Enter);
    let pending = state.pending_execution.as_ref().expect("pending execution");
    assert_eq!(pending.timeout_secs, 180);
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
    assert_eq!(state.status_role, TuiStyleRole::Warning);

    run_control(&mut state, ControlAction::AdHocCommand);

    assert!(state.ad_hoc_input.is_none());
    assert!(state.response.contains("No matching USB device"));
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
    let mut state = TuiState::new_with_all_usb(
        executable_items(test_commands(), Vec::new()),
        Vec::new(),
        vec![test_diagnostic_usb_device(1, 2, "USB Hub")],
        Vec::new(),
        TuiTheme::dark(),
    );

    state.focus = Pane::Devices;
    state.highlighted_device = state.devices.len();
    handle_key_code(&mut state, KeyCode::Enter);

    assert_eq!(state.device_view, DeviceView::AllUsbTroubleshooting);
    let lines = device_lines(&state, &TuiTheme::dark())
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
}

#[test]
fn all_usb_troubleshooting_view_can_select_visible_operation_target() {
    let target = test_usb_device(1, 3, "Onyx A");
    let mut state = TuiState::new_with_all_usb(
        executable_items(test_commands(), Vec::new()),
        vec![target.clone()],
        vec![test_diagnostic_usb_device(1, 2, "USB Hub"), target],
        Vec::new(),
        TuiTheme::dark(),
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
                RiskLevel::Safe,
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
        .map(|index| Preset::built_in(format!("cmd-{index:02}"), "AT", RiskLevel::Safe, ["bulk"]))
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
    let configured_log_dir = dir.join("configured-logs");
    let default_log_dir = state_dir.join("logs");
    fs::create_dir_all(&state_dir).unwrap();
    fs::create_dir_all(&configured_log_dir).unwrap();
    fs::create_dir_all(&default_log_dir).unwrap();
    fs::write(state_dir.join("history.jsonl"), "{}\n").unwrap();
    fs::write(
        configured_log_dir.join("2026-07-03T00-00-00Z.session.log"),
        "configured",
    )
    .unwrap();
    fs::write(
        default_log_dir.join("2026-07-03T01-00-00Z.session.log"),
        "default",
    )
    .unwrap();

    let entries = log_entries_from_paths(&LoggingPaths {
        state_dir,
        session_dir: configured_log_dir,
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
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLogsDirectory)
    );
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
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLogsDirectory)
    );
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
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLogsDirectory)
    );
    let buffer = rendered_buffer(&mut state, 100, 24);
    assert!(buffer.contains("Selected log no longer exists"));
    assert!(buffer.contains("2026-07-03T02-50-00Z.session.log"));
    assert!(buffer.contains("Open logs folder"));
    assert!(!buffer.contains("Open log in Response"));
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
        .map(|index| Preset::built_in(format!("cmd-{index:02}"), "AT", RiskLevel::Safe, ["bulk"]))
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
  "response": "+QCCID: 8942310020003626445F"
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
        response_range_title(log_line_count, 10, 0),
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
    assert!(buffer.contains("Risk: [safe]"));
}

#[test]
fn clear_response_removes_previous_rendered_content() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in(
        "signal-quality",
        "AT+CSQ",
        RiskLevel::Safe,
        ["signal"],
    ));
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
    assert!(buffer.contains("Risk: [safe]"));
    let status_buffer = rendered_status_buffer(&mut state, 80, 12);
    assert_contains_in_order(
        &status_buffer,
        &[
            "Status: completed",
            "Completed: 2026-07-02T11:41:12Z",
            "Command: signal-quality",
            "AT command: AT+CSQ",
            "Result: OK 7ms",
            "Risk:",
        ],
    );
    assert!(status_buffer.contains("Command: signal-quality"));
    assert!(status_buffer.contains("AT command: AT+CSQ"));
    assert_eq!(state.status, "Response body cleared.");

    let rows = action_menu_rows(&state, ActionMenuKind::Response);
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenResponseDirectory)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::CopyResponse)
    );
    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::SaveResponse)
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
    assert!(buffer.contains("Risk:"));
    assert!(buffer.contains("Output masking: off"));
    assert_contains_in_order(
        &buffer,
        &[
            "Status: completed",
            &completed_line,
            "Command: modem-response",
            "AT command: AT",
            "Result: OK 7ms",
            "Risk:",
            "Output masking: off",
        ],
    );
}

#[test]
fn cancelled_execution_status_shows_cancelled_event_timestamp() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in(
        "signal-quality",
        "AT+CSQ",
        RiskLevel::Safe,
        ["signal"],
    ));
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
            "Risk:",
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

    assert!(buffer.contains("> modem-response"));
    assert!(buffer.contains("[safe]"));
    assert!(buffer.contains("[sensitive]"));
}

#[test]
fn selected_command_keeps_risk_label_and_non_color_cue() {
    let mut state = test_state();
    state.selected_command = 2;
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| render_frame(frame, &mut state))
        .unwrap();
    let buffer = format!("{:?}", terminal.backend().buffer());

    assert!(buffer.contains("> imsi"));
    assert!(buffer.contains("[sensitive]"));
    assert!(buffer.contains("MASKED"));
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
fn safe_response_does_not_show_mask_state_when_no_values_are_masked() {
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
    assert!(buffer.contains("Risk: [safe]"));
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
    assert_eq!(state.status, COPY_REQUEST_SENT_FEEDBACK);
    assert_eq!(state.status_role, TuiStyleRole::Status);
    let copy_row = action_menu_rows(&state, ActionMenuKind::Response)
        .into_iter()
        .find(|row| row.action == ActionMenuAction::CopyResponse)
        .expect("copy row");
    assert_eq!(copy_row.label, "Copy response");

    finish_clipboard_copy(&mut state, Err(AtctlError::Transport("blocked".to_owned())));
    assert_eq!(
        state.status,
        "Copy request failed: transport error: blocked"
    );
    assert_eq!(state.status_role, TuiStyleRole::Error);
    let copy_row = action_menu_rows(&state, ActionMenuKind::Response)
        .into_iter()
        .find(|row| row.action == ActionMenuAction::CopyResponse)
        .expect("copy row");
    assert_eq!(copy_row.label, "Copy response");
}

#[test]
fn log_actions_offer_open_folder_without_path_copy_actions() {
    let mut state = test_state();
    let log_path = PathBuf::from("/tmp/atctl/logs/2026-06-18T04-02-06Z.session.log");
    state.logs = vec![test_log_entry(LogListingKind::Session, log_path.clone())];
    state.selected_log = 0;

    let rows = action_menu_rows(&state, ActionMenuKind::Log);

    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLogsDirectory)
    );
    let context = action_menu_context_line(&state, ActionMenuKind::Log).expect("context");
    assert!(context.starts_with("Logs folder: "));
    assert!(context.contains("logs"));
    assert!(!rows.iter().any(|row| row.label == "Copy log path"));
    assert!(!rows.iter().any(|row| row.label == "Copy log folder"));
}

#[test]
fn log_actions_without_selected_log_still_offer_logs_folder() {
    let mut state = test_state();
    state.logs.clear();

    let rows = action_menu_rows(&state, ActionMenuKind::Log);

    assert!(
        !rows
            .iter()
            .any(|row| row.action == ActionMenuAction::OpenLog)
    );
    assert!(
        rows.iter()
            .any(|row| row.action == ActionMenuAction::OpenLogsDirectory)
    );
}

#[test]
fn log_view_close_clears_opened_log_without_status_clear_action() {
    let mut state = test_state();
    state.focus = Pane::Response;
    state.response = ResponseState::masked("persisted log body");
    state.viewed_log = Some(ViewedLog {
        kind: LogListingKind::Session,
        label: "test.session.log".to_owned(),
    });

    run_action_menu(
        &mut state,
        ActionMenuKind::Response,
        ActionMenuAction::CloseLogView,
    );

    assert!(state.action_menu.is_none());
    assert!(state.response.is_empty());
    assert!(state.viewed_log.is_none());
    assert_eq!(state.status, "Log view closed.");
}

#[test]
fn response_copy_prefixes_command_when_modem_does_not_echo() {
    let mut state = test_state();
    let command = ExecutableItem::Preset(Preset::built_in(
        "signal-quality",
        "AT+CSQ",
        RiskLevel::Safe,
        ["signal"],
    ));
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

    assert!(state.output_masking_ack_input.is_none());
    assert!(state.output_masking_enabled);

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
    assert!(!buffer.contains("Expected effect: reads sensitive"));
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
        TuiStyleRole::Focus,
        TuiStyleRole::Selected,
        TuiStyleRole::Status,
        TuiStyleRole::Muted,
        TuiStyleRole::RiskSafe,
        TuiStyleRole::RiskSensitive,
        TuiStyleRole::RiskWrite,
        TuiStyleRole::RiskPersistent,
        TuiStyleRole::RiskDangerous,
        TuiStyleRole::RiskUnknown,
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
fn risk_roles_use_approved_dark_and_light_palettes() {
    let dark = TuiTheme::dark();
    assert_eq!(
        dark.risk_style(RiskLevel::Sensitive).fg,
        Some(Color::Rgb(0xd6, 0xb3, 0xff))
    );
    assert_eq!(
        dark.risk_style(RiskLevel::Dangerous).fg,
        Some(Color::Rgb(0xff, 0x6b, 0x6b))
    );

    let light = TuiTheme::light();
    assert_eq!(
        light.risk_style(RiskLevel::Safe).fg,
        Some(Color::Rgb(0x00, 0x7c, 0x89))
    );
    assert_eq!(
        light.risk_style(RiskLevel::Dangerous).fg,
        Some(Color::Rgb(0xb0, 0x00, 0x20))
    );
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
    TuiState::new(
        test_commands(),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_log_entry(
            LogListingKind::History,
            "/tmp/history.jsonl",
        )],
        TuiTheme::dark(),
    )
}

fn test_state_with_sequences() -> TuiState {
    TuiState::new_with_all_usb(
        executable_items(test_commands(), crate::sequences::builtin::builtins()),
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_usb_device(1, 3, "Onyx A")],
        vec![test_log_entry(
            LogListingKind::History,
            "/tmp/history.jsonl",
        )],
        TuiTheme::dark(),
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
        masked_sender: Some("90****".to_owned()),
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
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render_frame(frame, state)).unwrap();
    format!("{:?}", terminal.backend().buffer())
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

fn test_commands() -> Vec<Preset> {
    vec![
        Preset::built_in("modem-response", "AT", RiskLevel::Safe, ["basic"]),
        Preset::built_in("disable-command-echo", "ATE0", RiskLevel::Write, ["basic"]),
        Preset::built_in("imsi", "AT+CIMI", RiskLevel::Sensitive, ["sim"]),
        Preset::built_in("danger-reset", "AT+CFUN=0", RiskLevel::Dangerous, ["modem"]),
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
    sms_candidates: Vec<SmsMessageCandidate>,
    value_candidate_sets: Vec<SequenceValueCandidateSet>,
    preset_error: Option<AtctlError>,
    sequence_error: Option<AtctlError>,
    sequence_status: Option<AtStatus>,
}

impl TuiCommandExecutor for TestExecutor {
    fn execute_item(
        &mut self,
        item: &ExecutableItem,
        confirmed: bool,
        timeout_secs: u64,
        device_selection: Option<TuiDeviceSelection>,
        sequence_params: &[SequenceParamValue],
        raw_log: Option<&mut RawLogSink>,
    ) -> Result<ExecutionOutput> {
        self.calls.push((item.name().to_owned(), confirmed));
        self.timeouts.push(timeout_secs);
        self.devices.push(device_selection);
        match item {
            ExecutableItem::Preset(preset) | ExecutableItem::CandidateAction { preset, .. } => {
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
                } else if preset.risk == RiskLevel::Sensitive {
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
                Ok(ExecutionOutput::Command(SendExecution {
                    risk: preset.risk,
                    status: AtStatus::Ok,
                    text,
                    lines: vec![preset.command.clone(), "OK".to_owned()],
                    raw_response: raw_text.as_bytes().to_vec(),
                    raw_text,
                    masked: true,
                    duration: Duration::from_millis(7),
                }))
            }
            ExecutableItem::Sequence(sequence) => {
                self.sequence_params.push(
                    sequence_params
                        .iter()
                        .map(|param| (param.name.clone(), param.value.clone()))
                        .collect(),
                );
                if let Some(error) = self.sequence_error.take() {
                    return Err(error);
                }
                if let Some(raw_log) = raw_log {
                    raw_log.append_exchange(RawLogExchange {
                        command_name: Some("test-sequence-step"),
                        command: "AT+CMGF=1",
                        risk: sequence.risk,
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
                Ok(ExecutionOutput::Sequence(SequenceExecution {
                    name: sequence.name.clone(),
                    risk: sequence.risk,
                    status,
                    steps: Vec::new(),
                    masked_notes: Vec::new(),
                    raw_notes: Vec::new(),
                    value_candidate_sets,
                    masked_transcript: transcript.clone(),
                    raw_transcript: transcript,
                    duration: Duration::from_millis(7),
                }))
            }
        }
    }
}
