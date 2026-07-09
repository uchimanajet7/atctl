# Open Questions

This document tracks decisions that require explicit approval. Implementers must
not guess these values.

Open items are listed first when present. Resolved historical decisions follow.

## OQ-023: Sequences for Multi-Step AT Operations

Status: resolved

Scope:

```text
System: atctl tui, atctl sequence list/run, atctl send, atctl preset run, atctl bridge
Area: SMS send/read/reply checks, vendor/provider data-send checks, multi-step AT execution, TUI layout
```

Decision:

- Use the product-facing term `Sequence` for named multi-step AT operations.
  Do not use `workflow` as the primary user-facing feature name for this
  capability.
- Presets remain one-shot AT command definitions. `atctl send` remains a
  direct one-shot command surface. `atctl preset run` remains a named one-shot
  preset surface.
- Add production CLI surfaces `atctl sequence list` and
  `atctl sequence run <SEQUENCE>` for named multi-step Sequence execution.
- User-authored Sequence definitions are TOML extension files loaded from
  explicit `--sequence-file` or `--sequence-dir` flags. They are available only
  when explicitly loaded, similar to file presets.
- Product-provided standard SMS send/read/reply Sequences must not require
  users to create a TOML file before ordinary use.
- Product-provided standard Sequences, repository-managed example Sequences,
  and user-authored Sequence definitions keep separate origin and review
  responsibility. After loading and validation, they share the applicable
  Sequence contract for listing, selection, risk aggregation, confirmation,
  masking, transcripts, raw diagnostic export, and execution.
- SMS send must review destination and message body before USB access. SMS
  read-by-index must be write-risk because `AT+CMGR` can change unread/read
  status. SMS reply must review SMS storage index and reply body, read the original
  message with `AT+CMGR`, derive the reply destination from the returned sender,
  and then use the standard `AT+CMGS` submit path.
- Quectel TCP/IP data-send checks are vendor-specific and SORACOM TCP endpoint
  checks are provider-specific. They belong in repository-managed example
  Sequence definitions loaded explicitly. They are not default vendor-neutral
  product Sequences.
- The shared Sequence engine must support prompt waits, payload writes,
  Ctrl-Z or ESC terminators, delayed URC waits, final response waits,
  per-step timeouts, total timeout, masking, risk aggregation, raw diagnostic
  export, readable step transcripts with origin sections, active input/review
  items, SMS decoded-body analysis, derived response values such as
  `sms_sender`, structured step results with `analysis`, per-step text
  generated from `evidence`, and success notes.
- Sequence output must distinguish modem/socket submit evidence from
  end-to-end application evidence. `+CMGS` plus `OK` is SMS submit evidence, not
  handset receipt proof. `SEND OK` is Quectel module-accepted-payload evidence,
  not remote application receipt. `+QIOPEN: <id>,0` is socket-open evidence,
  `AT+QISEND=<id>,0` counters are TCP/socket acknowledgement evidence, and
  non-empty `QIRD` output or remote service logs are end-to-end application
  evidence. Repository-managed TCP examples require the counters to show no
  remaining unacknowledged payload bytes before reporting Sequence success.
  `+QIRD: 0` is no buffered response data.
- The TUI must not add another permanent pane for Sequences. The existing
  topology remains aligned top/bottom bands. The executable-item pane becomes
  `Commands / Sequences` once implemented.
- Categories remain workflow categories such as `sms`, `data`, `network`, and
  `pdp`; vendor or source identity such as Quectel must not be generated as a
  category.
- Sequence input belongs in a `Run Sequence` modal or equivalent temporary
  focused surface after a Sequence is selected. That surface must show current
  value, value source, default, or resolution hint instead of bare unexplained
  required fields. Candidate-backed values such as SMS storage index and
  standard PDP context ID must show available candidates in the same modal when
  candidate rows are known from an explicit same-session execution result. When
  candidates are not loaded, the modal must show the explicit product action
  that obtains them. Opening the modal must not perform hidden modem or network
  I/O to populate candidates, and the modal must show candidate source and
  count. Product-known candidate assistance does not make vendor/provider-
  specific TCP Sequence definitions default standard Sequences; repository-
  managed examples and add-ons may use `candidate` only for product-known
  standard parsers and otherwise use defaults, hints, and explicit add-on
  commands. Status shows compact current step context. Response shows the
  Sequence transcript.
- Candidate acquisition actions shown in the `Run Sequence` modal use the same
  risk-confirmation rules as normal command or Sequence execution. If a
  candidate action fails, it is reported as an action failure with full detail
  in Response and concise action state in compact Status; it is not reported as
  a completed failure of the selected Sequence body.
- Existing Controls actions such as timeout, raw diagnostic export, Response
  copy/save, output masking, and clear response apply to Sequence output where
  semantically valid. Controls must not become a dense Sequence-only command
  surface.
- PTY bridge remains a production surface. For Sequence-related completeness,
  it must either support prompt-capable manual multi-step operation for commands
  such as `AT+CMGS`, or a later specification must explicitly approve the
  product difference. The bridge should not be polluted with atctl-specific
  meta-commands unless that is separately approved.

References:

- 3GPP TS 27.007, AT command set for User Equipment:
  https://www.3gpp.org/dynareport/27007.htm
- 3GPP TS 27.005, SMS and CBS AT command interface:
  https://www.3gpp.org/DynaReport/27005.htm
- Quectel EC2x/EG9x/EM05 TCP/IP AT commands manual:
  https://sixfab.com/wp-content/uploads/2018/09/Quectel_EC2xEG9xEM05_TCPIP_AT_Commands_Manual_V1.0.pdf
- RFC 9293, Transmission Control Protocol:
  https://www.rfc-editor.org/rfc/rfc9293
- SORACOM service endpoints:
  https://developers.soracom.io/en/docs/reference/endpoints/
- SORACOM modem testing and Ping Response Service:
  https://developers.soracom.io/en/docs/soracom-onyx-lte-usb-modem/testing/
- SORACOM Unified Endpoint:
  https://developers.soracom.io/en/docs/unified-endpoint/
- Soracom Binary Format v1:
  https://developers.soracom.io/en/docs/groups/binary-format-v1/
- Ratatui Layout:
  https://ratatui.rs/concepts/layout/
- Ratatui Popup:
  https://ratatui.rs/examples/apps/popup/
- Nielsen Norman Group Progressive Disclosure:
  https://www.nngroup.com/articles/progressive-disclosure/
- W3C WCAG 2.4.3 Focus Order:
  https://www.w3.org/WAI/WCAG21/Understanding/focus-order.html

Source:

- User approval on 2026-06-23 after Sequence naming, TUI layout, user-defined
  TOML loading, standard SMS, and Quectel example data-send design discussion.

## OQ-022: TUI Shortcut Reduction and Controls Pane

Status: resolved

Scope:

```text
System: atctl tui
Area: keyboard model, help overlay, Controls pane, Devices pane
```

Decision:

- The TUI primary flow is Categories -> Commands -> `Enter`.
- OQ-023 extends this executable-item surface to `Commands / Sequences` once
  Sequence support is implemented. Until then, the historical OQ-022 `Commands`
  wording refers to the one-shot command preset surface.
- Global letter shortcuts are limited to the small primary set: `/` for command
  search, `?` for help, and `q` for quit.
- Navigation keys, `Enter`, and `Esc` remain basic TUI controls.
- Secondary operations must not each require a global single-letter shortcut.
  They are exposed as `Enter`-activated rows in a normal focusable Controls
  pane.
- The Controls pane is an operation pane, not a shortcut reference or a
  catch-all action menu. It provides AT command input, edit-before-run or
  Sequence inputs, timeout override, raw diagnostic export, and output masking.
- `Rerun last` is not part of the approved Controls model. Repeated execution
  should happen from the selected command/Sequence row so the visible selection
  and executed item stay aligned.
- Response-owned actions are available from a Response action menu opened by
  focusing Response and pressing `Enter`: copy Response, save Response, copy
  last saved Response path, copy last saved Response directory, open last saved
  Response directory, and clear Response.
- Log-owned actions are available from a Logs action menu opened by focusing
  Logs and pressing `Enter`: open selected log in Response, copy selected log
  path, copy selected log directory, and open selected log directory.
- The normal TUI layout uses one canonical pane topology with aligned top and
  bottom bands:
  `Devices + Status / Categories / Commands` in the top band and
  `Controls / Response / Logs` in the bottom band. The horizontal divider
  between bands must align across the full screen.
- The normal vertical allocation uses a stable balanced split between the top
  selection area and the bottom result/review area. This keeps Response usable
  for Sequence transcripts and command output while preserving the approved
  topology. If the usable terminal height leaves one extra row, the extra row
  belongs to the bottom result/review area.
- Devices, Status, and Controls are compact utility panes. They should share a
  compact fixed-width left utility column instead of a broad percentage width.
  Categories should also stay compact, while Commands, Response, and Logs get
  the remaining width because their content is more likely to be long.
- The pane topology and Tab order must not change by terminal width.
- The normal Tab order is
  `Categories -> Commands -> Controls -> Response -> Logs -> Devices`, with
  the device-selection gate still allowed to start in Devices when no execution
  target has been selected.
- Controls rows remain a stable operation list. They should read as actions,
  not as a dense status table. Inline state is limited to values that change the
  action decision itself, such as the current timeout value, raw export
  start/stop state, and output masking state.
- Routine availability values such as `avail`, `no resp`, `ready`, or
  `sel dev` should not be repeated across every Controls row as a permanent
  column. Unavailable actions remain visible, and focusing or activating them
  provides nearby Controls feedback with the reason.
- Immediate actions must provide visible result feedback near the action
  surface that caused them. Controls action feedback belongs in Controls,
  Response action feedback belongs in the Response action menu, and Logs action
  feedback belongs in the Logs action menu. Response copy reports that the
  terminal clipboard request was sent, without claiming that clipboard contents
  were independently verified. Status may keep broader command context but is
  not the primary feedback location for pane actions.
- Running-command progress in Status should be a separated temporary progress
  block: a muted separator, compact timeout-budget text such as
  `Timeout 33/180s left 147s`, and a separate progress bar when height allows.
  The separate progress bar should use a visible filled/unfilled shape cue, not
  color alone. The compact label should avoid looking truncated in the
  fixed-width Status pane. If the available width cannot fit the normal label,
  the TUI may shorten it in steps to `33/180s left 147s` and then `33/180s`.
- Device selection and full-USB troubleshooting view switching belong in the
  Devices pane. They are activated through normal focus navigation and
  `Enter`, not through dedicated global letter shortcuts.
- The help overlay must be modal. While help is visible, ordinary pane actions
  must not execute in the background. `Esc`, `?`, and `q` close help.
- Help content must stay limited to concise keyboard operation and close
  instructions. It must not include pane-architecture explanations such as
  `Primary flow` or descriptive inventories of Controls, Devices, or Logs.
- The footer should stay compact and context-sensitive. It must not become a
  dense inventory of secondary commands.

References:

- Textual Footer:
  https://textual.textualize.io/widgets/footer/
- Textual input binding display:
  https://textual.textualize.io/guide/input/
- Charm Bubble Tea Bubbles help/key components:
  https://github.com/charmbracelet/bubbles
- Lazygit footer/help UX reference:
  https://www.bwplotka.dev/2025/lazygit/
- W3C WCAG 2.4.3 Focus Order:
  https://www.w3.org/WAI/WCAG21/Understanding/focus-order.html
- IBM Accessibility Toolkit tab-order guidance:
  https://www.ibm.com/able/toolkit/design/ux/
- Nielsen Norman Group Visual Hierarchy:
  https://www.nngroup.com/videos/visual-hierarchy/
- Carbon data table style guidance:
  https://carbondesignsystem.com/components/data-table/style/
- Nielsen Norman Group Help and Documentation:
  https://www.nngroup.com/articles/help-and-documentation/
- Nielsen Norman Group 10 Usability Heuristics:
  https://www.nngroup.com/articles/ten-usability-heuristics/
- Material communication guidance:
  https://codelabs.developers.google.com/codelabs/material-communication-guidance

Source:

- User approval on 2026-06-21 after TUI shortcut-count and help-modal behavior
  review, followed by user corrections that Controls placement, Tab order,
  disabled-row handling, and Help content must support the user's TUI task
  flow rather than expose implementation or design-rationale text.

## OQ-021: Raw Log Diagnostic Export

Status: resolved

Scope:

```text
System: atctl send, atctl preset run, atctl tui, atctl bridge
Area: raw modem response capture, sensitive diagnostic export, release-quality CLI surface
```

Decision:

- Raw log output is a possible final diagnostic evidence feature when ordinary
  masked output, masked saved logs, copied responses, and foreground unmasked
  display are insufficient to resolve a modem, device, network, or vendor
  support issue.
- Raw log output is not a normal masked log extension. It may include sensitive
  modem, subscriber, network, APN, or PDP authentication values.
- Raw log output MUST NOT be created automatically in the default state/log
  directory. The user MUST choose the destination explicitly.
- CLI raw export is enabled with `--raw-log-file <PATH>`, not with a boolean
  `--raw-log` switch.
- `atctl send`, `atctl preset run`, and `atctl bridge` MUST require
  `--raw-log-ack raw-log` whenever `--raw-log-file <PATH>` is used in
  non-interactive or `--yes` execution. Interactive CLI execution MAY prompt the
  user to type `raw-log`.
- The TUI MUST treat raw export as an explicit capture mode: the user enters a
  target path, then types `raw-log` before capture starts. Capture writes future
  AT command exchanges until stopped; it MUST NOT retroactively dump old
  responses.
- Raw log acknowledgement is separate from command risk acknowledgement:
  command risk confirms modem state change risk, while raw log acknowledgement
  confirms sensitive data persistence risk.
- Raw export MUST be available consistently across AT execution surfaces:
  `atctl send`, `atctl preset run`, `atctl tui`, and `atctl bridge`.
- Raw export files MUST refuse to overwrite an existing file and SHOULD use
  user-only file permissions when created.
- Terminal, TUI, history, session logs, and saved Response behavior MUST keep
  their existing masking rules. `--no-mask` affects foreground display output
  only and MUST NOT imply raw export creation.
- Raw export format MUST be lossless for transmitted and received bytes. JSONL
  with base64 byte fields is approved for the first implementation.

References:

- https://users.soracom.io/ja-jp/guides/diagnostic/advanced/
- https://curl.se/docs/manpage.html
- https://man7.org/linux/man-pages/man8/tcpdump.8.html
- https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
- https://cwe.mitre.org/data/definitions/532.html

Source:

- User approval on 2026-06-21 after raw log design discussion.

## OQ-018: Running-Command Interruption

Status: resolved

Scope:

```text
System: atctl tui
Area: long-running AT commands, command interruption, host-side read abort, USB reconnect, command resync
```

Decision:

- Do not add a normal `Cancel` action for running AT commands.
- Do not add host-side read abort, USB reconnect, AT resync, endpoint re-probe,
  or session abort as an application feature in the current scope.
- Long-running AT command handling applies to AT commands as a class, not only
  to `AT+COPS=?`.
- Stopping only the host-side read wait must not be described as successful
  command cancellation, because the modem-side operation may still be running or
  may produce delayed output.
- The current implementation should continue to rely on visible running state,
  elapsed time, timeout, remaining time, timeout-budget progress, timeout
  override, and blocking of conflicting sends.
- Modem state changes for cases such as unresponsive or disconnected modems
  should be handled through explicit user-selected AT commands, such as standard
  `AT+CFUN` modem functionality presets or vendor-specific file presets, not
  through a misleading cancellation feature.

Source:

- User direction on 2026-06-20 after discussing that atctl can stop host-side
  waiting but cannot guarantee modem-side command cancellation.

## OQ-019: PTY Bridge Runtime Behavior

Status: resolved

Scope:

```text
System: atctl bridge
Area: PTY bridge symlink, cleanup, signal handling, input translation, safety, and multiple-client behavior
```

Decision:

- Checkpoint 12 PTY bridge implementation uses `portable-pty` as approved in
  OQ-010.
- `atctl bridge --symlink <PATH>` accepts the same USB target selection options
  as direct device commands and resolves the USB target before creating or
  replacing the symlink.
- First-time bridge workflow starts with `atctl devices`. The user chooses the
  target from the current runtime AT operation-target output, preferably by
  copying `bus` and `address` into `--bus <BUS> --address <ADDRESS>`.
- VID/PID values are runtime selectors, not required prior knowledge. They
  should be used for bridge startup only when unique in the current
  `atctl devices` output.
- Existing regular files and directories are never overwritten.
- Existing symlinks are not replaced by default. A separate
  `--replace-symlink` option may replace only an existing symlink.
- Shutdown removes only the symlink created by the current bridge process, and
  only if it still points to the same PTY slave path.
- Signal handling attempts clean shutdown and symlink cleanup for SIGINT,
  SIGTERM, and SIGHUP.
- PTY input is line-oriented. `CR`, `LF`, and `CRLF` all terminate a command.
  Empty commands are ignored.
- Safe and sensitive commands may run from the PTY without confirmation.
  Sensitive responses are masked by default.
- Write, persistent, dangerous, and unknown commands require an exact typed risk
  acknowledgement from the PTY client before sending.
- The initial bridge implementation is a single bridge loop. Multiple external
  clients opening the same PTY symlink concurrently are not supported and may
  interleave input.
- `screen /tmp/atctl 115200` uses `115200` as a serial-tool compatibility
  argument, not as a physical UART baud rate.
- Raw logging, session-wide raw mode, and session-level abort/reconnect/resync
  are not part of Checkpoint 12.

References:

- https://docs.rs/portable-pty/latest/portable_pty/
- https://docs.rs/portable-pty/latest/portable_pty/trait.MasterPty.html
- https://docs.rs/ctrlc/latest/ctrlc/

## OQ-020: Device Listing Default Scope

Status: resolved

Scope:

```text
System: atctl devices
Area: first-time target discovery and full USB troubleshooting output
```

Decision:

- `atctl devices` default output should show plausible `atctl` operation
  targets, not every USB device visible through `libusb`.
- Full USB visibility remains available through `atctl devices --all-usb`.
- The default operation-target view is descriptor-based and must not use a
  built-in known-device list, product-name table, hard-coded VID/PID list, or
  default supported-device allow-list.
- The initial descriptor-based filter is conservative: device classes commonly
  used by communication, miscellaneous, or vendor-specific modem devices, plus
  at least one descriptor shape with both bulk IN and bulk OUT endpoints.
- This descriptor filter is not an AT probe and must not be documented as
  guaranteed modem support.
- If a target is not shown by default, users can inspect `--all-usb` and then
  use explicit runtime selectors such as `--bus` and `--address`.
- The source basis is USB descriptor semantics, not product names:
  - USB-IF Defined Class Codes define class code use for Device and Interface
    Descriptors. `02h` is Communications and CDC Control, `09h` is Hub, `11h`
    is Billboard, `EFh` is Miscellaneous, and `FFh` is Vendor Specific.
  - USB-IF also defines `00h` in a Device Descriptor as "use class code info
    from Interface Descriptors", so a device-class-only filter is a known
    conservative limitation and not a universal modem detector.
  - USB standard descriptor documentation and libusb descriptor documentation
    describe the device / configuration / interface / endpoint descriptor
    hierarchy used by `atctl`.
  - CDC ACM guidance requires paired bulk IN and bulk OUT endpoints for its
    data path, which supports using paired bulk endpoints as a serial-like
    candidate signal without treating it as an AT-probe guarantee.
- The TUI must preserve CLI/TUI discovery parity: the normal Devices pane uses
  the same operation-target scope as `atctl devices`, and the TUI also needs an
  explicitly labeled full-USB troubleshooting view equivalent to
  `atctl devices --all-usb`.
- The TUI full-USB troubleshooting view must preserve the distinction between
  operation targets and non-target USB devices. Non-target devices must be
  visible for diagnosis but not eligible for AT sending.
- The TUI full-USB troubleshooting view is reached from an `Enter`-activated
  row in the Devices pane, not through a dedicated global letter shortcut. CLI
  diagnostics remain available through `atctl devices --all-usb` and
  `atctl inspect --bus <BUS> --address <ADDRESS>`.

Source:

- User direction on 2026-06-19 after `cargo run -- devices` showed unrelated
  LAN, hub, camera, microphone, and billboard USB devices in the first-time
  bridge workflow.
- USB-IF Defined Class Codes:
  https://www.usb.org/defined-class-codes
- Microsoft USB standard descriptors:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/standard-usb-descriptors
- libusb USB descriptors:
  https://libusb.sourceforge.io/api-1.0/group__libusb__desc.html
- Zephyr USB CDC ACM:
  https://docs.zephyrproject.org/latest/services/connectivity/usb/device_next/cdc_acm.html

## OQ-017: TUI Explicit Device Selection Gate

Status: resolved

Scope:

```text
System: atctl tui
Area: Devices pane, command execution gating, multi-device workflow
```

Decision:

- Devices pane is an interactive selection surface when more than one matching
  USB device is visible.
- If no matching USB device is visible, device-dependent actions such as preset
  execution and AT command input sending are disabled. Non-device actions such as
  help, quit, viewing existing logs, scrolling, and copying already displayed
  Response text may remain available.
- If exactly one matching USB device is visible at startup, the TUI should
  auto-select it, show selected-device detail, and allow command execution
  immediately.
- If multiple matching USB devices are visible at startup, the TUI must not
  silently select one. It starts with no active execution device, blocks command
  sending, and requires explicit user selection in Devices first.
- Initial device selection should use the visible Devices list: `d` focuses or
  re-enters Devices selection, `Up` / `Down` move the highlighted candidate,
  and `Enter` selects the highlighted device.
- Once a device is selected, Devices shows detail for the selected target,
  including USB manufacturer when readable, USB product when readable, VID, PID,
  bus, and address.
- Normal Devices display does not show any built-in device label, profile label,
  compatibility label, or agent-defined product name such as `Known`, `[known]`,
  or `Profile hint`.
- The documented validation hardware does not define a closed supported-device
  list.
  The product assumes no pre-known device inventory; discovery uses USB devices
  visible at runtime and explicit user selectors.
- The selected device's VID, PID, bus, and address are passed to TUI command
  execution.
- Device reselection remains available after command execution. A user can
  select device A, run `modem-response`, select device B, and run
  `modem-response` again.
- Reselecting a device changes the target for subsequent commands only; it does
  not rewrite or relabel already displayed Response content.
- Device reselection is disabled while a command is actively running.

Source:

- User direction on 2026-06-19 after reviewing a TUI screenshot with one
  visible Quectel EG25-G / SORACOM Onyx target.

## OQ-016: TUI Device Pane and Long-Running Command Timeout Control

Status: resolved

Scope:

```text
System: atctl tui, preset execution
Area: Devices pane, Status pane, per-command timeout
```

Decision:

- Devices pane must not remain a static placeholder when live USB candidates can
  be enumerated.
- Devices pane should render visible matching USB candidates and show an
  explicit no-device message when none are visible.
- When multiple matching USB devices are visible, TUI command execution should
  use the selected candidate's bus/address. This point is superseded by OQ-017,
  which requires explicit selection before command sending when multiple
  candidates are visible.
- Status should avoid pipe-delimited multi-item lines when vertical space is
  available. It should use key-value lines for state, selected item, optional
  non-default source, command when relevant, risk, timeout, selected device,
  and detail. Sequence summary text belongs in executable rows, review modals,
  search matching, or detail/help surfaces, not in compact Status.
- User AT command default timeout remains 30 seconds.
- Known long-running presets may declare `timeout_secs`.
- `available-operators` / `AT+COPS=?` should declare `timeout_secs = 180`
  because SORACOM documents the scan as typically taking 2 to 3 minutes.
- TUI should provide a Controls pane action to set a temporary session timeout
  override before execution. Entering `default` clears that override.
- Running execution should use the effective timeout from TUI override, preset
  timeout hint, or default timeout, in that order.

## OQ-015: TUI Preset / Ad-Hoc Input Polish Before PTY

Status: resolved

Scope:

```text
System: atctl tui, preset loading, preset safety
Area: status layout, preset set model, user preset files, AT command input
```

Decision:

- PTY bridge work must not start immediately after Checkpoint 11.
- A separate Checkpoint 11.5 must be completed before PTY bridge work.
- Checkpoint 11.5 covers TUI status layout, preset cleanup, preset loading,
  preset set display, effective preset risk, repository-managed file preset
  files, and TUI AT command input.
- The Status area must be compact. It must not absorb large unused space that
  should instead support Response, Logs, or command navigation.
- Status remains responsible for current state and concise command context, but
  it must not duplicate response/log bodies or Sequence purpose summaries.
- Checkpoint 11.5 user-review correction keeps Status compact but places it
  under Devices rather than in a full-width band. Devices plus Status should
  occupy the existing top-left column height, and the lower area should be used
  for Response and Logs.
- The saved history/session list pane should be titled `Logs`; row labels may
  still identify `history:` and `session:` entries.
- User AT command execution timeout should default to 30 seconds. Internal
  endpoint auto-detection probes should stay short and separate from the user
  command timeout.
- TUI command execution should keep redrawing while a command is running and
  should show elapsed time, timeout, remaining time, and a timeout-budget
  progress indicator.
- Product presets must be reorganized around standard modem workflows,
  not limited to passive diagnostics.
- Standard workflow product presets may include modem identity, SIM, network
  registration, signal, PDP/APN readiness, and SMS readiness commands.
- Standard workflow product presets should cover the AT command checkpoints listed
  in SORACOM's advanced data-send/receive troubleshooting reference where those
  commands are vendor-neutral and fit the existing safety model. Current
  coverage includes the SORACOM-listed checkpoints and adjacent standard
  manual troubleshooting commands for radio access, operator format/reset,
  detailed registration reporting, extended signal, PDP authentication, and
  PDP address inspection.
- APN setting and SMS send are valid product workflows. They must not be
  removed from the product direction merely because they can change state or
  require multi-step handling.
- Vendor-specific and carrier-specific commands must be separated from product
  presets into file presets.
- Quectel-specific commands and SORACOM APN setup must be represented as
  repository-managed TOML file preset examples.
- The repository-managed Quectel and SORACOM TOML files must be created and
  verified as part of the same checkpoint that implements multi-file preset
  loading. They are not optional future reference files.
- File presets must be loaded only from explicit per-invocation
  `--preset-file` or `--preset-dir` paths. `~/.config/atctl` must not be used
  as an automatic add-on discovery location.
- Drop-in preset files must be read deterministically and duplicate preset names
  must fail with an actionable error instead of silent override behavior.
- CLI preset lists must show preset set labels such as `Product presets` or a
  file-level TOML `title`, plus source path review fields for file presets.
  CLI preset execution must show source/path review notice before USB access
  even when non-interactive risk acknowledgement is supplied. TUI file preset
  identity must be visible without color through non-selectable source group
  headers or `Source: <title>` detail when file presets are visible. The normal
  TUI list must not add a
  `Product presets` header for default product rows.
- User TOML must declare risk, but declared risk must not be trusted as the
  only safety input.
- Preset execution must use effective risk derived from both declared risk and
  command classification. User-declared risk must not downgrade classifier
  output.
- User-defined vendor commands should be safe to load through conservative
  classification, masking, confirmation, and preset set identity.
- TUI must provide an `AT command` input route for one-off commands that
  are not saved as presets. This route is an `Enter`-activated action in the
  Controls pane, not a dedicated global letter shortcut.
- AT command input must allow ordinary AT command syntax, including quotes, commas,
  semicolons, equals signs, question marks, and command parameters.
- The important safety boundary is command classification and confirmation, not
  arbitrary rejection of normal AT command characters.
- SMS send and other prompt-required multi-step commands must not be treated as
  ordinary one-shot AT command input until a multi-step command design is
  specified and approved.

Rationale:

- `atctl` should help users confirm that a modem, SIM, network registration,
  APN/PDP state, and SMS workflows can actually work, not only inspect passive
  state.
- Separating standard workflow product presets from vendor/carrier file presets keeps the
  default UI understandable while still making practical modem-specific
  workflows available.
- Multi-file TOML preset loading is needed because vendor, carrier, project,
  and personal presets become hard to manage in a single file.
- Repository-managed file preset examples double as real verification material
  for preset set display, drop-in loading, duplicate handling, and risk
  handling.
- Effective risk prevents user or example TOML from accidentally weakening the
  safety model.

Resolved by:

- User direction on 2026-06-18 after the TUI/preset/input proposal review.

## OQ-014: TUI History / Logs Pane Completion Scope

Status: resolved

Scope:

```text
System: atctl tui
Area: history/log viewing
```

Decision:

- Checkpoint 10 remains complete and approved; it must not be reopened for
  History/Logs pane content viewing.
- TUI masked log content viewing is a separate checkpoint before PTY work.
- The TUI must allow masked log content to be read in-app, not only list log
  file names.
- The viewer must use existing masked history/session log files.
- The viewer must not create logs as a side effect of viewing.
- The viewer must not reveal raw response values.
- Opened masked log content must be scrollable inside the TUI.
- The Response pane remains primarily the AT command response pane. Log viewing
  is a temporary masked log-view mode inside that pane, not a replacement of
  the pane's main role.
- While showing masked log content, Response must show line numbers and a
  visible line range in the Response pane itself. The range must describe the
  visible range, not only the first visible line.
- Log-view line numbers are read-only orientation aids. They must not introduce
  or imply an editable cursor.
- Opening a selected saved log should move focus to the Response pane while
  preserving the History selection state.
- Session log files shown in list output or the TUI Logs pane should be ordered
  newest-first. The aggregate `history.jsonl` row may remain separate from that
  newest-first session list because it is a single append-only history file.
- TUI in-pane wording should describe the mixed history/session file list as
  saved logs, not recent logs, when the aggregate history file is present.
- The TUI may copy the selected saved log path and containing directory path.
  It must remain a read-only review surface and must not add log deletion,
  pruning, rotation, raw-log browsing, or OS file-manager launching as default
  TUI behavior.
- Normal focus cycling should prioritize interactive panes and must not route
  through informational-only panes as if they were actionable controls.
- PTY bridge work remains blocked until the TUI masked log viewer checkpoint is
  implemented, reviewed, and approved.

Rationale:

- A file-name-only History pane is useful as implementation evidence but is not
  the completed product experience for reviewing prior command/log material.
- The viewer is separated from Checkpoint 10 to preserve the approved masked/raw
  reveal checkpoint boundary.

Resolved by:

- User approval on 2026-06-18 to implement TUI masked log content viewing as a
  separate phase after completing Checkpoint 10.

## OQ-013: TUI Output Masking

Status: resolved

Scope:

```text
System: atctl tui
Area: foreground Response display masking
```

Decision:

- TUI output masking is on by default.
- `atctl tui --no-mask` starts the TUI session with foreground output masking
  off.
- The TUI Controls pane provides an `Output masking` row with inline state
  `on` or `off`.
- Turning output masking off from inside the TUI requires a confirmation dialog
  with exact typed acknowledgement `unmask`.
- Turning output masking on from inside the TUI may happen immediately.
- The dialog must explain that unmasked sensitive modem, subscriber, payload,
  message, credential, or TCP response values may become visible in the TUI
  Response display.
- The dialog must explain that Response copy follows the visible Response
  display.
- The dialog must explain that saved responses, history, session logs, and raw
  diagnostic export behavior remain separate and masked unless raw diagnostic
  export is started through its own path and `raw-log` acknowledgement.
- `Esc` and `q` cancel the output-masking dialog.
- Output masking off persists until the user turns output masking on again or
  exits the TUI.
- Output masking off survives focus changes, category changes, command
  selection changes, response clearing, and ordinary command execution during
  that TUI session.
- TUI state may keep unmasked response text in memory only as needed for the
  current foreground Response display and copy behavior.
- Unmasked response text must never be passed to normal log writers.
- Unmasked response text must never be written to config, state, history,
  session files, or saved Response files.
- Saved masked log viewing remains masked even if TUI session output masking is
  off.
- OQ-013 completion requires documentation updates, masked/default behavior,
  `atctl tui --no-mask`, typed acknowledgement tests, cancel tests,
  state-indicator tests, session-persistence tests, copy-behavior tests, saved
  Response masking tests, saved-log masking tests, and raw diagnostic export
  separation tests.

Rationale:

- The foreground TUI session is the interactive product surface where the user
  may need to inspect decoded SMS bodies, TCP response data, or other sensitive
  diagnostic values without repeating an execution for each response.
- Normal files remain masked to avoid sensitive data disclosure through logs,
  saved responses, history, or session files.
- The output-masking state is visible without depending on color alone.

Resolved by:

- User approval of the session output masking design on 2026-06-23.

## OQ-012: TUI Risk Visual Differentiation

Status: resolved

Scope:

```text
System: atctl tui
Area: risk labels and risk state visibility
```

Decision:

- Risk display must always include text labels and non-color cues:
  - `safe` -> `[safe]`
  - `sensitive` -> `[sensitive] MASKED`
  - `write` -> `[write] CONFIRM`
  - `persistent` -> `[persistent] PERSISTS`
  - `dangerous` -> `[dangerous] DANGER`
  - `unknown` -> `[unknown] REVIEW`
- The approved dark and light palettes are:
  - dark background `#263238`, base text `#ECEFF1`
  - dark focus/status/safe `#4DD0E1`
  - dark selection/write `#FFD54F` / `#FFD166`
  - dark sensitive `#D6B3FF`
  - dark persistent `#FFB86C`
  - dark dangerous `#FF6B6B`
  - dark unknown `#B0BEC5`
  - light background `#FAFAFA`, base text `#263238`
  - light focus/status/safe `#007C89`
  - light selection/write `#7A5A00`
  - light sensitive `#6B3FA0`
  - light persistent `#9A4D00`
  - light dangerous `#B00020`
  - light unknown `#4B5563`
- Risk styling applies to Commands, Status, Confirmation, and Response areas.
- Selected rows must not erase risk-specific styling.
- TUI theme selection is:
  - default: dark
  - `--theme dark`
  - `--theme light`
  - `--theme no-color`
  - `NO_COLOR` without explicit `--theme` uses no-color.
- OQ-012 completion requires documentation updates, semantic role
  implementation, theme-selection tests, risk-role tests, render-buffer checks,
  and a user confirmation point for dark, light, and no-color output.

Rationale:

- The design preserves the user-approved cyan/yellow direction while defining
  objective dark and light foreground/background pairs.
- Color is not the only risk indicator.
- `NO_COLOR` remains usable because labels, keywords, markers, bold emphasis,
  and pane/dialog context remain available.

Resolved by:

- User approval of OQ-012-1 through OQ-012-5.

## OQ-001: GitHub Owner

Status: resolved

Decision:

```text
uchimanajet7
```

Needed for:

- Repository URL
- README links
- Homebrew formula `homepage`
- Release artifact URLs

Resolved repository URL:

```text
https://github.com/uchimanajet7/atctl
```

## OQ-002: License

Status: resolved

Decision:

```text
MIT
```

Rationale:

- MIT is a common permissive open source license.
- It allows use, copy, modification, distribution, sublicensing, and sale when
  the copyright notice and license text are preserved.
- It includes warranty and liability disclaimers.

Project-owner tradeoff:

- Other parties can reuse, fork, modify, redistribute, sell, or include the
  project in closed-source products, as long as they preserve the required
  notice.
- MIT does not require downstream users to publish their modifications.
- If stronger reciprocity or patent-specific terms become necessary, the
  license decision must be revisited before release.

## OQ-003: Homebrew Tap

Status: resolved

Decision:

```text
Homebrew tap repository: https://github.com/uchimanajet7/homebrew-atctl
User-facing tap name: uchimanajet7/atctl
```

Planned install flow:

```sh
brew install uchimanajet7/atctl/atctl
```

Rationale:

- Homebrew's one-argument GitHub tap form maps `brew tap <user>/<repo>` to
  `https://github.com/<user>/homebrew-<repo>`.
- Therefore `brew tap uchimanajet7/atctl` uses the GitHub repository
  `uchimanajet7/homebrew-atctl`.
- The fully qualified formula name `uchimanajet7/atctl/atctl` selects the
  `atctl` formula from that tap without requiring users to type a separate tap
  step first.
- The source repository remains `uchimanajet7/atctl`; the tap repository stores
  Homebrew formula and tap metadata.

References:

- https://docs.brew.sh/Taps
- https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap

## OQ-004: Direct `atctl send` Safety Policy

Status: resolved

Decision:

- Plain read/test commands may run without confirmation and may print plain
  output.
- Read/test commands that expose sensitive identifiers or credentials may run
  without confirmation, but output and logs must be masked by default.
- Unknown read/test commands may run without confirmation, but must be treated
  as sensitive by default.
- Write, change, delete, persistent, dangerous, and non-read/test unknown
  commands must require explicit confirmation.
- Automation may bypass an interactive confirmation only when both `--yes` and
  `--risk-ack <risk>` are present.
- `--yes` alone must not bypass confirmation for direct `atctl send`.
- `--risk-ack <risk>` must match the implementation's classified risk level.
  If it does not match, the command must fail before sending anything to the
  modem.
- Direct-send completion requires a maintained risk-pattern table for known
  dangerous and persistent command families.

Examples:

```sh
# Allowed without confirmation. Output is still subject to default masking.
atctl send AT
atctl send ATI
atctl send AT+CIMI
atctl send AT+CGDCONT?

# Not allowed. The risk being accepted is not explicit.
atctl send 'AT+CFUN=0' --yes

# Allowed for automation if the classifier also resolves the command as dangerous.
atctl send 'AT+CFUN=0' --yes --risk-ack dangerous
```

Rationale:

- `--yes` means "do not prompt"; it does not state what risk was reviewed.
- `--risk-ack <risk>` makes non-interactive execution equivalent to the user
  understanding and explicitly accepting the classified risk.
- A mismatch between the command classifier and `--risk-ack` prevents stale
  scripts from silently running a command whose risk changed.

## OQ-005: Source Repository Release Artifacts

Status: resolved

Scope:

```text
Repository: uchimanajet7/atctl
System: GitHub Actions release workflow and GitHub Releases assets
```

This question is only about release artifacts produced from the source
repository. By itself, it does not decide Homebrew formula behavior, Homebrew
bottles, or tap repository layout.

Decision:

- Tag releases must use GitHub Actions to build a prebuilt Apple Silicon macOS
  binary.
- Tag releases must publish the binary and a checksum to GitHub Releases.
- Tag releases must validate that the pushed tag version matches `Cargo.toml`
  `package.version` before release asset publication.
- GitHub Release notes must come from the matching released-version section in
  `CHANGELOG.md`, and that section must include a release date and non-empty
  user-facing release-note content.
- GitHub Release binary creation by itself does not decide Homebrew formula
  behavior, Homebrew bottles, or tap repository behavior.

Initial validation remains macOS Apple Silicon.

References:

- https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
- https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository
- https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes
- https://docs.github.com/en/rest/releases/assets
- https://cli.github.com/manual/gh_release_create
- https://keepachangelog.com/en/1.1.0/

## OQ-006: Source Repository Release Asset Naming

Status: resolved

Scope:

```text
Repository: uchimanajet7/atctl
System: GitHub Releases asset names, archive format, checksum format
```

Decision:

- For release version `{VERSION}`, where `{VERSION}` is the semantic version
  without the leading `v`, the Apple Silicon macOS archive asset must be:

  ```text
  atctl-v{VERSION}-aarch64-apple-darwin.tar.gz
  ```

- The checksum asset for that archive must be:

  ```text
  atctl-v{VERSION}-aarch64-apple-darwin.tar.gz.sha256
  ```

- `aarch64-apple-darwin` identifies the Rust/macOS Apple Silicon target.
- This naming is a common Rust CLI release-asset convention, not a formal
  GitHub or Rust standard. The exact use of a leading `v` in the asset filename
  varies by project.

References:

- https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html
- https://github.com/sharkdp/bat/releases
- https://github.com/BurntSushi/ripgrep/releases
- https://github.com/rust-lang/rust-bindgen/releases

## OQ-007: Source Repository Checksum Content and Provenance

Status: resolved

Scope:

```text
Repository: uchimanajet7/atctl
System: GitHub Releases checksum file content, optional provenance metadata
```

Decision:

- Tag releases publish one checksum file per archive.
- The checksum file must use sha256sum-compatible content:

  ```text
  <sha256 hex>  atctl-v{VERSION}-aarch64-apple-darwin.tar.gz
  ```

- Tag releases do not publish an aggregate checksum manifest unless separately
  approved.
- Tag releases do not publish provenance, attestation, or SBOM metadata unless
  separately approved.
- Provenance, attestation, or SBOM metadata must be decided separately before
  being promised or implemented.

## OQ-008: Homebrew Distribution Strategy

Status: resolved

Scope:

```text
Repository: uchimanajet7/homebrew-atctl
System: Homebrew Formula/atctl.rb, tap CI, bottle metadata
```

This question is only about how Homebrew installs `atctl` from the tap. It does
not decide whether the source repository creates GitHub Release binaries.

Decision:

- Normal end-user installation must use the Homebrew tap and formula:

  ```sh
  brew install uchimanajet7/atctl/atctl
  ```

  The equivalent tapped form is `brew tap uchimanajet7/atctl` followed by
  `brew install atctl`.

- The preferred normal Homebrew state is a bottle-backed formula for each
  packaged platform target.
- The formula must keep source-build support as a fallback for missing bottles,
  disabled bottle use, and maintainer source verification.
- The source-build fallback must build from the `uchimanajet7/atctl` source
  repository release archive.
- The formula must declare `libusb` as a runtime dependency.
- The formula must declare Rust and `pkgconf` as source-build dependencies, not
  as runtime dependencies for bottled installs.
- The Homebrew tap must not install the prebuilt GitHub Releases `.tar.gz`
  artifact as the normal Homebrew path.
- Bottle publishing is tap repository release automation and must be reviewed as
  such, not accepted as incidental generated workflow output.
- The formula update workflow from `uchimanajet7/atctl` releases to
  `uchimanajet7/homebrew-atctl` must be implemented as tap repository work.

## OQ-009: Signing and Notarization

Status: resolved

Decision:

- The normal end-user installation path is Homebrew.
- GitHub Releases prebuilt `.tar.gz` artifacts are release/manual artifacts, not
  the normal end-user install path.
- Developer ID signing and Apple notarization are not required for the normal
  Homebrew install path.
- Developer ID signing and Apple notarization are not required before publishing
  GitHub Releases `.tar.gz` artifacts that remain release/manual artifacts.
- The project must not present direct GitHub Releases download as the normal
  end-user install path until signing/notarization and related macOS download
  behavior are decided.
- If direct GitHub Releases download is promoted to a normal end-user install
  path, Developer ID signing, Apple notarization, Gatekeeper behavior,
  quarantine warnings, credential handling, and CI secret handling must be
  decided before that promotion.

Rationale:

- Homebrew formula distribution is the normal path for this CLI/TUI executable;
  it is not a macOS GUI app or Cask-first product.
- Homebrew bottles preserve the same fully qualified formula install flow while
  avoiding a Rust build on the user's Mac when a matching bottle is available.
- Source-build support remains useful as a fallback and verification path.
- GitHub Releases artifacts are separate release/manual artifacts.
- Unsigned or unnotarized direct-download macOS binaries may trigger Gatekeeper
  or quarantine warnings depending on how the user downloads and runs them.
- Promoting direct download to the normal install route would add Apple
  Developer credentials, certificate handling, CI secret management, and Apple
  notary service dependency to the release process.

References:

- https://docs.brew.sh/Formula-Cookbook
- https://docs.brew.sh/Bottles
- https://docs.brew.sh/Cask-Cookbook
- https://developer.apple.com/developer-id/
- https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
- https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution

## OQ-010: PTY Bridge Implementation Approach

Status: resolved

Decision:

- Phase 4 PTY bridge initial implementation must use `portable-pty` as the
  first implementation approach.
- Platform-specific PTY implementation is not the default direction.
- The `atctl` codebase must keep a thin PTY bridge boundary so that future Linux
  support can be investigated without rewriting unrelated command, transport,
  masking, logging, or safety logic.
- This decision does not promise Linux support for the initial release.
- If `portable-pty` cannot satisfy required slave path, symlink, `screen`/`cu`,
  cleanup, signal handling, or terminal behavior requirements, the limitation
  must be documented in the specification and a platform-specific fallback must
  receive explicit approval before implementation.

References:

- https://docs.rs/portable-pty/latest/portable_pty/

## OQ-011: Exact Onyx / EG25-G Endpoint Mapping

Status: resolved

Decision:

- The specification must not define a fixed interface, bulk IN endpoint, or
  bulk OUT endpoint value as the normative Onyx / EG25-G implementation
  mapping.
- The implementation must use USB descriptor inspection, endpoint candidate
  enumeration, AT probe, and runtime selection.
- The implementation must support manual override with `--interface`,
  `--bulk-in`, and `--bulk-out`.
- `atctl inspect` must show candidate endpoint pairs and explain whether each
  candidate is selected by descriptor shape, AT probe result, or manual
  override.
- Endpoint values observed on real hardware may be recorded only as
  evidence/observation, not as required implementation constants.
- Observed endpoint evidence should include enough context to be useful: device
  identity, VID/PID, OS, date, `atctl inspect` output, successful `AT` probe,
  and selected interface/endpoint pair.

Rationale:

- Interface and endpoint numbers can vary by USB configuration, alternate
  setting, firmware, host behavior, or environment.
- A fixed endpoint mapping would make the implementation fragile and would make
  one observed environment look like a universal requirement.
