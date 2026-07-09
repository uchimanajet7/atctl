# Implementation Status

This document tracks implementation progress for `atctl`. It is operational
state, not the normative specification. Continue implementation by reading this
file together with `docs/SPEC.md`.

## Current Specification

```text
Specification: docs/SPEC.md
Specification version: 0.4.109
Open decisions: none for the currently specified Sequence design; OQ-018 is resolved as out of scope, OQ-021 raw log diagnostic export is resolved, OQ-022 TUI shortcut reduction is resolved, and OQ-023 Sequence product design is resolved
```

## Current Phase

```text
Phase: 5 - Packaging and Documentation
Checkpoint: 13 - final release and Homebrew workflow
Status: source repository release workflow is version-guarded and changelog-backed after Checkpoint 12.6 application features were implemented, agent-verified, and user approved; source release and Homebrew publication are documented as separate operations; Homebrew tap workflow remains pending
Next checkpoint: Checkpoint 13 Homebrew tap formula, tap-side manual Formula update PR workflow, source-build fallback, bottle automation, and tap CI
```

Rust project readiness decisions on 2026-07-04:

- MIT `LICENSE` file: approved and added with copyright holder
  `uchimanajet7`.
- `Cargo.toml` package metadata: approved as general Rust/Cargo publication
  metadata with `license = "MIT"` and no separate `license-file` for the
  standard MIT license.
- `docs/SPEC.md` license section: approved and synchronized with the resolved
  MIT license decision, the root `LICENSE` file, and `Cargo.toml`
  `license = "MIT"` metadata.
- `README.md` license section: approved and added as a short user-facing
  pointer to the root `LICENSE` file.

Rust package source-file scope decision on 2026-07-05:

- `Cargo.toml` now uses an explicit Cargo package `include` whitelist.
- The Cargo source package is treated as Rust package metadata and `.crate`
  output, not as the normal end-user install path.
- The normal end-user install path remains Homebrew.
- Included Cargo package files are limited to source files, repository-managed
  preset and Sequence examples required by current source/tests, `README.md`,
  `CHANGELOG.md`, and `LICENSE`.
- Repository docs remain GitHub/source-repository documentation. They are not
  included in the Cargo source package only because `README.md` links to them.
- `README.md` documentation links now point to GitHub URLs so they remain usable
  when the README is rendered outside the source repository, such as on
  crates.io or docs.rs.

Source repository release workflow decision on 2026-07-05:

- `.github/workflows/release.yml` was added for source repository tag releases
  matching `v*.*.*`.
- The workflow uses the fixed `macos-26` GitHub-hosted runner label for the
  standard arm64 macOS release environment instead of the moving
  `macos-latest` label.
- The workflow builds only the approved Apple Silicon macOS target
  `aarch64-apple-darwin`.
- The workflow verifies release source with `cargo fmt --check`,
  `cargo check --all-targets --all-features --locked`,
  `cargo test --all-features --locked`, and
  `cargo clippy --all-targets --all-features --locked -- -D warnings`.
- The workflow creates `atctl-v{VERSION}-aarch64-apple-darwin.tar.gz` with
  the `atctl` executable at the archive top level, generates the matching
  `.sha256` file, validates that the pushed tag version matches `Cargo.toml`
  `package.version`, extracts the matching released-version section from
  `CHANGELOG.md`, and creates a GitHub Release with
  `gh release create --verify-tag --notes-file`.
- The matching `CHANGELOG.md` released-version section must include a
  `YYYY-MM-DD` release date and non-empty release-note content. The workflow
  fails before release creation when that section is missing or empty.
- The workflow is source-repository only. It does not update a Homebrew tap,
  create or update Homebrew Formula pull requests, trigger Homebrew
  publication automatically, publish Homebrew bottles, create tags, add
  SBOM/provenance/attestation output, sign or notarize artifacts, or add other
  platform release archives.

Dependency and CI maintenance decision on 2026-07-09:

- `.github/dependabot.yml` is the source repository update detector for Cargo
  dependencies and GitHub Actions references.
- `.github/workflows/dependency-review.yml` runs on dependency-relevant pull
  requests and fails when a dependency change introduces a vulnerability at
  moderate severity or higher.
- `.github/workflows/maintenance.yml` runs on dependency-relevant pull
  requests, on manual dispatch, and weekly at a non-hour-start cron minute. It
  checks Cargo lockfile consistency, RustSec advisories, direct dependency
  drift, duplicate dependency tree signal, and GitHub Actions workflow syntax.
- Source-repository workflows pin remote GitHub Actions by full-length commit
  SHA and keep same-line version comments for Dependabot update context.
- `scripts/maintenance/check-deps.sh` is the shared local/CI maintenance check.
- `scripts/maintenance/update-rust-deps.sh` refreshes Cargo dependency locks
  and then runs maintenance checks plus the normal Rust verification gate.

Latest agent verification for dependency and CI maintenance on 2026-07-09:

- `bash -n scripts/maintenance/check-deps.sh` passed.
- `bash -n scripts/maintenance/update-rust-deps.sh` passed.
- `git diff --check` passed.
- `cargo metadata --locked --format-version 1` passed.
- `scripts/maintenance/check-deps.sh` passed after installing the required
  local maintenance tools, and passed again after GitHub Actions SHA pinning.
  It loaded 1159 RustSec advisories, found no Cargo vulnerabilities, reported
  direct dependencies up to date, printed duplicate dependency tree review
  signal, and completed `actionlint`.
- `scripts/maintenance/update-rust-deps.sh --help` passed.
- `actionlint -color` passed after GitHub Actions SHA pinning.
- Residual search found no remaining workflow `uses:` references pinned only to
  major tags such as `@v7` or `@v5`.
- `cargo fmt --check` passed.
- `cargo check --all-targets --all-features --locked` passed.
- `cargo test --all-features --locked` passed with 267 tests.
- `cargo clippy --all-targets --all-features --locked -- -D warnings` passed.
- `cargo package --list --allow-dirty` passed and did not list `.github/`,
  `scripts/`, `docs/`, or `_local/` files in the Cargo source package.

Homebrew tap install-name decision on 2026-07-05:

- The Homebrew tap repository remains `uchimanajet7/homebrew-atctl`.
- The user-facing tap name remains `uchimanajet7/atctl`.
- The normal one-line install command is
  `brew install uchimanajet7/atctl/atctl`, which selects the `atctl` formula
  from the `uchimanajet7/atctl` tap.
- The equivalent tapped form remains `brew tap uchimanajet7/atctl` followed by
  `brew install atctl`.

Homebrew publication separation decision on 2026-07-05:

- Source repository GitHub Release creation and Homebrew publication are
  separate release operations.
- A source repository release build may be run without publishing or updating
  Homebrew material.
- Homebrew Formula update pull-request creation belongs in
  `uchimanajet7/homebrew-atctl`, not in the source repository release workflow.
- The intended automation path is a manually triggered tap workflow, such as
  `.github/workflows/update-formula-pr.yml`, using `workflow_dispatch` with the
  chosen source release tag as input.
- That tap workflow should update `Formula/atctl.rb` and create or update a
  pull request in the tap repository only when an operator explicitly runs it.

Current add-on loading boundary: default startup loads only product-provided
Presets and Sequences. File preset and Sequence add-ons require explicit
`--preset-file`, `--preset-dir`, `--sequence-file`, or `--sequence-dir` flags
for the current invocation. CLI list output includes the external definition
source path for review, and CLI run surfaces show external definition
source/path notice before USB access even when non-interactive risk
acknowledgement is supplied.

Latest agent verification for this boundary on 2026-06-28:

- `cargo fmt --check`, `cargo check --all-targets --all-features --locked`,
  `cargo test --all-features --locked`, and
  `cargo clippy --all-targets --all-features --locked -- -D warnings` passed.
- `cargo run -- preset list --preset-dir examples/presets` showed
  `source-path`, `-` for product rows, and TOML paths for external rows.
- `cargo run -- sequence list --sequence-dir examples/sequences` showed
  `source-path`, `-` for product rows, and TOML paths for external rows.
- External `preset run` and `sequence run` with `--yes --risk-ack write` showed
  source/path review notice before failing at the fake USB selector.

Latest agent verification for TUI candidate refresh action update on
2026-07-04:

- Updated `README.md`, `docs/SPEC.md` to version `0.4.103`,
  `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, this file,
  `src/tui/mod.rs`, and `src/tui/tests.rs`.
- Candidate-backed Sequence inputs keep explicit refresh/load actions visible
  after same-session candidates are loaded, covering product-known
  `sms-message` and `pdp-context` candidates for built-in, repository-managed,
  and user-authored add-on Sequences without hidden modem I/O on modal open.
- `cargo fmt --check`, `cargo test --all-features --locked tui::tests`,
  `cargo check --all-targets --all-features --locked`,
  `cargo test --all-features --locked`, and
  `cargo clippy --all-targets --all-features --locked -- -D warnings` passed
  for the updated TUI behavior.

## Checkpoints

| Checkpoint | User confirmation point | Status |
|---|---|---|
| 1 | Rust scaffold, dependency design, CLI skeleton, parser/masking/risk basics, mock transport tests | complete - user approved |
| 2 | USB descriptor inspection and endpoint candidate reporting before real-device probing | complete - user approved |
| 3 | `atctl send AT` mock tests pass; ready for user-run real Onyx check | complete - user approved |
| 4 | safety policy, masking, confirmation, and `--risk-ack` behavior implemented | complete - user approved |
| 5 | initial built-in presets, user config loading, and single-file user preset loading before TUI | complete - user approved; preset model superseded by Checkpoint 11.5 |
| 6 | command history and masked session logging before TUI | complete - user approved |
| 7 | TUI app skeleton and core panes | complete - user approved |
| 8 | TUI visual accessibility and theme foundation before command execution | complete - user approved |
| 9 | TUI command execution, confirmation dialogs, and terminal restoration hardening before PTY | complete - user approved |
| 10 | TUI risk styling and initial masked/raw reveal before PTY, later superseded by session output masking | complete - user approved |
| 11 | TUI masked log viewer before PTY | complete - user approved |
| 11.5 | TUI compact status, preset set/loading cleanup, repository-managed file preset examples, effective preset risk, ad-hoc AT input, Response copy, timeout control, and explicit device selection before PTY | complete - user approved |
| 12 | PTY bridge design and first implementation | complete - user approved |
| 12.5 | built-in modem functionality presets and Quectel power-down file preset before release | complete |
| 12.6 | Sequences for SMS send/read/decode/reply-by-index, active review, and Quectel/SORACOM TCP example data-send evidence before release | complete - user approved |
| 13 | final release and Homebrew workflow after application features are complete | source repository release workflow added; Homebrew tap workflow pending |

## Completed

- Specification and supporting documents created.
- Initial entries OQ-001 through OQ-011 in `docs/OPEN-QUESTIONS.md` resolved.
- Implementation status tracking file created.
- Rust package scaffold created.
- Phase 1 module boundaries created.
- Non-hardware core behavior implemented:
  - CLI parser skeleton.
  - AT response parser basics.
  - sensitive value masking basics.
  - direct command risk classification basics.
  - mock transport tests.
- Checkpoint 1 user review completed and approved.
- USB descriptor inspection through `rusb` implemented.
- Endpoint candidate reporting by descriptor shape only implemented.
- USB serial descriptor display is masked in user-facing `devices` output.
- Checkpoint 2 user review completed and approved.
- `atctl send` core workflow implemented through the transport trait.
- USB transport open, interface claim, endpoint auto-detection by safe `AT`
  probe, bulk write, bulk read, and close/release path implemented.
- Mock transport tests cover `send AT`, masking, JSON masking, AT error
  handling, and raw diagnostic export behavior.
- Checkpoint 3 user review completed and approved.
- Real SORACOM Onyx / Quectel EG25-G verification succeeded for:
  - `atctl devices --vid 0x2c7c --pid 0x0125`
  - `atctl inspect --vid 0x2c7c --pid 0x0125`
  - `atctl send AT --vid 0x2c7c --pid 0x0125`
  - `atctl send ATI --vid 0x2c7c --pid 0x0125`
- Direct-send confirmation planning implemented.
- Interactive confirmation for write, persistent, dangerous, and unknown
  commands requires typing the classified risk level.
- Non-interactive automation bypass requires matching `--yes --risk-ack <risk>`.
- `--risk-ack` mismatch is rejected before USB access even for safe commands.
- Text and JSON send output preserve default masking.
- Checkpoint 4 user review completed and approved.
- User-run Checkpoint 4 verification confirmed:
  - `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings`
  - `--risk-ack` mismatch rejection before USB access
  - dangerous `--yes` without matching `--risk-ack` rejection before USB access
  - `--yes --risk-ack dangerous` passing safety validation and stopping at fake VID/PID
  - interactive `ATE0` confirmation accepting `write` and rejecting `abc`
- Release and Homebrew workflow notes were documented in `docs/PACKAGING.md`
  but moved out of the active checkpoint sequence because packaging belongs to
  the final phase after application feature implementation.
- Initial built-in presets from `docs/SPEC.md` implemented.
- `atctl preset list` implemented with name, risk, categories, and command output.
- Explicit file preset loading implemented. Earlier XDG user-preset auto-load
  behavior is superseded by the current explicit add-on loading boundary.
- Config file parsing and XDG config path discovery implemented.
- Device defaults from config are applied without overwriting CLI USB options.
- `atctl preset run <NAME>` uses the same transport, masking, and confirmation
  path as direct `atctl send`, using explicit preset risk levels.
- Checkpoint 5 user review completed and approved.
- User-run Checkpoint 5 Onyx verification confirmed:
  - `cargo run -- preset run modem-response --vid 0x2c7c --pid 0x0125`
  - response was `AT`, `OK`
- Command history JSONL writing implemented separately from session log writing.
- Masked session log writing implemented.
- Log files are written with user-only file permissions where supported.
- `atctl logs list` implemented.
- Raw diagnostic export uses explicit user-selected output paths and separate
  `raw-log` acknowledgement; it is still not silently enabled.
- Checkpoint 6 user review completed and approved.
- User-run Checkpoint 6 Onyx logging verification confirmed:
  - `XDG_STATE_HOME=<temp> cargo run -- preset run modem-response --vid 0x2c7c --pid 0x0125`
  - `XDG_STATE_HOME=<temp> cargo run -- logs list`
  - `find <temp>/atctl -type f -print`
  - response was `AT`, `OK`
  - `logs list` showed one history file and one session log
  - both generated files used user-only file permissions
- `ratatui` and `crossterm` dependencies added for TUI implementation.
- `atctl tui` skeleton implemented with Devices, Categories, Commands,
  Response, and History / Status panes.
- TUI key handling implemented for `q`, `?`, left/right focus movement,
  up/down selection, `Enter` preview, `c`, `l`, `m`, `e`, `r`, and `s`
  placeholder status handling.
- TUI command execution is intentionally disabled in Checkpoint 7.
- TUI terminal raw mode, alternate screen entry, and drop-based restoration are
  implemented.
- TUI styling is back on the user-approved colored baseline: focused panes and
  the status accent use cyan, and selected command/category rows use yellow.
- Light/dark theme support remains a separate design decision. Do not change
  the TUI palette or emphasis again without explicit approval for the concrete
  theme approach and colors.
- TUI color, theme, and accessibility requirements are now specified in
  `docs/SPEC.md` section 16.1. The current cyan/yellow styling is not verified
  or documented as complete light/dark theme support.
- Checkpoint 7 user review completed and approved.
- TUI visual accessibility and theme foundation implemented.
- TUI style choices are represented through semantic roles in
  `src/tui/theme.rs`.
- The default color-enabled theme preserves the user-approved cyan focus/status
  and yellow selection accents.
- `NO_COLOR` disables software-added foreground colors while preserving
  non-color affordances such as selection markers, text labels, borders, and
  bold focus/selection emphasis.
- Risk labels remain text-visible as `[safe]`, `[sensitive]`, `[write]`, and
  related risk values.
- Light/dark theme support remains incomplete; no separate light/dark palettes
  or contrast-conformance claims have been added.
- Checkpoint 8 user review completed and approved.
- Checkpoint 9 TUI command execution implemented through the shared CLI send
  pipeline.
- TUI safe and sensitive commands execute from `Enter` without an extra dialog,
  using the existing USB transport, masking, logging, and risk policy.
- TUI confirmation dialogs are implemented for confirmation-required commands.
  The dialog shows command name, command string, risk, and expected effect.
- TUI confirmation requires typing the exact displayed risk level before the
  command is sent.
- At Checkpoint 9, dangerous commands were hidden because no explicit
  dangerous-preset product behavior existed yet. Checkpoint 12.5 supersedes this
  for explicit modem functionality presets with typed dangerous confirmation.
- TUI runtime keeps drop-based terminal restoration and was checked through a
  PTY start, confirmation rejection, and quit path.
- Checkpoint 9 user-review feedback addressed:
  - execution now shows the preset name, AT command string, risk, and expected
    effect before USB transport execution starts
  - completion output keeps command context above the response
  - Status and History are separate panes
  - Status uses the unused space under Devices and shows selected or active
    command context
  - This Status layout was later superseded by Checkpoint 11.5 requirements for
    a compact non-interactive Status area.
  - Recent logs are compacted to file names in the TUI
  - Response rendering clears its pane before drawing, so `c` removes stale
    characters from prior responses
- Checkpoint 9 user review completed and approved.
- Checkpoint 10 implementation completed and was approved by the user:
  - `atctl tui --theme dark`, `--theme light`, and `--theme no-color` are
    supported.
  - Default TUI theme is dark.
  - `NO_COLOR` without explicit `--theme` uses no-color mode.
  - Approved dark/light risk palettes are implemented through semantic style
    roles.
  - Risk labels include non-color cues such as `[sensitive] MASKED` and
    `[write] CONFIRM`.
  - Commands, Status, Confirmation, and Response areas show risk context.
  - Selected command rows preserve risk-specific token styling.
  - Historical Checkpoint 10 raw reveal was replaced by session output masking
    in the current specification.
  - Status owns command state and metadata; Response prioritizes the actual
    current response body.
  - Response rendering normalizes modem line terminators and removes
    terminal-affecting control sequences before drawing, so unmasked display cannot
    overwrite pane borders or prior rendered text.
  - TUI session output masking is on by default, can start off with
    `atctl tui --no-mask`, and can be disabled in-app after exact `unmask`
    acknowledgement.
- History, saved responses, and session logs remain masked; raw diagnostic
  export remains a separate acknowledged workflow.
- `+QCCID:` masked output treats trailing `F` padding as part of the sensitive
  ICCID response value and hides it; unmasked foreground display and
  acknowledged raw diagnostic export may still show the modem-returned `F`.
- Checkpoint 10 user-review ICCID padding mask fix was confirmed by the user.
- Checkpoint 10 completed and was approved by the user.
- Checkpoint 11 implementation completed and was approved by the user:
  - History pane keeps a selected log row.
  - `l` focuses History.
  - `Up` and `Down` move the selected log while History is focused.
  - `Enter` on History reads the selected existing history/session log file.
  - Response pane shows the selected log's masked content.
  - Log viewing re-applies masking before display.
  - Log viewing does not create log files and does not expose raw response text.
  - Status shows that a masked log is being viewed and identifies the log type
    and selected log label.
  - Response pane content can be scrolled with `Up`, `Down`, `PageUp`, and
    `PageDown` when Response is focused.
  - Response remains the AT command response pane; opening a log puts it into a
    temporary masked log-view mode only.
  - Masked log-view mode shows line numbers and a visible line range in the
    Response pane itself.
  - Opening a selected log moves focus to Response so the opened content can be
    read and scrolled immediately.
- Normal focus cycling is limited to interactive panes in this order:
  Categories, Commands, Response, History.
- Checkpoint 11 user review completed and approved.
- Checkpoint 11.5 requirements were documented before implementation:
  - Status must become compact and non-interactive instead of absorbing large
    unused space.
  - Standard workflow built-in presets must cover practical modem validation
    workflows, not only passive diagnostics.
  - Quectel and SORACOM commands must move out of built-in presets and into
    repository-managed TOML file preset examples.
  - Repository-managed `examples/presets/quectel.toml` and
    `examples/presets/soracom.toml` must be created and verified in the same
    checkpoint that implements multi-file preset loading.
  - File preset add-ons must load from explicit per-invocation file or
    directory flags.
  - CLI preset listings must show preset set labels, while TUI must
    distinguish file presets from built-in presets through non-selectable source
    group headers and `Source: <title>` detail only when the distinction is
    relevant.
  - Preset execution must use effective risk derived from declared risk and
    command classification.
  - TUI must provide an ad-hoc AT command input route using the same safety
    policy as preset execution.
  - SMS send and other prompt-required multi-step commands remain separate from
    one-shot ad-hoc execution until specifically designed.
- Checkpoint 11.5 Response copy requirement was added after explicit user
  approval:
  - The Response action menu copy action copies the current Response body without pane
    borders or surrounding UI.
  - Current command responses copy the executed AT command and displayed
    response body without duplicating a modem echo line.
  - Normal copy uses masked visible content. When TUI session output masking is
    off, copy follows the unmasked visible Response display.
  - Masked log view copies only the displayed masked log body without line
    number UI or raw values.
  - Initial clipboard integration may use OSC 52 and must not read the
    clipboard, shell out to `pbcopy`, or add a clipboard dependency without a
    separate approved design.
  - After a Response copy action, the Response action menu shows nearby
    copy-request feedback. The TUI reports that the terminal clipboard request
    was sent, not that clipboard contents were independently verified.
- Checkpoint 11.5 documentation update was reviewed and approved by the user.
- Checkpoint 11.5 baseline implementation was completed before the explicit
  device selection gate requirement was added:
  - Status is compact and non-interactive, placed under Devices in the left
    column rather than rendered as a full-width band.
  - Built-in presets are reorganized around standard workflow checks for basic
    control, modem identity, SIM, registration, signal, PDP/APN readiness,
    failure diagnostics, SMS readiness, and modem functionality.
  - Quectel and SORACOM-specific commands are removed from built-in presets.
  - Repository-managed file preset examples were created at
    `examples/presets/quectel.toml` and `examples/presets/soracom.toml`.
  - `atctl preset list`, `atctl preset run <NAME>`, and `atctl tui`
    accept explicit per-invocation file preset locations with
    `--preset-file <FILE>` and `--preset-dir <DIR>`.
  - Default startup loads only product presets. Explicit file preset location
    flags add file presets for that invocation while keeping product presets
    loaded.
  - Duplicate preset names across built-in presets and loaded file presets fail with an
    actionable error.
  - Preset list output shows preset set, declared risk, effective risk,
    categories, and command.
  - Effective risk is computed from declared risk and command classification,
    so TOML cannot downgrade classifier enforcement.
  - TUI Commands and Status keep the default built-in-only view free of preset
    set labels. When file presets are visible, Commands uses non-selectable
    source group headers and Status/confirmation use `Source: <title>` detail
    labels without relying on color.
  - TUI Controls opens one-shot ad-hoc AT input using the same classification,
    confirmation, masking, logging, and transport path as preset execution.
  - Prompt-required SMS/multi-step commands are rejected by one-shot ad-hoc
    input until a separate multi-step design is approved.
  - TUI Controls copies the current Response body via OSC 52 without pane
    borders, pane titles, line-number UI, Status, or History content.
  - Response copy follows the current visible masking state: normal copies
    masked visible text, and unmasked text is copyable only while TUI session
    output masking is off.
  - SORACOM's advanced data-send/receive troubleshooting AT checkpoints were
    rechecked against built-in presets on 2026-06-18.
  - `available-operators` (`AT+COPS=?`) was added as a safe core network preset
    for available carrier / RAT listing. Because this can take longer than
    ordinary reads, the preset now carries a 180-second timeout hint; CLI users
    can still override it with `--timeout` when needed.
  - `packet-attach` (`AT+CGATT?`) was added as a safe core network/PDP preset for
    PS attach status.
  - The risk classifier now treats `AT+COPS=?` and `AT+CGATT?` as known safe
    standard workflow read/test commands.
- Checkpoint 11.5 user-review correction requirements were documented before
  implementation:
  - User AT command execution timeout default must be 30 seconds.
  - Endpoint auto-detection `AT` probe timeout must remain short and separate
    from the user command timeout.
  - TUI command execution must keep the UI redrawing while the command is in
    progress.
  - TUI Status must show elapsed time, timeout, remaining time, and a
    timeout-budget progress indicator during execution.
  - Status must move from a full-width band to the area under Devices.
  - The lower area must return to Response and Logs.
  - The saved history/session list pane must be titled `Logs`.
- Checkpoint 11.5 user-review correction implementation was completed before
  the explicit device selection gate requirement was added:
  - Default user AT command timeout is 30 seconds for direct send, preset run,
    and TUI execution.
  - Endpoint auto-detection probe timeout remains a separate 3-second timeout.
  - TUI execution runs through a worker thread so the UI can continue
    redrawing while the command is in progress.
  - TUI Status shows running command context, elapsed time, timeout, remaining
    time, and a timeout-budget progress indicator during execution.
  - New command execution, ad-hoc input, output masking, response clear, response
    copy, and quit actions are blocked while a command is running.
  - Status is placed under Devices in the left column.
  - The lower area contains Response and Logs.
  - The saved history/session list pane title is `Logs`.
- Checkpoint 11.5 follow-up implementation for Devices / Status / timeout was
  completed before the explicit device selection gate requirement was added:
  - Devices pane lists visible matching USB targets instead of only showing a
    static placeholder.
  - `d` cycles the selected visible USB device, and TUI execution passes the
    selected bus/address to the transport. This is superseded by the explicit
    selection gate requirement, where `d` focuses Devices selection and
    `Enter` confirms the highlighted candidate.
  - Status uses key-value lines instead of packing unrelated values into
    pipe-delimited single lines.
  - Presets support optional `timeout_secs`.
  - The built-in `available-operators` preset declares `timeout_secs = 180`
    because `AT+COPS=?` is a long-running operator scan.
  - CLI `preset list` shows a `timeout-secs` column.
  - `preset run` uses a preset timeout hint when the command is otherwise on
    the default 30-second timeout.
  - TUI Controls opens a temporary timeout input; entering `default` clears the
    override.
  - TUI execution uses timeout priority: TUI override, preset timeout hint,
    then default 30 seconds.
- Long-running AT command cancellation/specification follow-up was documented:
  - Long-running behavior applies to AT commands as a class, not only to
    `AT+COPS=?`.
  - A normal running-command `Cancel` action is not part of the current
    implementation because host-side read cancellation alone cannot guarantee
    that modem-side command execution has stopped.
  - The current behavior remains visible running state, elapsed time, timeout,
    remaining time, timeout-budget progress, timeout override, and blocking of
    conflicting sends while a command is active.
  - Running-command interruption, host-side read abort, USB reconnect, and AT
    resync were later resolved as out of scope for the application feature set.
- Checkpoint 11.5 explicit device selection gate requirements were documented
  before implementation:
  - Devices is an interactive selection surface when more than one matching USB
    device is visible.
  - If no matching USB device is visible, device-dependent actions such as
    preset execution and ad-hoc AT sending are disabled.
  - If exactly one matching USB device is visible at startup, the TUI
    auto-selects it, shows selected-device detail, and starts command-ready.
  - If multiple matching USB devices are visible at startup, the TUI starts
    with no active execution device and blocks command sending until the user
    explicitly selects one.
  - `d` focuses or re-enters Devices selection, `Up` and `Down` move the
    highlighted candidate, and `Enter` selects the highlighted device.
  - After selecting a device, Devices shows selected-device detail including USB
    manufacturer when readable, USB product when readable, VID, PID, bus, and
    address.
  - Normal Devices display does not show any built-in device label, profile
    label, compatibility label, or agent-defined product name such as `Known`,
    `[known]`, or `Profile hint`.
  - The documented validation hardware remains SORACOM Onyx / Quectel EG25-G,
    but it does not define a closed supported-device list.
  - The product assumes no pre-known device inventory; discovery uses USB devices
    visible at runtime and explicit user selectors.
  - TUI command execution passes the selected device VID, PID, bus, and address.
  - The user can select device A, run `modem-response`, select device B, and
    run `modem-response` again in the same TUI session.
  - Reselecting a device affects subsequent commands only and does not rewrite
    already displayed Response content.
  - Device reselection is disabled while a command is actively running.
- Checkpoint 11.5 explicit device selection gate implementation is complete and
  user approved:
  - TUI state separates the highlighted Devices row from the active execution
    device.
  - A sole visible matching device is auto-selected at startup.
  - Multiple visible matching devices start with no active execution target.
  - Preset execution and ad-hoc AT input are blocked until a device is selected
    when no active execution target exists.
  - `d` focuses Devices selection, `Up` and `Down` move the highlighted
    candidate, and `Enter` confirms the highlighted device.
  - Devices shows selected-device detail outside the selection flow.
  - TUI command execution uses the confirmed active device's VID/PID and
    bus/address.
  - Device reselection after command completion changes the target for
    subsequent commands only.
- Checkpoint 11.5 device identity display correction is implemented:
  - Normal TUI Devices display uses USB descriptor values and explicit selector
    fields rather than built-in device labels, profile labels, compatibility
    labels, or agent-defined product names.
  - Devices detail shows USB manufacturer when readable, USB product when
    readable, VID, PID, bus, and address.
  - `Known`, `[known]`, `Profile hint`, and `USB ID` are not used in normal
    device identity display.
  - `KnownDevice`, `known_name`, and built-in known-device selection logic were
    removed from implementation.
  - `devices --all` was removed because it only existed to support the incorrect
    known-device-list model.
  - TUI execution constrains the selected target by VID, PID, bus, and address.
- Device listing default scope correction is documented and implemented:
  - `atctl devices` now shows descriptor-based plausible AT operation targets
    by default instead of every USB device visible through `libusb`.
  - `atctl devices --all-usb` shows the full USB visibility troubleshooting
    view.
  - The default target filter does not use a known-device list, product-name
    table, hard-coded VID/PID list, or supported-device allow-list.
  - The initial filter is conservative: communication, miscellaneous, or
    vendor-specific device class plus at least one descriptor shape containing
    both bulk IN and bulk OUT endpoints.
  - This descriptor filter is not an AT probe and does not guarantee modem
    support.
  - The descriptor basis is documented with source references: USB-IF class
    codes, USB standard descriptors, libusb descriptor APIs, and CDC ACM bulk
    endpoint guidance.
  - CLI/TUI discovery parity is implemented: TUI normal Devices uses the same
    operation-target scope as `atctl devices`, and a Devices pane action row
    opens an in-app full-USB troubleshooting view equivalent to
    `atctl devices --all-usb`.
  - TUI full-USB troubleshooting keeps operation targets distinct from
    diagnostic-only USB devices; diagnostic-only devices cannot become AT
    sending targets.
- Checkpoint 11.5 TUI list-pane scroll correction is implemented:
  - Devices, Categories, Commands, and Logs are selectable list panes and must
    not lose lower items when item count exceeds pane height.
  - `Up`, `Down`, `PageUp`, `PageDown`, `Home`, and `End` must keep the
    highlighted or selected item visible in the focused list pane.
  - Page movement must be based on the focused pane's visible row capacity, not
    a fixed value.
  - Response keeps line-based scrolling for command output and opened masked
    logs.
  - Status remains informational and non-scrollable.
- Checkpoint 11.5 TUI Status/footer responsibility correction is documented and
  in scope:
  - Status must show state and command context only.
  - Generic keyboard shortcut hints must not be shown inside Status.
  - A dedicated one-row footer or command bar must show short context-sensitive
    key hints.
  - The footer must omit lower-priority hints instead of wrapping into adjacent
    panes.
  - The `?` help overlay remains a concise keyboard operation reference and
    must not include pane-architecture descriptions.
- Application TUI secondary operations now use the reduced-shortcut Controls
  model:
  - `/` opens command search and filters visible commands by name, AT command,
    category, or preset set.
  - `?` opens a modal help overlay. While help is visible, background pane
    actions do not execute; `Esc`, `?`, and `q` close help.
  - `q` quits during normal TUI operation.
  - The Controls pane provides `Enter`-activated rows for ad-hoc AT input,
    edit-before-run or Sequence inputs, timeout override, raw diagnostic
    export, and output masking.
  - Response and Logs provide focused `Enter` action menus for context-specific
    Response copy/save/clear, Response-folder opening, opened-log copy/close,
    selected-log opening, and logs-folder opening.
  - Devices provides `Enter`-activated operation-target selection and full-USB
    troubleshooting view switching. Dedicated global letter shortcuts are not
    required for these secondary operations.
- Checkpoint 12 PTY bridge design was approved and implemented:
  - `portable-pty 0.9.0` and `ctrlc 3.5.2` were added for the PTY bridge and
    signal cleanup path.
  - `atctl bridge --symlink <PATH>` accepts USB selection options and
    `--replace-symlink`.
  - First-time bridge usage starts with `atctl devices`; the user selects the
    current runtime target, preferably with `--bus <BUS> --address <ADDRESS>`.
  - VID/PID values remain runtime selectors and are not treated as required
    prior knowledge or a known-device list.
  - Bridge startup opens the USB transport before creating the symlink, so
    zero-device and multiple-device failures do not leave a symlink behind.
  - Existing regular files and directories are never overwritten.
  - Existing symlinks are rejected unless `--replace-symlink` is provided.
  - Cleanup removes only the symlink created by the current bridge process when
    it still points to the same PTY target.
  - SIGINT, SIGTERM, and SIGHUP set a stop flag so the main bridge loop can
    exit and run cleanup.
  - PTY input is decoded as CR/LF line-oriented AT commands.
  - Safe and sensitive commands can run from the PTY. Sensitive output is
    masked by default.
  - Write, persistent, dangerous, and unknown commands prompt over the PTY and
    require the exact risk label before sending.
- PTY client disconnect from quitting `screen` is treated as normal bridge
  shutdown, not as a USB transport error.
- Transport errors stop the bridge rather than continuing in an ambiguous USB
  session.
- Checkpoint 12 user review completed and approved:
  - User-run `cargo run -- devices` found EG25-G `0x2c7c:0x0125` at bus 1,
    address 4.
  - User-run `cargo run -- inspect --bus 1 --address 4` showed descriptor-shape
    AT candidates.
  - User-run `cargo run -- bridge --symlink /tmp/atctl --bus 1 --address 4`
    started the bridge and created `/tmp/atctl`.
  - `screen /tmp/atctl 115200` executed `AT`, `ATI`, masked `AT+CIMI`, and
    write-risk `ATE0` confirmation as expected.
  - Quitting `screen` with `Ctrl-A`, then `K`, then `y` stopped the bridge
    cleanly after the PTY client disconnect correction.
  - `/tmp/atctl` was removed during cleanup.
- Checkpoint 12.5 modem functionality preset update is complete:
  - `docs/PRESETS.md` was created as the user-facing preset reference.
  - Built-in presets now include `modem-functionality` (`AT+CFUN?`) as a safe
    read and `set-modem-minimum-functionality`, `set-modem-full-functionality`, and
    `restart-modem` as dangerous standard `AT+CFUN` modem functionality actions.
  - Built-in presets now include standard failure diagnostics:
    `extended-error-report` (`AT+CEER`), `error-reporting-status`
    (`AT+CMEE?`), `enable-verbose-errors` (`AT+CMEE=2`), and
    `modem-activity-status` (`AT+CPAS`). `enable-verbose-errors` is
    write-risk because it changes error reporting behavior.
  - The Quectel repository-managed file preset example now includes
    `sim-init-status-quectel` (`AT+QINISTAT`), `pin-retries-quectel`
    (`AT+QPINC?`), `network-name-quectel` (`AT+QSPN`),
    `network-time-quectel` (`AT+QLTS`), `mbn-list-quectel`
    (`AT+QMBNCFG="List"`), and `power-down-quectel` (`AT+QPOWD`).
  - `AT+CFUN?`, `AT+CEER`, `AT+CMEE?`, `AT+CMEE=?`, `AT+CPAS`,
    `AT+QINISTAT`, `AT+QSPN`, and `AT+QLTS` are classified as known safe
    reads. `AT+QPINC?`, `AT+QMBNCFG="List"`, and `AT+QCFG?` are classified as
    known sensitive reads. `AT+CMEE=...` is classified as write-risk.
    `AT+CFUN=...` and `AT+QPOWD` are classified as dangerous.
  - TUI command lists no longer hide dangerous presets. Dangerous presets are
    visible with risk cues and still require exact typed risk confirmation before
    USB access.
- Running-command interruption, host-side read abort, USB reconnect, and AT
  resync were resolved as out of scope for the application feature set.
- OQ-021 raw diagnostic export is resolved and implemented:
  - `atctl send`, `atctl preset run`, and `atctl bridge` expose
    `--raw-log-file <PATH> --raw-log-ack raw-log`.
  - TUI exposes raw diagnostic export capture through the Controls pane, with
    explicit path input, exact `raw-log` acknowledgement, visible active capture
    state, and an explicit Controls action to stop active capture.
  - Raw export files are explicit user-selected JSONL files, refuse overwrite,
    use user-only permissions where supported, and store lossless tx/rx bytes as
    base64 with human-readable previews.
  - CLI raw export validates acknowledgement and overwrite before USB access,
    but creates the raw export file only after command transmission starts.
    USB target selection failures do not leave misleading header-only rawlog
    files; command-send/read failures write a `transport_error` event.
  - Normal terminal/TUI output, command history, session logs, and saved
    Response files keep their existing masking behavior.
- Built-in preset display names have been updated for TUI/CLI clarity:
  - Short or ambiguous names now identify the target or action, such as
    `modem-info`, `current-operator`, `signal-quality`,
    `pdp-auth-settings`, `modem-functionality`, and
    `disable-command-echo`.
  - State-changing presets use action-oriented names, such as
    `enable-verbose-errors`, `set-modem-minimum-functionality`,
    `set-modem-full-functionality`, and `restart-modem`.
  - `docs/SPEC.md`, `docs/PRESETS.md`, `docs/TROUBLESHOOTING.md`, the built-in
    preset definitions, and TUI tests use the same preset names.
- OQ-022 TUI shortcut reduction and Controls pane behavior is resolved and
  implemented:
  - The primary TUI path remains Categories -> Commands -> `Enter`.
  - OQ-023 extends the executable-item surface to `Commands / Sequences` once
    Sequence support is implemented.
  - Global letter shortcuts are limited to `/`, `?`, and `q`.
  - Secondary TUI actions are available through the relevant focusable pane and
    `Enter`, not through many global single-letter shortcuts. Controls owns
    command/session controls, Response owns Response and opened-log view
    actions, and Logs owns log-list actions.
  - The normal layout uses one canonical pane topology with aligned top and
    bottom bands: Devices/Status, Categories, and Commands in the top band;
    Controls, Response, and Logs in the bottom band.
  - The normal vertical allocation uses a stable balanced split so the bottom
    Response/Logs review area has enough space for command output and Sequence
    transcripts. When the usable height has an odd extra row, the bottom
    review area keeps that row.
  - Devices, Status, and Controls share a compact left utility width;
    Categories stays compact; Commands, Response, and Logs receive the
    remaining width for longer AT command, response, and log content.
  - Normal focus order is
    Categories -> Commands -> Controls -> Response -> Logs -> Devices, with the
    existing device-selection gate still starting in Devices when no execution
    target has been selected.
  - Controls rows stay in a stable operation list that reads as actions, not as
    a dense status table. Inline state is limited to values that change the
    action decision itself, and routine action results or unavailable reasons
    appear near the focused action surface.
  - Running-command Status progress uses a separated temporary progress block
    with a width-aware timeout-budget label and a separate compact progress
    bar.
  - Devices full-USB troubleshooting view switching is available through the
    Devices pane and `Enter`.
  - Help is modal, consumes ordinary keys while visible, and shows concise
    keyboard operation instructions without `Primary flow` or pane inventory
    prose.
- OQ-023 Sequence product design is resolved and documented:
  - The product-facing term is `Sequence` for named multi-step AT operations.
  - Presets remain one-shot AT command definitions.
  - `atctl send` and `atctl preset run` remain one-shot surfaces.
  - `atctl sequence list` and `atctl sequence run <SEQUENCE>` are specified as
    production CLI surfaces for multi-step Sequence execution.
  - Product-provided standard SMS send/read/reply Sequences are specified as
    ordinary product actions that do not require users to author TOML before
    use.
  - User-authored Sequence definitions are specified as TOML extension files
    loaded from explicit `--sequence-file` / `--sequence-dir` flags.
  - Repository-managed Quectel TCP/IP and SORACOM TCP endpoint examples are
    specified as explicitly loaded vendor/provider-specific Sequence
    definition files, not default vendor-neutral product Sequences.
  - The shared Sequence engine is specified for prompt waits, payload writes,
    Ctrl-Z or ESC terminators, delayed URC waits, final response waits,
    per-step timeouts, total timeout, masking, risk aggregation, raw diagnostic
    export, active input/review items, SMS decoded-body analysis, derived
    response values, structured step results, success notes, and readable step
    transcripts with origin sections.
  - TUI Sequence integration is specified without adding another permanent
    pane: the executable-item pane becomes `Commands / Sequences`, required
    inputs use a `Run Sequence` modal, Status shows compact current step
    context, and Response shows the step transcript.
- Checkpoint 12.6 Sequence implementation is in place:
  - Shared Sequence model and TOML loader.
  - TOML support for active review items, `success_notes`, and step `evidence`
    rendered as `Analysis:`.
  - Product-provided standard SMS send, receive/list with decode analysis,
    read-by-index, and reply-by-index Sequences.
  - Repository-managed Quectel TCP/IP and SORACOM TCP endpoint example
    Sequence definitions under `examples/sequences`.
  - CLI `atctl sequence list` and `atctl sequence run <SEQUENCE>`, including
    structured JSON step `analysis` results and notes.
  - TUI `Commands / Sequences` display and `Run Sequence` modal active input
    and review before confirmation.
  - Sequence parameter value-resolution metadata for editable defaults, user
    input, modem-confirmed values, selectable values, derived values,
    Sequence-provided values, and external prerequisites.
  - TUI `Run Sequence` input and confirmation render current values with
    source/default/hint context. CLI Sequence list and missing-parameter errors
    use the same Sequence metadata.
  - TUI `Run Sequence` input renders `source=select` SMS storage-index
    candidates from known `AT+CMGL` / `sms-receive-check` results in the same
    modal, shows same-session candidate source and count, supports keyboard
    candidate selection with a visible window that follows the highlighted row,
    does not perform hidden candidate acquisition when the modal opens, and
    keeps manual entry available.
  - Sequence parameters support product-known candidate assistance. The current
    candidate names are `sms-message` and `pdp-context`.
  - TUI `Run Sequence` input shows selectable candidate actions inside the same
    modal before candidates are loaded and keeps those actions available as
    explicit refresh/load actions after candidates are loaded. These actions run
    through the normal command execution path and do not execute hidden modem
    or network I/O when the modal opens.
  - Repository-managed Quectel and SORACOM TCP examples use
    `candidate = "pdp-context"` only for standard PDP context assistance.
    Quectel socket connect ID stays an editable add-on value with a hint to use
    an explicitly loaded Quectel socket-state command when needed; vendor/
    provider TCP Sequences are not promoted to default product-provided
    standard Sequences.
  - TUI `Run Sequence` confirmation is a phase-based modal state: the risk
    instruction and current `Input:` line remain visible, and long value or
    review detail is summarized before it hides the action needed to run or
    cancel.
  - TUI compact Status uses typed concise result summaries rather than
    free-form `Detail:` rows. Full Sequence failure text remains in Response,
    while Status shows state/context such as `Result: failed`.
  - TUI Response shows `Result: failed` before failed-before-response detail,
    so failure is visible without relying on compact Status or color alone.
  - Sequence transcripts separate `Command:`, `Payload:`, `Modem response:`,
    `Decoded SMS:`, `Analysis:`, `Notes:`, and `Result:` so modem output and
    atctl-derived analysis are not mixed under an `Evidence:` prefix.
    Transcript sections are separated with blank lines for readability.
- Checkpoint 12.6 user review is complete and approved:
  - User-run SMS send, receive/list, read, and reply checks were confirmed.
  - User-run Quectel/SORACOM ping and TCP checks were confirmed where the
    reviewer intentionally sent network traffic or payloads.
  - User-run TUI candidate refresh and related Sequence input behavior checks
    were confirmed.

## In Progress

- None.

## Pending

- Checkpoint 13 implementation: Homebrew tap formula, tap-side manual Formula
  update pull-request workflow, source-build fallback, bottle automation, and
  tap CI material in `uchimanajet7/homebrew-atctl`.

## Verification Log

Entries before 2026-06-26 may mention product-language gates as historical
checks. Those checks are superseded by the current test-ownership boundary:
product source and normal `cargo test` cover product behavior, not
coding-agent process compliance.

| Date | Command | Result | Notes |
|---|---|---|---|
| 2026-07-05 | Release/Homebrew publication separation documentation sync | passed | Updated `docs/PACKAGING.md`, `docs/SPEC.md` to version `0.4.108`, and this file. The source repository release workflow remains limited to GitHub Release assets, checksum, and changelog-backed release notes. Homebrew Formula update pull-request creation is documented as explicit `uchimanajet7/homebrew-atctl` tap repository work, normally through a manual `workflow_dispatch` workflow for a chosen release tag. |
| 2026-07-05 | Release/Homebrew separation wording search | passed | `rg` confirmed the current spec version, source release workflow prohibitions for Homebrew Formula pull requests and automatic Homebrew publication, tap-side `workflow_dispatch` wording, and independently executable source release/Homebrew publication wording in `docs/SPEC.md`, `docs/PACKAGING.md`, and this file. |
| 2026-07-05 | `cargo fmt --check` | passed | Formatting is clean after the Release/Homebrew publication separation documentation sync. |
| 2026-07-05 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the Release/Homebrew publication separation documentation sync. |
| 2026-07-05 | `cargo test --all-features --locked` | passed | 267 unit tests and doc tests passed after the Release/Homebrew publication separation documentation sync. |
| 2026-07-05 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the Release/Homebrew publication separation documentation sync. |
| 2026-07-05 | `cargo package --list --allow-dirty` | passed | Cargo source package output remains limited to the approved package `include` whitelist; repository `docs/**`, `_local/**`, and `.github/workflows/**` are not included in the `.crate` source package. |
| 2026-07-05 | Homebrew install command documentation sync | passed | Updated `README.md`, `docs/INSTALL.md`, `docs/PACKAGING.md`, `docs/SPEC.md` to version `0.4.107`, `docs/OPEN-QUESTIONS.md`, and this file so the normal Homebrew one-line install command is `brew install uchimanajet7/atctl/atctl`. The tap repository remains `uchimanajet7/homebrew-atctl`; the user-facing tap name remains `uchimanajet7/atctl`; `brew install atctl` is documented only as the equivalent form after `brew tap uchimanajet7/atctl`. |
| 2026-07-05 | Homebrew install command wording search | passed | The wording search found the fully qualified command in user-facing install docs and found `brew install atctl` only as the tapped equivalent form or historical/pending Homebrew tap context. No stale alternative tap-name install path remains. |
| 2026-07-05 | `cargo fmt --check` | passed | Formatting is clean after the Homebrew install command documentation sync. |
| 2026-07-05 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the Homebrew install command documentation sync. |
| 2026-07-05 | `cargo test --all-features --locked` | passed | 267 unit tests and doc tests passed after the Homebrew install command documentation sync. |
| 2026-07-05 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the Homebrew install command documentation sync. |
| 2026-07-05 | `cargo package --list --allow-dirty` | passed | The Cargo package file list still includes `README.md`, so the updated normal install command is present in the Rust source package metadata/docs surface. Repository `docs/**` remain source-repository documentation and are not part of the Cargo source package include whitelist. |
| 2026-07-05 | Source repository release workflow release-note update | passed | Updated `.github/workflows/release.yml`, `CHANGELOG.md`, `docs/SPEC.md` to version `0.4.106`, `docs/PACKAGING.md`, `docs/OPEN-QUESTIONS.md`, and this file. The workflow now validates that the tag version matches `Cargo.toml`, extracts the matching released-version section from `CHANGELOG.md`, requires a `YYYY-MM-DD` release date and non-empty notes, and creates the GitHub Release with `gh release create --verify-tag --notes-file`. Homebrew tap workflow, bottles, tags, release publishing by the agent, SBOM/provenance/attestation, signing/notarization, and other platform artifacts remain out of this source-repository workflow scope. |
| 2026-07-05 | Source repository release workflow static scope check | passed | `.github/workflows/release.yml` retains the approved tag trigger, fixed `macos-26` arm64 macOS runner, release-job `contents: write`, first-party `actions/checkout`, `aarch64-apple-darwin` target, required cargo verification commands, top-level executable archive packaging, SHA-256 generation, tag/Cargo version validation, changelog-backed notes, and `gh release create --verify-tag --notes-file`. The workflow file does not contain Homebrew tap update, bottle publishing, SBOM/provenance/attestation, signing/notarization, `workflow_dispatch`, `macos-latest`, `--generate-notes`, or `--clobber` behavior. |
| 2026-07-05 | Release workflow YAML parse | passed | Ruby YAML parsing loaded `.github/workflows/release.yml` successfully after the release-note update. |
| 2026-07-05 | Changelog release-note extraction dry run | passed | The workflow extraction logic produced `/tmp/atctl-release-notes-test.md` from `CHANGELOG.md` for Cargo package version `0.1.0`, validated the `YYYY-MM-DD` heading date, and confirmed non-empty release-note content. |
| 2026-07-05 | `cargo fmt --check` | passed | Formatting is clean after the source repository release-note update. |
| 2026-07-05 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the source repository release-note update. |
| 2026-07-05 | `cargo test --all-features --locked` | passed | 267 unit tests and doc tests passed after the source repository release-note update. |
| 2026-07-05 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the source repository release-note update. |
| 2026-07-05 | `cargo package --list --allow-dirty` | passed | Cargo source package output remains limited to the approved package `include` whitelist; `.github/workflows/release.yml`, `docs/**`, `_local/**`, and release workflow files are not included in the `.crate` source package. |
| 2026-07-05 | `cargo package --allow-dirty` | passed | Initial sandboxed run failed because DNS resolution for `index.crates.io` was blocked by the sandbox; the approved network rerun packaged 57 files, 809.0KiB uncompressed and 146.9KiB compressed, then verified the package by compiling the extracted package copy. |
| 2026-07-05 | `cargo fmt --check` | passed | Formatting is clean after the source repository release workflow update. |
| 2026-07-05 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the source repository release workflow update. |
| 2026-07-05 | `cargo test --all-features --locked` | passed | 267 unit tests and doc tests passed after the source repository release workflow update. |
| 2026-07-05 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the source repository release workflow update. |
| 2026-07-05 | `cargo package --list --allow-dirty` | passed | Cargo source package output remains limited to the approved package `include` whitelist; `.github/workflows/release.yml`, `docs/**`, `_local/**`, and release workflow files are not included in the `.crate` source package. |
| 2026-07-05 | Cargo source package scope update | passed | Updated `Cargo.toml`, `README.md`, `docs/SPEC.md` to version `0.4.105`, `docs/PACKAGING.md`, and this file. Cargo package output now uses an explicit `include` whitelist so source-package output excludes project-local agent files, backups, local history, build outputs, and release-workflow drafts. Homebrew remains the normal end-user install path; Cargo source package output is documented as a separate Rust packaging concern. |
| 2026-07-05 | `cargo fmt --check` | passed | Formatting is clean after the Cargo source package scope update. |
| 2026-07-05 | `cargo package --list --allow-dirty` | passed | Cargo package output is limited to `CHANGELOG.md`, `Cargo.lock`, Cargo-generated `Cargo.toml.orig`, `LICENSE`, `README.md`, repository-managed examples under `examples/presets/**` and `examples/sequences/**`, and Rust source files under `src/**`; `_local/**`, `docs/**`, and `target/**` are excluded. |
| 2026-07-05 | `cargo package --allow-dirty` | passed | Cargo packaged 57 files, 808.1KiB uncompressed and 146.4KiB compressed, then verified the package by compiling the extracted package copy. |
| 2026-07-05 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the Cargo source package scope update. |
| 2026-07-05 | `cargo test --all-features --locked` | passed | 267 unit tests and doc tests passed after the Cargo source package scope update. |
| 2026-07-05 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the Cargo source package scope update. |
| 2026-07-04 | TUI candidate refresh action update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.103`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Candidate-backed Sequence inputs now keep explicit refresh/load actions visible after same-session candidates are loaded, covering product-known `sms-message` and `pdp-context` candidates for built-in, repository-managed, and user-authored add-on Sequences without hidden modem I/O on modal open. |
| 2026-07-04 | `cargo fmt --check` | passed | Formatting is clean after the TUI candidate refresh action update. |
| 2026-07-04 | `cargo test --all-features --locked tui::tests` | passed | 115 TUI tests passed, including loaded SMS candidate refresh with same-modal write confirmation, loaded PDP candidate refresh, candidate action failure separation, SMS candidate selection, and existing candidate acquisition behavior before candidates are loaded. |
| 2026-07-04 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI candidate refresh action update. |
| 2026-07-04 | `cargo test --all-features --locked` | passed | 267 unit tests and doc tests passed after the TUI candidate refresh action update. |
| 2026-07-04 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI candidate refresh action update. |
| 2026-07-04 | TUI Logs action-menu modal feedback persistence update | passed | Updated `docs/SPEC.md` to version `0.4.102`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Log action menu feedback now distinguishes modal-state feedback from transient action feedback, so the selected-log-missing warning remains visible while the same Log actions menu is open and selection moves do not hide why the open-log row is unavailable. |
| 2026-07-04 | `cargo fmt --check` | passed | Formatting is clean after the TUI Logs action-menu modal feedback persistence update. |
| 2026-07-04 | `cargo test --all-features --locked deleted_selected_log_feedback_persists_during_log_action_navigation` | passed | Added targeted visible-state coverage for a deleted selected session log: the menu offers only `Open logs folder`, keeps the missing-log message after Down/Home/End navigation, and does not show `Open log in Response`. |
| 2026-07-04 | `cargo test --all-features --locked tui::tests` | passed | 113 TUI tests passed, including the new deleted-log modal-state feedback persistence coverage. |
| 2026-07-04 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Logs action-menu modal feedback persistence update. |
| 2026-07-04 | `cargo test --all-features --locked` | passed | 265 unit tests and doc tests passed after the TUI Logs action-menu modal feedback persistence update. |
| 2026-07-04 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Logs action-menu modal feedback persistence update. |
| 2026-07-04 | TUI Logs stale-target handling update | passed | Updated `docs/SPEC.md` to version `0.4.101`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Log action menus now bind to the selected log identity instead of a mutable list index, refresh the Logs list non-destructively, report when the selected log no longer exists, and do not fall back to `history.jsonl` or another row after external deletion. |
| 2026-07-04 | `cargo fmt --check` | passed | Formatting is clean after the TUI Logs stale-target handling update. |
| 2026-07-04 | `cargo test --all-features --locked tui::tests` | passed | 112 TUI tests passed, including deleted-session-with-history coverage before the action menu and after the action menu is already open. |
| 2026-07-04 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Logs stale-target handling update. |
| 2026-07-04 | `cargo test --all-features --locked` | passed | 264 unit tests and doc tests passed after the TUI Logs stale-target handling update. |
| 2026-07-04 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Logs stale-target handling update. |
| 2026-07-03 | TUI Logs same-session refresh and logging path alignment update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.99`, this file, `src/cli.rs`, `src/cli/tests.rs`, `src/tui/mod.rs`, and `src/tui/tests.rs`. TUI Logs now uses the same resolved logging paths as log writers and `atctl logs list`, including configured `[log].log_dir`; after TUI execution finishes, the Logs list refreshes in the same session so new `.session.log` files can appear without restarting. Log-list refresh errors remain owned by the Logs pane and do not replace the current execution Status or Response. |
| 2026-07-03 | `cargo fmt --check` | passed | Formatting is clean after the TUI Logs same-session refresh update. |
| 2026-07-03 | `cargo test --all-features --locked tui::tests` | passed | 108 TUI tests passed, including resolved log-directory listing, same-session session-log refresh, refresh-error Logs-pane rendering, existing Logs action behavior, Status context, and Response behavior. |
| 2026-07-03 | `cargo test --all-features --locked cli::tests::logging_paths_from_loaded_config_uses_configured_session_log_dir` | passed | CLI logging path resolution uses configured `[log].log_dir` as the session-log directory. |
| 2026-07-03 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Logs same-session refresh update. |
| 2026-07-03 | `cargo test --all-features --locked` | passed | 260 unit tests and doc tests passed after the TUI Logs same-session refresh update. |
| 2026-07-03 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Logs same-session refresh update. |
| 2026-07-03 | TUI Status terminal timestamp grammar update | passed | Updated `docs/SPEC.md` to version `0.4.98`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Compact Status now keeps terminal execution timestamps in Status immediately after `Status:` using event-owned labels such as `Completed:`, `Failed:`, and `Cancelled:`. Normal Status no longer uses `Completed at:`, `Failed at:`, or `Cancelled at:` labels, does not combine the timestamp label with `Result:`, and widens the compact utility column so the full UTC timestamp row fits in normal TUI layouts. |
| 2026-07-03 | `cargo fmt --check` | passed | Formatting is clean after the TUI Status terminal timestamp grammar update. |
| 2026-07-03 | `cargo test --all-features --locked tui::tests` | passed | 105 TUI tests passed, including completed, failed, and cancelled terminal event timestamp rows, Response clear-state coherence, Status row order, running progress context, viewed-log Status context, and non-color state affordance coverage. |
| 2026-07-03 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Status terminal timestamp grammar update. |
| 2026-07-03 | `cargo test --all-features --locked` | passed | 256 unit tests and doc tests passed after the TUI Status terminal timestamp grammar update. |
| 2026-07-03 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Status terminal timestamp grammar update. |
| 2026-07-03 | TUI Status lifecycle-label grammar update | passed | Updated `docs/SPEC.md` to version `0.4.97`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Compact Status now lets `Status:` own lifecycle state, uses neutral `Command:`, `Sequence:`, or `Action:` target rows for active execution context, labels literal one-shot AT strings as `AT command:`, and renders terminal timestamps with event-owned labels such as `Completed at:`, `Failed at:`, or `Cancelled at:`. Response clear-state behavior remains owned by the Response pane while Status keeps the previous execution context. |
| 2026-07-03 | `cargo fmt --check` | passed | Formatting is clean after the TUI Status lifecycle-label grammar update. |
| 2026-07-03 | `cargo test --all-features --locked tui::tests` | passed | 104 TUI tests passed, including selected Sequence context, running command context, completed command context after selection movement, failed Sequence compact Status, candidate action failure, Response clear-state coherence, and non-color state affordance coverage. |
| 2026-07-03 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Status lifecycle-label grammar update. |
| 2026-07-03 | `cargo test --all-features --locked` | passed | 255 unit tests and doc tests passed after the TUI Status lifecycle-label grammar update. |
| 2026-07-03 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Status lifecycle-label grammar update. |
| 2026-07-02 | TUI Status execution-context grammar update | passed | Updated `docs/SPEC.md` to version `0.4.96`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Compact Status now separates selected-item context from confirming, running, executed, and viewed-log contexts with `Selected ...`, `Confirming ...`, `Executing ...`, and `Executed ...` labels. Running Status no longer duplicates a static timeout row when the progress block is present. Completed/failed Status preserves result, finished-time, risk, and masking priority in compact layouts; the separate command row is omitted only when it would displace those higher-priority rows. |
| 2026-07-02 | `cargo fmt --check` | passed | Formatting is clean after the TUI Status execution-context grammar update. |
| 2026-07-02 | `cargo test --all-features --locked tui::tests` | passed | 104 TUI tests passed, including selected/active execution label separation, completed-result Status after selection movement, running progress context, Response clear-state coherence, viewed-log Status context, and compact Status preservation of result, finished time, and risk. |
| 2026-07-02 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Status execution-context grammar update. |
| 2026-07-02 | `cargo test --all-features --locked` | passed | 255 unit tests and doc tests passed after the TUI Status execution-context grammar update. |
| 2026-07-02 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Status execution-context grammar update. |
| 2026-07-02 | TUI completed/failed Status row-order update | passed | Updated `docs/SPEC.md` to version `0.4.95`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Completed/failed active execution Status now orders post-execution context as state, executed item identity, source when present, command text when relevant, result, finished time, risk, then output masking when shown. Very compact Status layouts may keep the command text on the executed item identity row so `Result`, `Finished`, `Risk`, and masking context remain visible instead of being pushed out. |
| 2026-07-02 | `cargo fmt --check` | passed | Formatting is clean after the TUI completed/failed Status row-order update. |
| 2026-07-02 | `cargo test --all-features --locked tui::tests` | passed | 103 TUI tests passed, including completed command Status order, failed Sequence Status order, short Status command inlining, and preservation of `Risk` when no output-masking row is shown. |
| 2026-07-02 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI completed/failed Status row-order update. |
| 2026-07-02 | `cargo test --all-features --locked` | passed | 254 unit tests and doc tests passed after the TUI completed/failed Status row-order update. |
| 2026-07-02 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI completed/failed Status row-order update. |
| 2026-07-02 | TUI Status finished-time and Response clear-state update | passed | Updated `docs/SPEC.md` to version `0.4.94`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Completed/failed execution context now keeps finished-time Status context using the full `YYYY-MM-DDTHH:MM:SSZ` timestamp, split or combined with the result label only when the compact Status dimensions cannot fit a separate full `Finished:` row. Clearing the Response body leaves an intentional `Response body cleared.` / `Cleared: ...` Response empty state instead of falling back to `No response.` while the previous execution Status remains visible. |
| 2026-07-02 | `cargo test --all-features --locked tui::tests` | passed | 103 TUI tests passed, including finished-time Status rendering, compact Status dimension handling, Response clear-state rendering, hidden copy/save/clear actions after a cleared body, and preservation of the previous execution Status after Response clear. |
| 2026-07-02 | `cargo fmt --check` | passed | Formatting is clean after the TUI Status finished-time and Response clear-state update. |
| 2026-07-02 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Status finished-time and Response clear-state update. |
| 2026-07-02 | `cargo test --all-features --locked` | passed | 254 unit tests and doc tests passed after the TUI Status finished-time and Response clear-state update. |
| 2026-07-02 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI Status finished-time and Response clear-state update. |
| 2026-07-01 | TUI action-menu shared folder context update | passed | Updated `docs/SPEC.md` to version `0.4.92`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Response actions, Log view actions, and Log actions now show their folder location once as shared action-menu context (`Response folder:` or `Logs folder:`) instead of repeating `Saves to:` / `Folder:` detail under individual rows. Action labels and side effects are unchanged. |
| 2026-07-01 | `cargo fmt --check` | passed | Formatting is clean after the TUI action-menu shared folder context update. |
| 2026-07-01 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI action-menu shared folder context update. |
| 2026-07-01 | `cargo test --all-features --locked tui::tests` | passed | 102 TUI tests passed, including Response actions, Log view actions, Log actions, shared Response folder context, shared Logs folder context, and negative checks for the old row-level `Saves to:` / `Folder:` details. |
| 2026-07-01 | `cargo test --all-features --locked` | passed | 253 unit tests and doc tests passed after the TUI action-menu shared folder context update. |
| 2026-07-01 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI action-menu shared folder context update. |
| 2026-06-30 | TUI Response / Logs action model correction | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.91`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Response execution-result actions are now copy, save, open Response folder, and clear. Response log-view actions are copy displayed log, open logs folder, and close log view. Logs list actions are open selected log in Response and open logs folder. One-shot action-menu commands close after selection and compact Status reports concise action results. The folder-location presentation from this entry was superseded by the 2026-07-01 shared action-menu context update. |
| 2026-06-30 | `cargo fmt --check` | passed | Formatting is clean after the TUI Response / Logs action model correction. |
| 2026-06-30 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI Response / Logs action model correction. |
| 2026-06-30 | `cargo test --all-features --locked tui::tests` | passed | 102 TUI tests passed, including Response action rendering, log-view Response actions, Logs action rendering, Response folder context, log folder context, clear Response behavior, and close log view behavior. |
| 2026-06-30 | `cargo test --all-features --locked` | passed | 253 unit tests and doc tests passed after the TUI Response / Logs action model correction. |
| 2026-06-30 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | failed, then passed | Initial run failed on a new `collapsible_if` warning in `path_display_label`; the helper was corrected and the same Clippy command then passed. |
| 2026-06-29 | TUI Controls / Response / Logs action responsibility update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.90`, `docs/OPEN-QUESTIONS.md`, `docs/DEVELOPMENT.md`, this file, `src/tui/mod.rs`, and `src/tui/tests.rs`. Controls now owns command/session actions only; Response and Logs expose focused `Enter` action menus for Response copy/save/clear, saved-file handling, selected-log handling, and explicit directory open requests. `Rerun last` was removed so repeated execution happens from the visible selected command or Sequence. This historical entry was superseded on 2026-06-30 by the simplified Response/Logs action model that removes primary file-copy actions. |
| 2026-06-29 | `cargo fmt --check` | passed | Formatting is clean after the TUI action responsibility update. |
| 2026-06-29 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the TUI action responsibility update. |
| 2026-06-29 | `cargo test --all-features --locked tui::tests` | passed | 100 TUI tests passed, including Response-focused Enter action menu behavior, Categories Enter non-execution behavior, Response action menu rendering, Logs action menu rendering, Response action menu copy feedback, saved-file action handling, and log action handling. |
| 2026-06-29 | `cargo test --all-features --locked` | passed | 251 unit tests and doc tests passed after the TUI action responsibility update. |
| 2026-06-29 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TUI action responsibility update. |
| 2026-06-29 | Logs listing and TUI log file action update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.89`, `docs/OPEN-QUESTIONS.md`, this file, `src/log/history.rs`, `src/tui/mod.rs`, and `src/tui/tests.rs`. Shared log listing now keeps `history.jsonl` separate and orders `.session.log` files newest-first; TUI Logs now labels the mixed list as `Saved logs:` and covered non-destructive selected-log file actions at that time. |
| 2026-06-29 | `cargo fmt --check` | passed | Formatting is clean after the Logs listing and TUI log file action update. |
| 2026-06-29 | `cargo test --all-features --locked log::history` | passed | History/session listing tests cover no-side-effect listing and newest-first session log ordering after the aggregate history row. |
| 2026-06-29 | `cargo test --all-features --locked tui::tests` | passed | TUI tests covered the `Saved logs:` heading, selected-log file actions, and missing-log feedback at that time. |
| 2026-06-29 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the Logs listing and TUI log file action update. |
| 2026-06-29 | `cargo test --all-features --locked` | passed | 247 unit tests and doc tests passed after the Logs listing and TUI log file action update. |
| 2026-06-29 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the Logs listing and TUI log file action update. |
| 2026-06-29 | Sequence wait/completion contract update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.88`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, this file, `examples/sequences/quectel.toml`, `examples/sequences/soracom.toml`, `src/transport/traits.rs`, `src/transport/test_support.rs`, `src/sequences/loader.rs`, and `src/sequences/engine/tests.rs`. Sequence transport reads now accumulate chunks until the requested matcher is satisfied, repository-managed ping Sequences wait for `+QPING:` instead of `OK`, and the loader rejects semantic success flags whose wait markers cannot provide the required result lines or counters. |
| 2026-06-29 | `cargo fmt --check` | passed | Formatting is clean after the Sequence wait/completion contract update. |
| 2026-06-29 | `cargo test sequences::loader` | passed | Loader validation rejects `require_ping_success` with only `expect = "OK"` and rejects `require_tcp_ack` without a `+QISEND:` counter marker; repository-managed example Sequences still load through the drop-in loader. |
| 2026-06-29 | `cargo test quectel_ping_sequence` and `cargo test soracom_ping_sequence` | passed | Ping Sequence tests cover received replies, zero-reply failure, SORACOM Ping Response destination coverage, and a command-accepted `OK` followed by later `+QPING:` result lines. |
| 2026-06-29 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after the Sequence wait/completion contract update. |
| 2026-06-29 | `cargo test --all-features --locked` | passed | 243 unit tests and doc tests passed after the Sequence wait/completion contract update. |
| 2026-06-29 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the Sequence wait/completion contract update. |
| 2026-06-29 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output loads product SMS Sequences plus `quectel-tcp-send-check`, `quectel-ping-check`, `soracom-ping-check`, and `soracom-unified-endpoint-tcp-send-check`; both ping Sequences keep declared/effective risk `write`, and `soracom-beam-tcp-test-echo-check` remains absent from the default repository-managed examples. |
| 2026-06-28 | SORACOM/Quectel ping Sequence update | passed | Removed `soracom-beam-tcp-test-echo-check` from the default repository-managed SORACOM example file, added `soracom-ping-check` for `pong.soracom.io`, added generic `quectel-ping-check`, added `require_ping_success` Sequence step metadata, parsed `+QPING:` reply/summary output into `Analysis:`, and classified `AT+QPING=` as write-risk. Updated `README.md`, `docs/SPEC.md` to version `0.4.87`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `examples/sequences/quectel.toml`, `examples/sequences/soracom.toml`, `src/at/risk.rs`, `src/cli.rs`, `src/sequences/model.rs`, `src/sequences/loader.rs`, `src/sequences/engine.rs`, and related tests. |
| 2026-06-28 | `cargo fmt --check` | passed | Formatting is clean after the SORACOM/Quectel ping Sequence update. |
| 2026-06-28 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after adding `require_ping_success`, QPING analysis, and QPING risk classification. |
| 2026-06-28 | `cargo test --all-features --locked` | passed | 241 unit tests and doc tests passed, including QPING success/failure parsing, SORACOM Ping Response destination coverage, and repository-managed example loading without the Beam Sequence. |
| 2026-06-28 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the SORACOM/Quectel ping Sequence update. |
| 2026-06-28 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output shows `quectel-ping-check`, `quectel-tcp-send-check`, `soracom-ping-check`, and `soracom-unified-endpoint-tcp-send-check`; `soracom-beam-tcp-test-echo-check` is not listed. Both ping Sequences show declared/effective risk as `write`. |
| 2026-06-28 | TCP acknowledgement success-condition update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.86`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `examples/sequences/quectel.toml`, `examples/sequences/soracom.toml`, `src/sequences/model.rs`, `src/sequences/loader.rs`, `src/sequences/engine.rs`, and related tests. Repository-managed TCP examples now send fixed-length payload bytes without SMS-style Ctrl-Z and require `AT+QISEND=<connectID>,0` counters to show the whole payload acknowledged before reporting Sequence success. |
| 2026-06-28 | `cargo fmt --check` | passed | Formatting is clean after the TCP acknowledgement success-condition update. |
| 2026-06-28 | `cargo check --all-targets --all-features --locked` | passed | All targets compile after adding `require_tcp_ack` Sequence step metadata and TCP acknowledgement retry/failure handling. |
| 2026-06-28 | `cargo test quectel_sequence` | passed | Targeted Quectel TCP Sequence coverage passed, including ACK retry, ACK-incomplete failure, PDP context reuse/activation, socket cleanup, and terminal error transcript behavior. |
| 2026-06-28 | `cargo test tcp_sequence_fixed_length_payload_does_not_append_ctrl_z` | passed | Raw diagnostic log coverage confirms fixed-length TCP payload writes do not append Ctrl-Z. |
| 2026-06-28 | `cargo test --all-features --locked` | passed | 238 unit tests and doc tests passed after the TCP acknowledgement success-condition update. |
| 2026-06-28 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the TCP acknowledgement success-condition update. |
| 2026-06-28 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | Product SMS Sequences plus repository-managed Quectel and SORACOM TCP example Sequences load successfully with the updated TOML metadata. |
| 2026-06-28 | SMS storage-index candidate label update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.85`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `src/sequences/builtin.rs`, `src/sequences/engine.rs`, `src/tui/mod.rs`, and `src/tui/tests.rs`. TUI SMS candidate rows now label modem-returned indexes as `storage=<index>`, candidate pagination is labeled as `Candidate rows`, and read/reply review text says `SMS storage index`. The selected value is still the unmodified modem-returned index used in `AT+CMGR=<index>`; no 0-based or 1-based normalization was added. |
| 2026-06-28 | `cargo fmt --check` | passed | Formatting is clean after the SMS storage-index candidate label update. |
| 2026-06-28 | `cargo test completed_sms_receive_sequence_updates_candidates_for_next_select_modal` | passed | Targeted TUI candidate refresh coverage confirms a `sms-receive-check` result appears in the next `Run Sequence` modal as `storage=<index>`. |
| 2026-06-28 | `cargo test --all-features --locked` | passed | 235 unit tests and doc tests passed after updating candidate labels, candidate-window range wording, SMS read/reply labels, and related docs. |
| 2026-06-28 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the SMS storage-index candidate label update. |
| 2026-06-28 | `cargo run -- sequence list` | passed | Product SMS Sequences still expose required params as `index(select,sms-message)`; the CLI parameter key remains `index` while user-facing TUI/review labels clarify `SMS storage index`. |
| 2026-06-27 | Quectel TCP Sequence repeated-run state handling | passed | Updated Sequence execution so Quectel TCP examples check `AT+QIACT?`, reuse an already active selected PDP context, only send `AT+QIACT=<contextID>` when needed, preserve terminal error responses in transcripts, and run visible `AT+QICLOSE=<connectID>` cleanup after socket-open success if a later step fails. Updated `README.md`, `docs/SPEC.md` to version `0.4.82`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, example Sequence TOML, engine/TUI/loader tests, and TUI non-success status handling. |
| 2026-06-27 | `cargo fmt --check` | passed | Formatting is clean after the Quectel TCP Sequence repeated-run state handling update. |
| 2026-06-27 | `cargo test` | passed | 230 unit tests and doc tests passed, including active PDP context reuse, inactive context activation, socket cleanup after later failure, terminal error transcript preservation, loader parsing of new Sequence step metadata, and TUI failed-state rendering when a Sequence transcript exists with non-success status. |
| 2026-06-27 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Quectel TCP Sequence repeated-run state handling update. |
| 2026-06-27 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output loads product SMS Sequences plus Quectel and SORACOM repository-managed TCP example Sequences with unchanged sequence-set labels and required parameter summaries. |
| 2026-06-27 | Rust 1.96 dependency and source-structure maintenance | passed | Updated `Cargo.toml` to Rust 1.96 / Edition 2024 direct dependency baselines, refreshed `Cargo.lock`, moved test-only transport support out of the public transport module, split large test modules into sibling `tests.rs` files, extracted TUI response/clipboard helpers, removed unused scaffold code, made config TOML reject unknown fields, updated `README.md`, `docs/SPEC.md` to version `0.4.81`, `docs/DEVELOPMENT.md`, and `docs/PACKAGING.md`. Product source and normal product tests still contain product behavior checks only. |
| 2026-06-27 | `rustup update stable`; `rustc -Vv`; `cargo -V` | passed | Stable toolchain is Rust/Cargo 1.96.0 before dependency maintenance. |
| 2026-06-27 | `cargo search` for direct dependencies | passed | Direct dependency latest versions were checked before updating `Cargo.toml`. |
| 2026-06-27 | `cargo update` | passed | `Cargo.lock` refreshed for the Rust 1.96-compatible dependency graph after manifest updates. |
| 2026-06-27 | `cargo tree --duplicates` | reviewed | Remaining duplicate transitive crates come from upstream dependency graphs such as `portable-pty`, `ctrlc`, and `ratatui`; no direct duplicate dependency was added. |
| 2026-06-27 | `cargo fmt --check` | passed | Formatting is clean after the Rust 1.96 dependency and source-structure maintenance. |
| 2026-06-27 | `cargo check --all-targets --all-features --locked` | passed | Full-target type checking passes with no warnings. |
| 2026-06-27 | `cargo test --all-features --locked` | passed | 226 unit tests and doc tests passed after the Rust 1.96 dependency and source-structure maintenance. |
| 2026-06-27 | `cargo clippy --all-targets --all-features --locked -- -D warnings` | passed | No Clippy warnings after the Rust 1.96 dependency and source-structure maintenance. |
| 2026-06-27 | Loaded definition internal normalization implementation | passed | Added internal `PresetDefinition` and `SequenceDefinition` normalization boundaries, routed product built-ins and TOML-loaded definitions through those boundaries, preserved origin metadata, and updated `README.md`, `docs/SPEC.md` to version `0.4.80`, `docs/DEVELOPMENT.md`, and `docs/PRESETS.md`. User-facing TUI/CLI labels and execution behavior are unchanged. |
| 2026-06-27 | `cargo fmt --check` | passed | Formatting is clean after the internal definition/draft normalization implementation. |
| 2026-06-27 | `cargo test definition` | passed | 4 tests passed, covering Preset/Sequence definition conversion and product-origin preservation. |
| 2026-06-27 | `cargo test presets::builtin` | passed | 4 tests passed, covering built-in preset names, curated order, timeout hint preservation, and vendor/carrier exclusion. |
| 2026-06-27 | `cargo test sequences::builtin` | passed | 1 test passed, confirming built-in Sequences keep product origin after definition conversion. |
| 2026-06-27 | `cargo test presets::loader` | passed | 8 tests passed, confirming TOML file presets still parse, reject legacy fields, keep titles, preserve timeout hints, reject duplicates, and load repository examples. |
| 2026-06-27 | `cargo test sequences::loader` | passed | 4 tests passed, confirming TOML Sequences still parse, reject duplicates, preserve deterministic directory order, and load repository examples. |
| 2026-06-27 | `cargo test` | passed | 226 unit tests and doc tests passed after the normalization implementation. |
| 2026-06-27 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the normalization implementation. |
| 2026-06-27 | `cargo run -- preset list --preset-dir examples/presets` | passed | CLI output still shows product presets under `Product presets` and repository-managed examples under `Quectel commands` and `SORACOM commands` after the normalization implementation. |
| 2026-06-27 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output still shows product Sequences under `Product Sequences` and repository-managed examples under `Quectel Sequences` and `SORACOM Sequences` after the normalization implementation. |
| 2026-06-26 | Loaded definition contract boundary update | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.79`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, `src/cli.rs`, this file, `_local/agent-history/incident-log.md`, and memory notes. The update preserves product-provided, repository-managed, and user-authored origins while making the shared loaded definition contract explicit for listing, selection, risk, confirmation, masking, logging, raw diagnostic export, and execution. Agent-process recurrence prevention remains outside product source and normal product tests. |
| 2026-06-26 | `cargo fmt --check` | passed | Formatting is clean after the loaded definition contract boundary update. |
| 2026-06-26 | `cargo test` | passed | 222 unit tests and doc tests passed. This includes the updated CLI help wording and existing product behavior coverage. |
| 2026-06-26 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the loaded definition contract boundary update. |
| 2026-06-26 | `cargo run -- --help` | passed | Root help now describes `preset` as `List or run one-shot product and loaded file presets` and `sequence` as `List or run product and loaded multi-step Sequences`. |
| 2026-06-26 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output shows product-provided standard SMS Sequences under `Product Sequences` and explicitly loaded repository-managed Quectel/SORACOM examples under their TOML titles, while sharing candidate metadata and Sequence list columns. |
| 2026-06-26 | `cargo run -- preset list --preset-dir examples/presets` | passed | CLI output shows product presets under `Product presets` and explicitly loaded Quectel/SORACOM file presets under their TOML titles with the same risk and listing columns. |
| 2026-06-26 | Loaded definition contract residual search | passed | The searched legacy Sequence/source phrases were absent from current docs/source/instructions/examples/history. `src` still contains no old product-language test module and no `include_str!` checks over `AGENTS.md` or `docs`. Remaining old product-language test module name matches are historical incident/status entries only. |
| 2026-06-26 | `git status --short` | failed | This workspace copy does not expose a `.git` directory to the command environment, so a Git working-tree summary could not be produced from this checkout. |
| 2026-06-26 | Recurrence-prevention ownership cleanup | passed | Updated `src/lib.rs`, `AGENTS.md`, `docs/SPEC.md` to version `0.4.78`, `docs/DEVELOPMENT.md`, this file, `_local/agent-history/incident-log.md`, and memory notes. Removed the `product_language_tests` document phrase-check module from the product library crate. Agent-process compliance is now kept in project agent instructions, incident records, memory, or a separately approved process check, not product source or normal product tests. |
| 2026-06-26 | `cargo fmt --check` | passed | Formatting is clean after the recurrence-prevention ownership cleanup. |
| 2026-06-26 | `cargo test` | passed | 222 unit tests and doc tests passed. The previous product-language document phrase checks are intentionally gone from the normal product test suite; the remaining tests cover product behavior, runtime contracts, and user-facing outputs. |
| 2026-06-26 | `cargo test product_language_tests` | passed | The old product-language filter now runs 0 tests, confirming that the document phrase-check module is no longer present in the product crate. |
| 2026-06-26 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the recurrence-prevention ownership cleanup. |
| 2026-06-26 | Recurrence-prevention ownership residual search | passed | `src` no longer contains `product_language_tests` or `include_str!` checks over `AGENTS.md` / `docs`. `docs/SPEC.md` and `docs/DEVELOPMENT.md` no longer contain the removed agent answer-scope gate headings. Remaining `product-language` matches are historical log/status/memory references or current rules forbidding reintroduction. |
| 2026-06-26 | Packaging distribution strategy documentation update | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.77`, `docs/INSTALL.md`, `docs/PACKAGING.md`, `docs/OPEN-QUESTIONS.md`, and this file. Packaging strategy now treats `atctl` as a CLI/TUI executable with Homebrew formula as the normal user path, bottle support as the preferred normal Homebrew state, source-build as fallback, GitHub Releases archives as release/manual artifacts, and direct download/Cask/macOS app distribution as outside the normal path unless separately approved. |
| 2026-06-26 | Packaging distribution residual-term search | passed | No superseded packaging-stage wording remained in `README.md` or `docs/`. Remaining early-stage wording matches are unrelated platform-scope notes. |
| 2026-06-26 | `cargo test` | passed | 235 unit tests and doc tests passed after the packaging distribution documentation update. |
| 2026-06-26 | Candidate action confirmation/failure documentation and implementation update | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.76`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `src/lib.rs`, and `src/tui/mod.rs`. TUI `Run Sequence` candidate actions now use same-modal risk confirmation when required, do not execute hidden I/O on modal open, and report failed candidate acquisition as `Action` failure rather than selected Sequence body failure. |
| 2026-06-26 | `cargo fmt --check` | failed, then passed | Initial check reported rustfmt differences in `src/tui/mod.rs`; `cargo fmt` was run, and the final `cargo fmt --check` passed. |
| 2026-06-26 | `cargo test product_language_tests::sequence_candidate_values_require_same_modal_candidate_selection` | failed, then passed | Tightening the product-language gate first exposed incomplete same-modal confirmation wording in `docs/SPEC.md` and `src/lib.rs`; after those corrections, the targeted test passed. |
| 2026-06-26 | `cargo test` | passed | 234 unit tests and doc tests passed, including product-language gates, candidate action confirmation, failed candidate action Status/Response separation, Sequence candidate extraction, TUI candidate actions, same-modal SMS candidate selection, add-on Sequence loading, and long confirmation input visibility. |
| 2026-06-26 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the candidate action confirmation/failure implementation. |
| 2026-06-26 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output shows product SMS candidate metadata as `sms-message`, TCP add-on PDP metadata as `pdp-context`, and Quectel socket connect ID as an editable default without product-standard socket candidate assistance. |
| 2026-06-26 | `cargo run -- preset list --preset-dir examples/presets` | passed | CLI output shows Quectel `pdp-contexts-quectel` and `socket-state-quectel` as explicitly loaded add-on commands with sensitive declared/effective risk. |
| 2026-06-26 | Superseded candidate action identifier search | passed | No internal runtime candidate-action identifiers such as `load-sms-messages`, `load-active-pdp-contexts`, or `load-pdp-contexts` remained in current docs, source, or examples. |
| 2026-06-26 | `git status --short` | failed | This workspace copy does not expose a `.git` directory to the command environment, so a Git working-tree summary could not be produced from this checkout. |
| 2026-06-26 | Action/Input/candidate assistance documentation and implementation update | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.75`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `examples/presets/quectel.toml`, `examples/sequences/quectel.toml`, `examples/sequences/soracom.toml`, `src/lib.rs`, `src/cli.rs`, `src/sequences/builtin.rs`, `src/sequences/engine.rs`, `src/sequences/loader.rs`, `src/sequences/model.rs`, and `src/tui/mod.rs`. Product-known candidate assistance is limited to `sms-message` and `pdp-context`; Quectel socket state is an explicit add-on command instead of product-standard candidate assistance. |
| 2026-06-26 | `cargo fmt --check` | passed | Formatting is clean after the Action/Input/candidate assistance implementation. |
| 2026-06-26 | `cargo test` | passed | 232 unit tests and doc tests passed, including product-language gates, Sequence candidate extraction, TUI candidate actions, same-modal SMS candidate selection, add-on Sequence loading, and long confirmation input visibility. |
| 2026-06-26 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Action/Input/candidate assistance implementation. |
| 2026-06-26 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | CLI output shows product SMS candidate metadata as `sms-message`, TCP add-on PDP metadata as `pdp-context`, and Quectel socket connect ID as an editable default without product-standard socket candidate assistance. |
| 2026-06-26 | `cargo run -- preset list --preset-dir examples/presets` | passed | CLI output shows Quectel `pdp-contexts-quectel` and `socket-state-quectel` as explicitly loaded add-on commands with sensitive declared/effective risk. |
| 2026-06-26 | Rejected identifier search | passed | No superseded value-resolution identifiers remained in current docs, source, or examples. |
| 2026-06-25 | Superseded Sequence value-resolution implementation | passed | Historical implementation later replaced by the 0.4.75 Action/Input/candidate assistance model. |
| 2026-06-25 | `cargo test` | failed, then passed | Historical full-test run for the superseded value-resolution model; 232 unit tests and doc tests passed at that point. |
| 2026-06-25 | `cargo fmt --check` | failed, then passed | Historical rustfmt-only formatting check for the superseded value-resolution implementation. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Historical Clippy check for the superseded value-resolution implementation. |
| 2026-06-25 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | Historical CLI list check for the superseded value-resolution implementation. |
| 2026-06-25 | Response failed-before-response result marker implementation | passed | Updated `AGENTS.md`, `docs/SPEC.md` to version `0.4.73`, `docs/DEVELOPMENT.md`, this file, `src/lib.rs`, and `src/tui/mod.rs`. TUI Response now starts failed-before-response content with `Result: failed`, then the existing failure text, so failure is visible without relying on compact Status or color alone. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the Response failed-before-response result marker implementation. |
| 2026-06-25 | `cargo test sequence_output_origin_gate_keeps_analysis_separate_from_modem_output` | failed, then passed | Initial run failed because `AGENTS.md` used `Do not rely...` while the product-language gate required the explicit `must not rely on compact Status or color alone` wording. Updated `AGENTS.md` and reran the targeted test successfully. |
| 2026-06-25 | `cargo test failed_sequence_status_keeps_full_error_out_of_compact_status` | passed | TUI render-buffer coverage confirms Response contains `Result: failed` and the existing failure text, while compact Status keeps only concise state context. |
| 2026-06-25 | `cargo test` | passed | 230 unit tests and doc tests passed after the Response failed-before-response result marker implementation. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Response failed-before-response result marker implementation. |
| 2026-06-25 | Compact Status free-form detail removal and recurrence-prevention implementation | passed | Updated `AGENTS.md`, `docs/SPEC.md` to version `0.4.72`, `docs/DEVELOPMENT.md`, this file, `_local/agent-history/incident-log.md`, `src/lib.rs`, and `src/tui/mod.rs`. TUI compact Status no longer renders free-form `Detail:` strings from execution failures; completed/failed states use typed concise `Result:` summaries, and full Sequence failure text remains in Response. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the compact Status free-form detail removal. |
| 2026-06-25 | `cargo test` | passed | 230 unit tests and doc tests passed, including product-language coverage for the `Detail:` Status prohibition and TUI render-buffer coverage that keeps Sequence expectation failure detail out of compact Status while preserving it in Response. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the compact Status free-form detail removal. |
| 2026-06-25 | Sequence confirmation phase-critical input visibility implementation | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.71`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, this file, `src/lib.rs`, and `src/tui/mod.rs`. TUI `Run Sequence` confirmation now keeps the risk instruction and current `Input:` line visible; long TCP Sequence review detail is summarized before it hides the confirmation action. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the Sequence confirmation input visibility implementation. |
| 2026-06-25 | `cargo test` | passed | 229 unit tests and doc tests passed, including product-language coverage for Sequence confirmation visibility and TUI render-buffer coverage that keeps `Type \`write\` to run` and `Input:` visible in long TCP Sequence confirmation. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence confirmation input visibility implementation. |
| 2026-06-25 | Sequence candidate provenance and no-hidden-acquisition implementation | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.70`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `src/lib.rs`, and `src/tui/mod.rs`. TUI SMS candidate rows now carry same-session source and acquisition metadata, show total count, state that opening the modal performs no modem read, and keep the highlighted candidate visible when more rows exist than fit in the modal. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the Sequence candidate provenance and no-hidden-acquisition implementation. |
| 2026-06-25 | `cargo test` | passed | 228 unit tests and doc tests passed, including candidate provenance/count display, no-hidden-acquisition wording, same-session source requirements, and overflow candidate-window coverage. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence candidate provenance and no-hidden-acquisition implementation. |
| 2026-06-25 | Sequence `source=select` same-modal candidate-selection implementation | passed | Updated `README.md`, `docs/SPEC.md` to version `0.4.69`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `src/sequences/engine.rs`, `src/tui/mod.rs`, and `src/cli.rs`. Sequence execution now returns structured SMS candidates; TUI stores candidates from Sequence and direct `AT+CMGL` command results; `sms-read-message` and `sms-reply-check` index input shows candidates in the `Run Sequence` modal and lets the operator select one without leaving it. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the Sequence `source=select` same-modal candidate-selection implementation. |
| 2026-06-25 | `cargo test` | passed | 227 unit tests and doc tests passed, including SMS candidate extraction, TUI same-modal candidate selection, candidate refresh from `sms-receive-check`, candidate refresh from direct `AT+CMGL`, and product-language coverage for `source=select`. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence `source=select` same-modal candidate-selection implementation. |
| 2026-06-25 | Sequence `source=select` recurrence-prevention update | passed | Updated project-local gates and specs so `source=select` cannot be considered complete with hint-only TUI behavior when candidates are available. Added test coverage that requires same-modal candidate-selection rules in `AGENTS.md`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, and `docs/SPEC.md`. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the Sequence `source=select` recurrence-prevention update. |
| 2026-06-25 | `cargo test` | passed | 224 unit tests and doc tests passed, including product-language coverage for same-modal candidate-selection requirements on `source=select` Sequence values. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence `source=select` recurrence-prevention update. |
| 2026-06-25 | Sequence input value-resolution documentation and implementation update | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.67`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `examples/sequences/quectel.toml`, `examples/sequences/soracom.toml`, `src/app/errors.rs`, `src/cli.rs`, `src/sequences/builtin.rs`, `src/sequences/engine.rs`, `src/sequences/loader.rs`, `src/sequences/model.rs`, and `src/tui/mod.rs`. Sequence params now support `default`, `source`, and `hint`; TUI Sequence input/review shows current values with source/default/hint context; CLI list and missing-parameter errors use the same metadata. |
| 2026-06-25 | `cargo fmt --check` | passed | Formatting is clean after the Sequence value-resolution update. |
| 2026-06-25 | `cargo test` | passed | 223 unit tests and doc tests passed, including Sequence default binding, missing-value hint output, TOML metadata parsing, and TUI render-buffer coverage for Sequence values/defaults/hints. |
| 2026-06-25 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence value-resolution update. |
| 2026-06-25 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | Explicit example loading shows TCP required values with source/default context such as `context_id=1(modem)`, `connect_id=0(default)`, and `read_length=1500(default)`. |
| 2026-06-25 | CLI Sequence missing-value checks | passed | `sms-reply-check` missing `index` reports `source=select` with the `sms-receive-check` / `AT+CMGL` reference path. Quectel and SORACOM TCP examples bind `context_id`, `connect_id`, and `read_length` defaults and stop on missing `payload` with a value-specific hint before transport access. |
| 2026-06-25 | Sequence value-resolution residual wording search | passed | Current docs and runtime source no longer contain the old Sequence input status wording, old required-value wording, or the old runtime modal heading. The remaining unresolved-value sentinel match is the normative prohibition in `docs/SPEC.md`. |
| 2026-06-24 | Sequence transcript spacing and TUI balanced layout documentation and implementation update | passed | Updated `AGENTS.md`, `README.md`, `docs/SPEC.md` to version `0.4.66`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `src/lib.rs`, `src/sequences/engine.rs`, and `src/tui/mod.rs`. Sequence text transcripts now separate origin sections with single blank lines and no decorative divider lines. The normal TUI layout keeps the approved topology while using a stable balanced top/bottom split, with odd-height extra space kept in the bottom Response/Logs review area. |
| 2026-06-24 | `cargo fmt --check` | passed | Formatting is clean after the Sequence transcript spacing and TUI balanced layout update. |
| 2026-06-24 | `cargo test` | passed | 220 unit tests and doc tests passed, including Sequence transcript blank-line section separation, product-language gate coverage, TUI balanced layout height coverage, and constrained running Status timeout feedback coverage. |
| 2026-06-24 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence transcript spacing and TUI balanced layout update. |
| 2026-06-24 | Sequence output and TUI layout residual search | passed | Runtime source still has no generated `Evidence:`, `Decoded SMS body`, `TCP receive evidence`, `masked_evidence`, or `raw_evidence`; remaining matches are negative assertions or product-language gate constants. TUI root layout uses computed balanced top-band height; the remaining `Constraint::Percentage(55)` is the unchanged Response/Logs horizontal split. |
| 2026-06-24 | Sequence output origin documentation and implementation correction | passed | Updated `AGENTS.md`, `docs/SPEC.md` to version `0.4.65`, `docs/DEVELOPMENT.md`, `docs/PRESETS.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, this file, `_local/agent-history/incident-log.md`, `src/lib.rs`, `src/sequences/engine.rs`, and `src/cli.rs`. Sequence text transcripts now separate `Command:`, `Payload:`, `Modem response:`, `Decoded SMS:`, `Analysis:`, `Notes:`, and `Result:`; CLI JSON uses step `analysis` instead of `evidence`; normal output no longer renders a literal `Evidence:` prefix. |
| 2026-06-24 | `cargo fmt --check` | passed | Formatting is clean after the Sequence output origin correction. |
| 2026-06-24 | `cargo test` | passed | 220 unit tests and doc tests passed, including Sequence transcript origin sections, SMS decoded-body separation, JSON `analysis`, and the project-local Sequence Output Origin Gate product-language test. |
| 2026-06-24 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the Sequence output origin correction. |
| 2026-06-24 | Sequence output residual search | passed | Runtime source no longer generates `Evidence:`, `Decoded SMS body`, `TCP receive evidence`, `masked_evidence`, `raw_evidence`, or JSON `evidence`; remaining matches are negative assertions, product gates, or the Sequence TOML definition field. |
| 2026-06-24 | TUI compact Status non-state explanation recurrence prevention and implementation correction | passed | Updated `AGENTS.md`, `docs/SPEC.md` to version `0.4.64`, `docs/DEVELOPMENT.md`, this file, `_local/agent-history/incident-log.md`, `src/lib.rs`, and `src/tui/mod.rs`. Compact Status no longer renders persistent `Copy:` explanations for foreground Response or viewed-log states. Added project gates and tests so Status wording changes must keep action semantics, help, Sequence summaries, Evidence/analysis notes, and copy/save behavior descriptions out of Status. |
| 2026-06-24 | `cargo fmt --check` | passed | Formatting is clean after the compact Status non-state explanation correction. |
| 2026-06-24 | `cargo test tui::tests` | passed | 82 TUI tests passed, including negative render-buffer coverage for `Copy:` explanations in unmasked foreground Response and viewed-log Status states. |
| 2026-06-24 | `cargo test` | passed | 219 unit tests and doc tests passed, including the project-local TUI Status Content Gate product-language test. |
| 2026-06-24 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the compact Status non-state explanation correction. |
| 2026-06-24 | Compact Status non-state explanation residual search | passed | Runtime source no longer renders `Copy: Copy response uses`, `visible Response body`, or `displayed masked log body`; remaining matches are negative assertions or normative prohibition text. |
| 2026-06-24 | TUI compact Status Sequence summary documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.63`, `docs/PRESETS.md`, `docs/OPEN-QUESTIONS.md`, `docs/DEVELOPMENT.md`, this file, and `src/tui/mod.rs`. Compact Status no longer renders the Sequence `summary` field as selected, active, or completed Sequence context; Sequence summaries remain available in executable rows, the `Run Sequence` modal, search matching, and approved detail/help surfaces. |
| 2026-06-24 | `cargo fmt --check` | passed | Formatting is clean after the compact Status Sequence summary update. |
| 2026-06-24 | `cargo test tui::tests` | passed | 80 TUI tests passed, including selected and completed Sequence Status coverage that rejects `Summary:` and the long `sms-receive-check` purpose sentence in the Status pane. |
| 2026-06-24 | `cargo test` | passed | 216 unit tests and doc tests passed after the compact Status Sequence summary update. |
| 2026-06-24 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the compact Status Sequence summary update. |
| 2026-06-24 | Compact Status Sequence summary residual search | passed | Confirmed the updated docs specify the compact Status prohibition and allowed summary placements. Source search found the remaining `Summary:` rendering in the `Run Sequence` modal, not in the compact Status selected, active, or completed Sequence paths. |
| 2026-06-23 | Product wording, SMS decode display, masking documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.62`, `README.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/OPEN-QUESTIONS.md`, and this file. Updated CLI/TUI wording so CLI list output uses `Product presets`, TUI Controls uses `AT command`, nonexistent `y` copy hints are removed, and SMS foreground Sequence transcripts show decoded bodies instead of UCS2 hex when decoding is supported. |
| 2026-06-23 | `cargo fmt --check` | passed | Formatting is clean after the product wording and SMS decoded foreground display update. |
| 2026-06-23 | `cargo test` | passed | 214 unit tests and doc tests passed, including CLI help wording, Product presets list label, TUI default-source-label absence, TUI Controls wording, SMS read decode/masking, SMS receive decode/masking, SMS reply sender derivation, TCP evidence, raw diagnostic export separation, and product language gates. |
| 2026-06-23 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the product wording and SMS decoded foreground display update. |
| 2026-06-23 | `cargo run -- preset list` and `cargo run -- preset list --preset-dir examples/presets` | passed | CLI preset list uses `Product presets` for product rows and uses TOML titles such as `Quectel commands` and `SORACOM commands` for explicitly loaded file preset rows. |
| 2026-06-23 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | Product SMS Sequences remain under `Product Sequences`; explicitly loaded Quectel and SORACOM examples use their TOML titles and keep TCP parameters visible. |
| 2026-06-23 | `cargo run -- --help`, `cargo run -- tui --help`, and `cargo run -- sequence run sms-read-message --help` | passed | Help output now describes primary commands, USB selectors, location flags, `--no-mask`, raw diagnostic export flags, risk acknowledgement, and repeated `--param NAME=VALUE`. |
| 2026-06-23 | Residual wording search over current docs and source | passed | Current README, SPEC, PRESETS, SAFETY, TROUBLESHOOTING, OPEN-QUESTIONS, DEVELOPMENT, CLI, and TUI source no longer contain user-facing `Ad-hoc AT`, `Copy: y`, `raw reveal`, or `Built-in presets`; remaining `Built-in presets` matches are negative assertions only, and remaining lowercase `ad-hoc` matches are internal execution IDs. |
| 2026-06-23 | TUI output masking documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.61`, `docs/OPEN-QUESTIONS.md`, `README.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/DEVELOPMENT.md`, and this file. Replaced current-response raw reveal with TUI session output masking, `atctl tui --no-mask`, `Output masking on/off`, exact `unmask` acknowledgement, visible-copy behavior, and masked saved/log behavior. |
| 2026-06-23 | `cargo fmt --check` | passed | Formatting is clean after the TUI output masking update. |
| 2026-06-23 | `cargo test` | passed | 211 unit tests and doc tests passed, including TUI output masking acknowledgement, cancel, session persistence, copy behavior, saved Response masking, saved log masking, raw capture separation, and CLI `tui --no-mask` parsing. |
| 2026-06-23 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the TUI output masking update. |
| 2026-06-23 | `cargo run -- tui --help` | passed | Help now lists `--no-mask` as `Start the TUI session with output masking off`. |
| 2026-06-23 | `cargo run -- tui --snapshot` | not applicable | The current CLI rejects `--snapshot` as an unexpected argument; render-buffer coverage is provided by TUI unit tests. |
| 2026-06-23 | `cargo fmt --check` | passed | Formatting is clean after SMS decode, reply-by-index, active Sequence input/review, and TCP evidence updates. |
| 2026-06-23 | `cargo test` | passed | 210 unit tests passed, including SMS UCS2 decode and masking, SMS reply sender derivation, Quectel TCP counter/QIRD evidence, CLI Sequence list output, TUI Sequence input/review modal, raw capture, and product language gates. |
| 2026-06-23 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No Clippy warnings after the SMS/TCP Sequence evidence update. |
| 2026-06-23 | `cargo run -- sequence list` | passed | Product Sequence list shows `sms-send-check`, `sms-receive-check`, `sms-read-message`, and `sms-reply-check`; `sms-reply-check` requires `index,message(sensitive)`. |
| 2026-06-23 | `cargo run -- sequence list --sequence-dir examples/sequences` | passed | Explicit example loading shows Quectel and SORACOM Sequence sets in addition to product SMS Sequences, with TCP payload and read-length parameters intact. |
| 2026-06-23 | `cargo run -- sequence run sms-send-check --help` and SORACOM Sequence help | passed | Static Sequence run help shows repeated `--param NAME=VALUE` usage plus both SMS and SORACOM example invocations. |
| 2026-06-23 | Sequence specification documentation update and consistency search | passed | Updated `docs/SPEC.md` to version `0.4.53`, `README.md`, `docs/PRESETS.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, `docs/DEVELOPMENT.md`, `docs/OPEN-QUESTIONS.md`, and `docs/IMPLEMENTATION-STATUS.md` for OQ-023 Sequence design. Added Checkpoint 12.6 before release work and confirmed current docs contain Sequence terms, planned CLI/TUI paths, user/repository definition paths, and Quectel/SMS evidence rules. |
| 2026-06-22 | TUI running Status progress noun label and width fallback documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.52`, `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/IMPLEMENTATION-STATUS.md`, and TUI implementation/tests so the normal compact Status progress label includes the `Timeout` noun, such as `Timeout 33/180s left 147s`, and only falls back to shorter labels when the available width cannot fit it. |
| 2026-06-22 | `cargo fmt --check` | passed | Formatting check after Status progress noun label and width fallback update. |
| 2026-06-22 | `cargo test tui::tests` | passed | 74 TUI tests passed, including the `Timeout` noun label and width fallback coverage for running Status progress. |
| 2026-06-22 | `cargo test` | passed | 184 unit tests and doc tests passed after Status progress noun label and width fallback update. |
| 2026-06-22 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after Status progress noun label and width fallback update. |
| 2026-06-22 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | TUI started and restored the terminal on quit. This smoke check did not execute a real modem command; running-progress label rendering and fallback are covered by TUI tests. |
| 2026-06-22 | TUI running Status progress label documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.51`, `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/IMPLEMENTATION-STATUS.md`, and TUI implementation/tests so the compact Status progress label uses short wording such as `33s / 180s, left 147s` instead of verbose `Elapsed ... remaining ...` text that can look truncated. |
| 2026-06-22 | `cargo fmt --check` | passed | Formatting check after compact running Status progress label update. |
| 2026-06-22 | `cargo test tui::tests` | passed | 73 TUI tests passed, including the shortened running Status progress label and absence of the verbose `Elapsed` / `remaining` wording. |
| 2026-06-22 | `cargo test` | passed | 183 unit tests and doc tests passed after compact running Status progress label update. |
| 2026-06-22 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after compact running Status progress label update. |
| 2026-06-22 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | TUI started and restored the terminal on quit. This smoke check did not execute a real modem command; running-progress label rendering is covered by TUI tests. |
| 2026-06-22 | TUI Controls action feedback and Status progress documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.50`, `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/IMPLEMENTATION-STATUS.md`, and TUI implementation/tests so Controls rows read as stable actions rather than a dense status table, Controls action results appear as nearby feedback without changing row labels, and running-command Status progress uses a separated elapsed/remaining line plus compact progress bar. |
| 2026-06-22 | `cargo fmt --check` | passed | Formatting check after TUI Controls action feedback and Status progress update. |
| 2026-06-22 | `cargo test tui::tests` | passed | 73 TUI tests passed, including stable Controls action labels, nearby Controls feedback, copy feedback without row-label mutation, and separated running Status progress. |
| 2026-06-22 | `cargo test` | passed | 183 unit tests and doc tests passed after TUI Controls action feedback and Status progress update. |
| 2026-06-22 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after TUI Controls action feedback and Status progress update. |
| 2026-06-22 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | TUI started with Controls action rows, no initial non-focused Controls feedback consuming row space, and terminal restoration on quit. |
| 2026-06-22 | TUI Response copy feedback documentation and implementation update | superseded | Historical superseded check for `docs/SPEC.md` version `0.4.49`; the current Controls feedback behavior is governed by version `0.4.60`, which keeps action labels stable and shows copy-request feedback as nearby Controls feedback rather than as a status-table row value. |
| 2026-06-22 | `cargo fmt --check` | passed | Formatting check after TUI Response copy feedback update. |
| 2026-06-22 | `cargo test tui::tests` | passed | 71 TUI tests passed, including Response copy success/failure feedback and Controls row detail coverage. |
| 2026-06-22 | `cargo test` | passed | 180 unit tests and doc tests passed after TUI Response copy feedback update. |
| 2026-06-22 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after TUI Response copy feedback update. |
| 2026-06-21 | OQ-022 aligned-band TUI layout documentation and implementation update | passed | Updated `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/SPEC.md` to version `0.4.48`, `docs/IMPLEMENTATION-STATUS.md`, and TUI implementation/tests so the canonical layout is Devices/Status, Categories, and Commands in the top band, Controls, Response, and Logs in the bottom band, and Devices/Status/Controls use a compact 30-34 column utility width while Commands/Response/Logs receive the remaining width. Masked-log Response range titles were shortened so line range and top/bottom state remain visible in the narrower result pane. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after aligned-band TUI layout update. |
| 2026-06-21 | `cargo test tui::tests` | passed | 70 TUI tests passed, including aligned top/bottom bands, compact utility width, Commands -> Controls focus order, modal Help behavior, and masked-log Response range title coverage. |
| 2026-06-21 | `cargo test` | passed | 179 unit tests and doc tests passed after aligned-band TUI layout update. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after aligned-band TUI layout update. |
| 2026-06-21 | `cargo run -- tui --theme no-color`, then `?`, `q`, `q` in an agent PTY | passed | TUI started with aligned top/bottom bands, compact left utility width, wider Commands/Response/Logs panes, Help opened and closed with `q`, and the second `q` restored the terminal. This agent environment had no matching USB operation target visible. |
| 2026-06-21 | OQ-022 TUI layout, Controls, and Help correction | passed | Updated `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/SPEC.md` to version `0.4.47`, `docs/IMPLEMENTATION-STATUS.md`, and TUI implementation/tests so the canonical layout is Devices/Status/Controls in the left utility column, normal focus order is Categories -> Commands -> Controls -> Response -> Logs -> Devices, Controls keeps stable visible rows with compact availability state, and Help omits pane-architecture prose. |
| 2026-06-21 | Residual TUI wording search | passed | Confirmed implemented Help no longer contains `Primary flow` or Controls/Devices/Logs inventory prose; remaining `Primary flow` matches are only explicit prohibition/documentation rationale. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after OQ-022 layout, Controls, and Help correction. |
| 2026-06-21 | `cargo test tui::tests` | passed | 70 TUI tests passed, including Controls left utility column placement, Commands -> Controls focus order, modal Help behavior, and compact raw-visible label coverage. |
| 2026-06-21 | `cargo test` | passed | 179 unit tests and doc tests passed after OQ-022 layout, Controls, and Help correction. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after OQ-022 layout, Controls, and Help correction. |
| 2026-06-21 | `cargo run -- tui --theme no-color`, then `?`, `q`, `q` in an agent PTY | passed | TUI started with Devices/Status/Controls in the left utility column, Categories/Commands adjacent, Response/Logs below, compact Controls state visible at 80 columns, Help without `Primary flow` or pane inventory prose, and terminal restoration on quit. |
| 2026-06-21 | OQ-022 TUI shortcut reduction documentation and implementation update | passed | Updated `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/SPEC.md`, `docs/IMPLEMENTATION-STATUS.md`, and TUI implementation/tests so secondary actions use Controls/Devices pane rows instead of many global letter shortcuts. |
| 2026-06-21 | OQ-022 Controls pane placement correction | passed | Historical superseded check for `docs/SPEC.md` version `0.4.46`; the current OQ-022 layout plus OQ-023 Sequence extension is governed by version `0.4.60`. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after Controls pane placement correction. |
| 2026-06-21 | `cargo test tui::tests` | passed | 70 TUI tests passed, including the Controls pane placement regression test and the existing focus-cycle test. |
| 2026-06-21 | `cargo test` | passed | 179 unit tests and doc tests passed after Controls pane placement correction. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after Controls pane placement correction. |
| 2026-06-21 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | TUI started with Categories and Commands adjacent in the top workflow row, Controls below Logs in the lower-right area, and terminal restoration on quit. |
| 2026-06-21 | Residual active shortcut wording search | passed | Current user-facing docs no longer present `a`, `t`, `u`, or `y` as active TUI shortcuts; remaining old key mentions are explicitly historical checkpoint or verification-log records. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after TUI Controls pane and shortcut reduction implementation. |
| 2026-06-21 | `cargo test tui::tests` | passed | 69 TUI tests passed, including Controls actions, Devices full-USB row activation, reduced footer hints, and modal help behavior. |
| 2026-06-21 | `cargo test` | passed | 178 unit tests and doc tests passed after TUI shortcut reduction implementation. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after TUI shortcut reduction implementation. |
| 2026-06-21 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | TUI started with the Controls pane visible and restored the terminal on quit. |
| 2026-06-21 | Built-in preset naming documentation and implementation update | passed | Updated `docs/SPEC.md` to version `0.4.44`, aligned `docs/PRESETS.md`, `docs/TROUBLESHOOTING.md`, `docs/IMPLEMENTATION-STATUS.md`, built-in preset definitions, and TUI tests with the approved clearer preset names. |
| 2026-06-21 | Residual old preset name search | passed | No old preset identifiers remained for the renamed built-ins; remaining `operator`, `revision`, and `info` matches are ordinary prose or command response text, not preset names. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after built-in preset naming update. |
| 2026-06-21 | `cargo test` | passed | 177 unit tests and doc tests passed after built-in preset naming update. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after built-in preset naming update. |
| 2026-06-21 | `cargo run -- preset list` | passed | Built-in list shows the approved names including `modem-info`, `current-operator`, `signal-quality`, `pdp-auth-settings`, `enable-verbose-errors`, `modem-functionality`, `restart-modem`, and `disable-command-echo`. |
| 2026-06-21 | OQ-021 raw diagnostic export documentation and implementation update | passed | Updated `AGENTS.md`, `README.md`, `docs/OPEN-QUESTIONS.md`, `docs/SPEC.md`, `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, and `docs/IMPLEMENTATION-STATUS.md`; implemented explicit raw export across send, preset run, TUI, and bridge. |
| 2026-06-21 | `cargo run -- send --help` | passed | Help shows `--raw-log-file <PATH>` and `--raw-log-ack <raw-log>`; old boolean `--raw-log` is not exposed. |
| 2026-06-21 | `cargo run -- preset run --help` | passed | Help shows `--raw-log-file <PATH>` and `--raw-log-ack <raw-log>` for preset execution. |
| 2026-06-21 | `cargo run -- bridge --help` | passed | Help shows `--raw-log-file <PATH>` and `--raw-log-ack <raw-log>` for PTY bridge execution. |
| 2026-06-21 | `cargo test` | passed | 171 unit tests and doc tests passed, including raw export writer, CLI raw export, bridge raw export, and TUI raw capture coverage. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after OQ-021 raw diagnostic export implementation. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after OQ-021 raw diagnostic export implementation. |
| 2026-06-21 | Raw diagnostic export follow-up fix | passed | Fixed CLI one-shot raw export so USB target selection failures do not create header-only rawlog files, and command-send/read failures write `transport_error` events. Updated troubleshooting example to include explicit device selection. |
| 2026-06-21 | `cargo test raw_log` | passed | 10 raw-log focused tests passed, including no-file-on-open-failure and transport-error event coverage. |
| 2026-06-21 | `cargo test` | passed | 174 unit tests and doc tests passed after the raw diagnostic export follow-up fix. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after raw diagnostic export follow-up fix. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after raw diagnostic export follow-up fix. |
| 2026-06-20 | `cargo fmt --check` | passed | Formatting is clean after adopting additional standard and Quectel AT command presets. |
| 2026-06-20 | `cargo test at::risk` | passed | 12 risk-classification tests passed, including `AT+CEER`, `AT+CMEE?`, `AT+CMEE=2`, `AT+CPAS`, `AT+QPINC?`, and `AT+QMBNCFG="List"`. |
| 2026-06-20 | `cargo test presets::` | passed | 14 preset tests passed, including built-in order, vendor/carrier exclusion from built-ins, and repository file preset loading. |
| 2026-06-20 | `cargo test` | passed | 160 unit tests and doc tests passed after the additional preset and risk-classification changes. |
| 2026-06-20 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings after the additional preset and risk-classification changes. |
| 2026-06-20 | `cargo run -- preset list` | passed | Built-in list includes `extended-error-report`, `error-reporting-status`, `enable-verbose-errors`, and `modem-activity-status` with expected risk levels. |
| 2026-06-20 | `cargo run -- preset list --preset-dir examples/presets` | passed | Example preset list includes Quectel `sim-init-status-quectel`, `pin-retries-quectel`, `network-name-quectel`, `network-time-quectel`, and `mbn-list-quectel`. |
| 2026-06-20 | `cargo run -- send AT+CMEE=2 --yes` | expected failure | Rejected before USB access because `--risk-ack write` is required. |
| 2026-06-20 | `cargo fmt --check` | passed | Formatting is clean after Checkpoint 12.5 changes. |
| 2026-06-20 | `cargo test at::risk` | passed | 10 risk-classification tests passed, including `AT+CFUN=...` and `AT+QPOWD` dangerous acknowledgement. |
| 2026-06-20 | `cargo test presets::` | passed | 14 preset tests passed, including built-in order and repository file preset loading. |
| 2026-06-20 | `cargo test tui::tests` | passed | 67 TUI tests passed, including visible dangerous presets with confirmation requirement. |
| 2026-06-20 | `cargo test` | passed | 158 unit tests and doc tests passed. |
| 2026-06-20 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No clippy warnings. |
| 2026-06-20 | `cargo run -- preset list` | passed | Built-in list includes `modem-functionality`, `set-modem-minimum-functionality`, `set-modem-full-functionality`, and `restart-modem`. |
| 2026-06-20 | `cargo run -- preset list --preset-dir examples/presets` | passed | Example preset list includes Quectel `power-down-quectel` as dangerous and keeps it out of built-ins. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after scaffold. |
| 2026-06-17 | `cargo test` | passed | 17 unit tests passed; dependency fetch required network approval. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings. |
| 2026-06-17 | `cargo run -- --help` | passed | CLI help renders. |
| 2026-06-17 | `cargo run -- config path` | passed | Non-hardware command prints default config path. |
| 2026-06-17 | `cargo run -- send AT+CFUN=0 --yes` | passed | Rejected before USB access because `--risk-ack dangerous` is required. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after Checkpoint 2 implementation. |
| 2026-06-17 | `cargo test` | passed | 24 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 2 implementation. |
| 2026-06-17 | `cargo run -- devices` | passed | Historical Checkpoint 2 run in this agent environment; no USB devices were visible. |
| 2026-06-17 | `cargo run -- inspect --interface 2 --bulk-in 0x85 --bulk-out 0x04` | passed | Historical Checkpoint 2 run; manual override was reported separately and no USB devices were visible. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after Checkpoint 3 implementation. |
| 2026-06-17 | `cargo test` | passed | 34 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 3 implementation. |
| 2026-06-17 | `cargo run -- send AT --vid 0x0000 --pid 0x0000` | expected failure | Stops before sending because no matching USB device exists. |
| 2026-06-17 | `cargo run -- send AT+CFUN=0 --yes` | expected failure | Rejected before USB access because `--risk-ack dangerous` is required. |
| 2026-06-17 | `cargo run -- devices --vid 0x2c7c --pid 0x0125` | passed | User-run real Onyx check: device listed as Quectel EG25-G / SORACOM Onyx, bus 1, address 3. |
| 2026-06-17 | `cargo run -- inspect --vid 0x2c7c --pid 0x0125` | passed | User-run real Onyx check: five descriptor-shape bulk IN/OUT candidates reported. |
| 2026-06-17 | `cargo run -- send AT --vid 0x2c7c --pid 0x0125` | passed | User-run real Onyx check: response was `AT`, `OK`. |
| 2026-06-17 | `cargo run -- send ATI --vid 0x2c7c --pid 0x0125` | passed | User-run real Onyx check: Quectel EG25 revision EG25GGBR07A08M2G returned `OK`. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after Checkpoint 4 implementation. |
| 2026-06-17 | `cargo test` | passed | 42 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 4 implementation. |
| 2026-06-17 | `cargo run -- send AT --risk-ack dangerous --vid 0x0000 --pid 0x0000` | expected failure | Rejected before USB access because classified risk `safe` does not match acknowledged risk `dangerous`. |
| 2026-06-17 | `cargo run -- send ATE0 --vid 0x0000 --pid 0x0000` | expected failure | Rejected before USB access because write command requires confirmation. |
| 2026-06-17 | `cargo run -- send AT+CFUN=0 --yes --risk-ack dangerous --vid 0x0000 --pid 0x0000` | expected failure | Safety acknowledgement accepted, then stops before sending because no matching USB device exists. |
| 2026-06-17 | `cargo run -- send AT+CFUN=0 --yes` | expected failure | Rejected before USB access because `--risk-ack dangerous` is required. |
| 2026-06-17 | `cargo fmt --check` | passed | User-run Checkpoint 4 confirmation. |
| 2026-06-17 | `cargo test` | passed | User-run Checkpoint 4 confirmation: 42 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | User-run Checkpoint 4 confirmation. |
| 2026-06-17 | `cargo run -- send AT --risk-ack dangerous --vid 0x0000 --pid 0x0000` | expected failure | User-run Checkpoint 4 confirmation: classified `safe` did not match acknowledged `dangerous`. |
| 2026-06-17 | `cargo run -- send AT+CFUN=0 --yes` | expected failure | User-run Checkpoint 4 confirmation: matching `--risk-ack dangerous` is required. |
| 2026-06-17 | `cargo run -- send AT+CFUN=0 --yes --risk-ack dangerous --vid 0x0000 --pid 0x0000` | expected failure | User-run Checkpoint 4 confirmation: safety acknowledgement accepted, then fake VID/PID stopped before device access. |
| 2026-06-17 | `cargo run -- send ATE0 --vid 0x0000 --pid 0x0000` with `write` | expected failure | User-run Checkpoint 4 confirmation: prompt accepted `write`, then fake VID/PID stopped before device access. |
| 2026-06-17 | `cargo run -- send ATE0 --vid 0x0000 --pid 0x0000` with `abc` | expected failure | User-run Checkpoint 4 confirmation: prompt rejected non-matching confirmation input. |
| 2026-06-17 | `sed -n '/## Final-Phase Release and Homebrew Workflow Plan/,/## Release Blocking Decisions/p' docs/PACKAGING.md` | passed | Release and Homebrew notes are retained as final-phase guidance, not an active checkpoint. |
| 2026-06-17 | `rg -n "Checkpoint: 5|Status: in progress|deferred - final phase|Final-Phase" docs/IMPLEMENTATION-STATUS.md docs/PACKAGING.md` | passed | Checkpoint 5 was corrected back to app-feature work and release/Homebrew work was deferred to the final phase. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after Checkpoint 5 implementation. |
| 2026-06-17 | `cargo test` | passed | 50 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 5 implementation. |
| 2026-06-17 | `cargo run -- preset list` | passed | Built-in presets list with risk, categories, and command output. |
| 2026-06-17 | `cargo run -- preset run modem-response --vid 0x0000 --pid 0x0000` | expected failure | Preset lookup and safety validation succeeded, then fake VID/PID stopped before device access. |
| 2026-06-17 | historical `XDG_CONFIG_HOME=<temp> cargo run -- preset list` | passed | Historical temporary user preset `custom-modem-response` was loaded from `presets.toml`; superseded by the current explicit add-on loading boundary. |
| 2026-06-17 | `cargo run -- preset run set-soracom-apn-cid1 --yes` | expected failure | Write preset rejected before USB access because `--risk-ack write` is required. |
| 2026-06-17 | `cargo run -- preset run set-soracom-apn-cid1 --yes --risk-ack write --vid 0x0000 --pid 0x0000` | expected failure | Matching risk acknowledgement accepted, then fake VID/PID stopped before device access. |
| 2026-06-17 | `cargo run -- preset run modem-response --vid 0x2c7c --pid 0x0125` | passed | User-run Checkpoint 5 Onyx confirmation: response was `AT`, `OK`. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after Checkpoint 6 implementation. |
| 2026-06-17 | `cargo test` | passed | 54 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 6 implementation. |
| 2026-06-17 | `XDG_STATE_HOME=<temp> cargo run -- logs list` | passed | Printed `No logs found.` and did not create log files. |
| 2026-06-17 | historical `--raw-log` placeholder rejection check | expected failure | Superseded by the resolved OQ-021 `--raw-log-file <PATH> --raw-log-ack raw-log` design. |
| 2026-06-17 | `XDG_STATE_HOME=<temp> cargo run -- preset run modem-response --vid 0x2c7c --pid 0x0125` | passed | User-run Checkpoint 6 Onyx logging confirmation: response was `AT`, `OK`. |
| 2026-06-17 | `XDG_STATE_HOME=<temp> cargo run -- logs list` | passed | User-run Checkpoint 6 Onyx logging confirmation: history and session log paths were listed. |
| 2026-06-17 | `find <temp>/atctl -type f -print` | passed | User-run Checkpoint 6 Onyx logging confirmation: one `history.jsonl` and one `.session.log` file were present. |
| 2026-06-17 | `sed -n '1,120p' <temp>/atctl/history.jsonl` | passed | History contained command metadata and no response body. |
| 2026-06-17 | `sed -n '1,200p' <temp>/atctl/logs/*.session.log` | passed | Session log contained masked metadata with `raw_log=false`; response was safe `AT` / `OK`. |
| 2026-06-17 | `ls -l <temp>/atctl/history.jsonl <temp>/atctl/logs/*.session.log` | passed | Generated files used `-rw-------` permissions. |
| 2026-06-17 | `cargo add ratatui crossterm` | passed | Added TUI dependencies; `ratatui` resolved to `0.29.0` because `0.30.1` requires Rust 1.88 while this repo declares Rust 1.85. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after Checkpoint 7 implementation. |
| 2026-06-17 | `cargo test` | passed | 59 unit tests passed. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 7 implementation. |
| 2026-06-17 | `cargo run -- tui` then `q` in an agent PTY | passed | TUI rendered required panes and exited successfully. |
| 2026-06-17 | `rg -n "name = \"crossterm\"|name = \"ratatui\"" Cargo.lock` | passed | Dependency graph uses one direct `crossterm` version and `ratatui 0.29.0`. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after unapproved non-colored TUI styling change, later reverted. |
| 2026-06-17 | `cargo test` | passed | 59 unit tests passed after unapproved non-colored TUI styling change, later reverted. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after unapproved non-colored TUI styling change, later reverted. |
| 2026-06-17 | `cargo fmt --check` | passed | Formatting check after restoring the user-approved colored TUI accents. |
| 2026-06-17 | `cargo test` | passed | 60 unit tests passed, including `tui::tests::approved_colored_accents_are_retained`. |
| 2026-06-17 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after restoring colored TUI accents and adding the regression test. |
| 2026-06-17 | `cargo run -- tui` then `q` in an agent PTY | passed | TUI started after the restoration and exited back to the terminal. |
| 2026-06-18 | Web research: W3C WCAG 2.2, W3C WAI contrast guidance, Ratatui color docs, `NO_COLOR` | passed | Added TUI color, contrast, theme, and opt-out requirements to `docs/SPEC.md`; current cyan/yellow baseline is not claimed as light/dark-complete. |
| 2026-06-18 | Progress plan update | passed | Inserted Checkpoint 8 for TUI visual accessibility and theme foundation; shifted TUI execution to Checkpoint 9, PTY to Checkpoint 10, and release/Homebrew to Checkpoint 11. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 8 implementation. |
| 2026-06-18 | `cargo test` | passed | 64 unit tests passed, including semantic style roles, non-color state affordances, approved colored baseline preservation, and `NO_COLOR` behavior. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 8 implementation. |
| 2026-06-18 | `printenv NO_COLOR` | passed | Agent environment had `NO_COLOR=1`, so inherited `cargo run -- tui` correctly used color opt-out behavior. |
| 2026-06-18 | `env -u NO_COLOR cargo run -- tui` then `q` in an agent PTY | passed | Color-enabled TUI started, emitted cyan/yellow foreground styling for the approved baseline, and exited successfully. |
| 2026-06-18 | `NO_COLOR=1 cargo run -- tui` then `q` in an agent PTY | passed | Foreground colors were suppressed while selection marker, risk labels, focus border, and quit behavior remained available. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 9 implementation. |
| 2026-06-18 | `cargo test` | passed | 69 unit tests passed, including TUI safe execution through a test executor, confirmation mismatch rejection, confirmation match execution, and dangerous command hiding, including invalid category fallback. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 9 implementation. |
| 2026-06-18 | `NO_COLOR=1 cargo run -- tui` with `Down`, `Down`, `Enter`, `abc`, `Enter`, `q` in an agent PTY | passed | Opened the `disable-command-echo` write-risk confirmation dialog, rejected non-matching confirmation input before sending, and restored the terminal on quit. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 9 user-review UI adjustments. |
| 2026-06-18 | `cargo test` | passed | 71 unit tests passed, including running-command context before transport execution and response-pane clear regression coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 9 user-review UI adjustments. |
| 2026-06-18 | `NO_COLOR=1 cargo run -- tui` then `q` in an agent PTY | passed | TUI rendered separate Status and History panes; Status showed selected command context `modem-response`, `AT`, and `safe`, then exited successfully. |
| 2026-06-18 | Progress plan update | passed | Inserted Checkpoint 10 for TUI risk styling and masked/raw reveal; shifted PTY bridge to Checkpoint 11 and release/Homebrew work to Checkpoint 12. |
| 2026-06-18 | Documentation alignment update | passed | Added Checkpoint 10 risk styling and masked/raw reveal requirements to `docs/SPEC.md`, added OQ-012 and OQ-013 to `docs/OPEN-QUESTIONS.md`, and updated `docs/SAFETY.md`, `docs/TROUBLESHOOTING.md`, and `README.md`. |
| 2026-06-18 | OQ-012 decision update | passed | Marked TUI risk visual differentiation as resolved and recorded approved dark/light/no-color theme behavior. |
| 2026-06-18 | OQ-013 decision update | passed | Marked TUI raw/unmasked reveal as resolved, recorded current-response-only reveal, exact `reveal` acknowledgement, visible masked/raw state, auto-clear behavior, and always-masked logs. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 10 implementation. |
| 2026-06-18 | `cargo test` | passed | 77 unit tests passed, including TUI theme parsing, approved dark/light palettes, selected-row risk cue preservation, raw reveal acknowledgement, cancel, mask toggle, and auto-clear behavior. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 10 implementation. |
| 2026-06-18 | `env -u NO_COLOR cargo run -- tui --theme dark` then `q` in an agent PTY | passed | Dark TUI started, emitted approved RGB dark palette and risk cues, then restored the terminal on quit. |
| 2026-06-18 | `env -u NO_COLOR cargo run -- tui --theme light` then `q` in an agent PTY | passed | Light TUI started, emitted approved RGB light palette and risk cues, then restored the terminal on quit. |
| 2026-06-18 | `cargo run -- tui --theme no-color` then `q` in an agent PTY | passed | No-color TUI started without foreground RGB color escapes while preserving labels, cues, selection marker, and bold focus/selection emphasis. |
| 2026-06-18 | `NO_COLOR=1 cargo run -- tui` then `q` in an agent PTY | passed | Omitted `--theme` respected `NO_COLOR` and used no-color output with risk labels and cues preserved. |
| 2026-06-18 | `NO_COLOR=1 cargo run -- tui --theme dark` then `q` in an agent PTY | passed | Explicit `--theme dark` overrode `NO_COLOR` for TUI rendering and emitted the approved RGB dark palette. |
| 2026-06-18 | Checkpoint 10 user-review response-pane fix | passed | Separated Status and Response responsibilities: Status owns command state and metadata, while Response prioritizes the actual current response body so raw values are not pushed below duplicated context. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 10 response-pane fix. |
| 2026-06-18 | `cargo test` | passed | 78 unit tests passed, including render-buffer coverage that raw visible response values appear without duplicated completed-command context. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 10 response-pane fix. |
| 2026-06-18 | Checkpoint 10 user-review response control-character fix | passed | Added Response rendering sanitization so modem CR and terminal control sequences cannot overwrite pane borders or stale text during raw reveal. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 10 response control-character fix. |
| 2026-06-18 | `cargo test` | passed | 80 unit tests passed, including response control-character sanitization and sanitized raw response render-buffer coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 10 response control-character fix. |
| 2026-06-18 | Checkpoint 10 user-review mask-state relevance fix | passed | Updated Status and Response so `Mask` state appears only when masked and raw response text differ; safe `modem-response` no longer shows `Mask: masked`. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 10 mask-state relevance fix. |
| 2026-06-18 | `cargo test` | passed | 81 unit tests passed, including safe-response coverage that `Mask: masked` is not rendered when no value is masked. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 10 mask-state relevance fix. |
| 2026-06-18 | Checkpoint 10 user-review ICCID padding mask fix | passed | Updated `+QCCID:` masking so trailing `F` padding is hidden in masked output while raw reveal and `--no-mask` can still show the modem-returned value. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 10 ICCID padding mask fix. |
| 2026-06-18 | `cargo test` | passed | 82 unit tests passed, including `+QCCID:` trailing `F` padding masking coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 10 ICCID padding mask fix. |
| 2026-06-18 | User review: Checkpoint 10 ICCID padding mask fix | passed | User confirmed the masked `+QCCID:` trailing `F` behavior is correctly fixed. |
| 2026-06-18 | Checkpoint 10 completion approval | passed | User approved Checkpoint 10 as complete and directed masked log viewing to be implemented as a separate checkpoint. |
| 2026-06-18 | Progress plan update | passed | Added Checkpoint 11 for TUI masked log viewer before PTY, shifted PTY bridge to Checkpoint 12, and shifted final release/Homebrew work to Checkpoint 13. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 11 implementation. |
| 2026-06-18 | `cargo test` | passed | 84 unit tests passed, including History selection and masked log content viewing coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 11 implementation. |
| 2026-06-18 | Checkpoint 11 user-review scroll and focus-order fix | passed | Added Response scrolling and revised focus cycling to interactive panes only. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 11 scroll and focus-order fix. |
| 2026-06-18 | `cargo test` | passed | 86 unit tests passed, including Response scroll and interactive-pane focus-cycle coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 11 scroll and focus-order fix. |
| 2026-06-18 | Checkpoint 11 Response log-view specification update | passed | Documented temporary masked log-view mode, Response-local line range, and read-only line numbers. |
| 2026-06-18 | Checkpoint 11 Response log-view implementation | passed | Implemented Response-local line range, read-only line numbers for masked log view, Response focus after opening logs, and removed detached `line N of M` status. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after Checkpoint 11 Response log-view update. |
| 2026-06-18 | `cargo test` | passed | 87 unit tests passed, including masked log-view line numbers, Response-local line range, and focus-after-open coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after Checkpoint 11 Response log-view update. |
| 2026-06-18 | Checkpoint 11 completion approval | passed | User approved Checkpoint 11 as complete after the masked log-view Response display update. |
| 2026-06-18 | Documentation consistency search for Checkpoint 11.5 terms | passed | Confirmed Checkpoint 11.5, file preset paths, `presets.d`, effective risk, ad-hoc input, and SMS readiness terms are present in the required documents. |
| 2026-06-18 | Documentation stale-term search for superseded preset/status wording | passed | Remaining matches are historical notes, negative requirements, or Checkpoint 11.5 review criteria rather than active contradictory requirements. |
| 2026-06-18 | Checkpoint 11.5 documentation approval | passed | User approved the documentation update as complete before Checkpoint 11.5 implementation started. |
| 2026-06-18 | Web research: crossterm clipboard docs, iTerm2 OSC 52 docs, xterm OSC control sequence references | passed | Confirmed Checkpoint 11.5 Response copy should use an explicit clipboard write path and not rely on terminal mouse selection of pane UI. |
| 2026-06-18 | `cargo test` | passed | 105 unit tests passed, including multi-file preset loading, repository pack loading through the drop-in loader, duplicate preset rejection, effective risk, TUI ad-hoc input, prompt-required SMS rejection, and Response copy payload behavior. |
| 2026-06-18 | historical `env XDG_CONFIG_HOME=<temp> cargo run -- preset list` | passed | Loaded `examples/presets/quectel.toml` and `examples/presets/soracom.toml` through a temporary `presets.d`; this XDG auto-load path is superseded by the current explicit `--preset-dir` boundary. |
| 2026-06-18 | historical `env XDG_CONFIG_HOME=<temp-with-duplicate> cargo run -- preset list` | expected failure | Duplicate `modem-response` preset failed with `duplicate preset name` instead of silently overriding core; current duplicate checks apply to explicitly loaded add-ons. |
| 2026-06-18 | `cargo run -- preset list` | passed | Default built-in-only list shows preset set, declared risk, effective risk, categories, and command; Quectel/SORACOM file presets are not embedded in built-ins. |
| 2026-06-18 | `cargo run -- tui --theme dark` then `q` in an agent PTY | passed | TUI started with compact Status and restored the terminal on quit. |
| 2026-06-18 | `cargo run -- tui --theme no-color` then `q` in an agent PTY | passed | No-color TUI started with risk labels and restored the terminal on quit. |
| 2026-06-18 | `cargo fmt --check` | passed | Final formatting check after Checkpoint 11.5 implementation. |
| 2026-06-18 | `cargo test` | passed | Final test run after Checkpoint 11.5 implementation; 105 tests passed. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Final lint check after Checkpoint 11.5 implementation; no warnings. |
| 2026-06-18 | Web source check: SORACOM advanced data-send/receive troubleshooting | passed | Confirmed missing standard diagnostic AT checkpoints were `AT+COPS=?` and `AT+CGATT?`; source last updated 2025-04-23. |
| 2026-06-18 | `cargo fmt --check` | passed | Formatting check after adding `available-operators` and `packet-attach` built-in presets. |
| 2026-06-18 | `cargo test` | passed | 105 tests passed, including standard workflow risk classification coverage for `AT+COPS=?` and `AT+CGATT?`. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after adding SORACOM diagnostic preset coverage. |
| 2026-06-18 | `cargo run -- preset list` | passed | Output includes `available-operators` and `packet-attach` as `core` presets with `safe` declared and effective risk. |
| 2026-06-18 | `cargo run -- preset run available-operators --vid 0x0000 --pid 0x0000` | expected failure | Safe preset validation passed without confirmation, then fake VID/PID stopped before device access. |
| 2026-06-18 | `cargo run -- preset run packet-attach --vid 0x0000 --pid 0x0000` | expected failure | Safe preset validation passed without confirmation, then fake VID/PID stopped before device access. |
| 2026-06-18 | `cargo run -- send --help` | passed | Help output shows user AT command timeout default as `30`. |
| 2026-06-18 | `cargo run -- preset list` | passed | Output still includes the current built-in presets, including `available-operators` and `packet-attach`. |
| 2026-06-18 | `cargo run -- tui --theme no-color` then `q` in an agent PTY | passed | TUI started with Status under Devices, bottom Response/Logs panes, `Logs` title, and terminal restoration on quit. |
| 2026-06-18 | `cargo fmt --check` | passed | Final formatting check after default-timeout, TUI running-status, and layout correction implementation. |
| 2026-06-18 | `cargo test` | passed | 108 tests passed, including default timeout, TUI timeout-budget rendering, and running-command action blocking coverage. |
| 2026-06-18 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Final lint check after default-timeout, TUI running-status, and layout correction implementation; no warnings. |
| 2026-06-19 | Web source check: SORACOM AT command reference | passed | Confirmed SORACOM documents `AT+COPS=?` as a long-running operator scan that typically takes 2 to 3 minutes, supporting the 180-second preset timeout hint. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after Devices, Status key-value, preset timeout hint, and TUI timeout override implementation. |
| 2026-06-19 | `cargo test` | passed | 114 tests passed, including preset timeout hints, CLI preset timeout override behavior, TUI temporary timeout input, and TUI device selection execution coverage. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Final lint check after Devices, Status key-value, preset timeout hint, and TUI timeout override implementation; no warnings. |
| 2026-06-19 | `cargo run -- preset list` | passed | Output includes the `timeout-secs` column, and `available-operators` shows `180` for `AT+COPS=?`. |
| 2026-06-19 | `cargo run -- tui --theme no-color`, `t`, `Esc`, then `q` in an agent PTY | passed | TUI rendered an explicit no-device Devices message in the compact left column, Status used key-value lines, the timeout input opened with the effective timeout, cancel restored Status, and quit restored the terminal. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after TUI list-pane viewport and focused-page navigation correction. |
| 2026-06-19 | `cargo test` | passed | 135 tests passed, including Devices, Categories, Commands, Logs, and focused Page/Home/End navigation viewport coverage. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after TUI list-pane viewport and focused-page navigation correction. |
| 2026-06-19 | `cargo run -- tui --theme no-color` then `q` in an agent PTY | passed | TUI started after list-pane viewport correction and restored the terminal on quit. |
| 2026-06-19 | Web source check: Textual Footer, Textual binding display, Bubble Tea Bubbles help/key components, lazygit footer/help UX reference, NN/g visibility of system status | passed | Confirmed shortcut hints belong in footer/help surfaces while Status should remain system state feedback. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after TUI Status/footer responsibility correction. |
| 2026-06-19 | `cargo test` | passed | 138 tests passed, including Status `Keys:` removal, context-sensitive footer hints, and narrow-footer omission coverage. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | No warnings after TUI Status/footer responsibility correction. |
| 2026-06-19 | `cargo run -- tui --theme no-color` then `q` in an agent PTY | passed | TUI rendered a one-row footer, did not show `Keys:` in Status, and restored the terminal on quit. |
| 2026-06-19 | Documentation stale-term search for explicit device selection gate | passed | Confirmed active docs no longer describe `d` as the current cycle-to-select model. Checkpoint 11.5 was later approved by the user. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after explicit TUI device selection gate implementation. |
| 2026-06-19 | `cargo test` | passed | 117 tests passed, including no-device send blocking, one-device auto-selection, multiple-device explicit selection, and post-command device reselection coverage. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after explicit TUI device selection gate implementation; no warnings. |
| 2026-06-19 | `cargo run -- tui --theme no-color`, `Enter`, then `q` in an agent PTY | passed | In this agent environment no matching USB device was visible; pressing Enter displayed the no-device block message instead of sending, and `q` restored the terminal. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after separating TUI device display from USB descriptor display and adding VID/PID to TUI execution selection. |
| 2026-06-19 | `cargo test` | passed | 118 tests passed, including TUI device detail rendering for USB manufacturer, USB product, VID, and PID. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after TUI selected-device VID/PID filtering and device identity display correction; no warnings. |
| 2026-06-19 | `cargo fmt --check` | passed | Final formatting check after removing `KnownDevice`, `known_name`, known-device-list discovery, and `devices --all`. |
| 2026-06-19 | `cargo test` | passed | 119 tests passed, including rejection of the removed `devices --all` flag and TUI device detail rendering without known/profile labels. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Final lint check after changing device discovery to runtime USB devices plus explicit selectors only; no warnings. |
| 2026-06-19 | `cargo run -- devices --all` | expected failure | Confirmed the removed compatibility flag is rejected with `unexpected argument '--all'`; normal discovery is `atctl devices`. |
| 2026-06-19 | Checkpoint 11.5 completion approval | passed | User approved Checkpoint 11.5 as complete after reviewing the long-running AT command cancellation policy and device discovery corrections. |
| 2026-06-19 | `cargo check` | passed | Added `portable-pty 0.9.0` and `ctrlc 3.5.2`; dependency resolution required network approval and updated `Cargo.lock`. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after Checkpoint 12 PTY bridge implementation. |
| 2026-06-19 | `cargo test` | passed | 126 tests passed, including PTY CR/LF decoding, PTY risk confirmation, masked sensitive PTY output, bridge CLI parsing, and symlink guard behavior. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after Checkpoint 12 PTY bridge implementation; no warnings. |
| 2026-06-19 | `cargo run -- bridge --symlink /private/tmp/atctl-bridge-no-device-20260619-check --vid 0x0000 --pid 0x0000` | expected failure | Confirmed fake VID/PID fails before symlink creation with `no matching USB device found`. |
| 2026-06-19 | `ls -l /private/tmp/atctl-bridge-no-device-20260619-check` | expected failure | Confirmed the bridge failure did not create the requested symlink path. |
| 2026-06-19 | `cargo run -- bridge --help` | passed | Help output shows `--symlink`, `--replace-symlink`, USB selectors, endpoint overrides, and default `--timeout 30`. |
| 2026-06-19 | Checkpoint 12 review procedure correction | passed | Corrected the user review workflow to start with `atctl devices` and prefer runtime `--bus <BUS> --address <ADDRESS>` selection instead of assuming validation-target VID/PID prior knowledge. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after bridge first-time runtime discovery documentation and help correction. |
| 2026-06-19 | `cargo test` | passed | 127 tests passed, including `bridge_help_describes_runtime_device_discovery`. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after bridge first-time runtime discovery documentation and help correction; no warnings. |
| 2026-06-19 | `cargo run -- bridge --help` | passed | Help output now tells first-time users to run `atctl devices`, choose the current runtime target, prefer `--bus <BUS> --address <ADDRESS>`, and treat VID/PID as usable only when unique in current output. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after `devices` default target filtering and `--all-usb` implementation. |
| 2026-06-19 | `cargo test` | passed | 128 tests passed, including conservative AT candidate device-class filtering and `--all-usb` CLI parsing. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after `devices` default target filtering and `--all-usb`; no warnings. |
| 2026-06-19 | `cargo run -- devices` | passed | In this agent environment no USB modem / AT candidate devices were visible; output pointed to `atctl devices --all-usb`. |
| 2026-06-19 | `cargo run -- devices --all-usb` | passed | In this agent environment no USB devices were visible through `libusb`; output reported no matching USB devices. |
| 2026-06-19 | `cargo run -- devices --help` | passed | Help output shows `--all-usb` as the full USB visibility troubleshooting option. |
| 2026-06-19 | `cargo run -- bridge --help` | passed | Help output mentions current AT operation target output and `atctl devices --all-usb` for full USB visibility troubleshooting. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after TUI full-USB troubleshooting view implementation. |
| 2026-06-19 | `cargo test` | passed | 130 tests passed, including TUI all-USB troubleshooting view target selection and diagnostic-only non-target blocking. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after TUI full-USB troubleshooting view implementation; no warnings. |
| 2026-06-19 | `cargo run -- tui --theme no-color`, then `u`, `q` in an agent PTY | passed | Historical pre-OQ-022 verification: TUI started, accepted the then-current full-USB troubleshooting view toggle, displayed `All USB: 0`, and restored the terminal. Current TUI uses a Devices pane action row instead of `u`. |
| 2026-06-19 | User-run Checkpoint 12 real-device bridge review | failed then fixed | `AT`, `ATI`, masked `AT+CIMI`, and write-risk confirmation passed; quitting `screen` before Ctrl-C exposed PTY client disconnect as `failed to write PTY: Input/output error`. |
| 2026-06-19 | `cargo test transport::pty` | passed | 10 tests passed, including PTY client disconnect write, flush, and EIO regression coverage. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after PTY client disconnect cleanup correction. |
| 2026-06-19 | `cargo test` | passed | 142 tests passed, including PTY client disconnect regression coverage. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after PTY client disconnect cleanup correction; no warnings. |
| 2026-06-19 | User approval for Checkpoint 12 | passed | User approved the PTY bridge first implementation after real-device `screen` workflow, PTY client shutdown handling, and symlink cleanup checks passed. |
| 2026-06-19 | `cargo test tui::tests` | passed | 63 TUI tests passed, including command search, edit-before-run, rerun, masked Response save, and empty Response save warning coverage. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after implementing the remaining required TUI key bindings and documentation corrections. |
| 2026-06-19 | `cargo test` | passed | 148 tests passed, including command search, edit-before-run safety flow, rerun, masked Response save, and empty Response save warning coverage. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after implementing the remaining required TUI key bindings; no warnings. |
| 2026-06-19 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | Historical pre-OQ-022 verification: TUI started, the footer showed then-current direct shortcuts, and quit restored the terminal. Current TUI keeps `/`, `?`, and `q` as global letters and moves secondary actions to Controls. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after implementing explicit `--preset-file` / `--preset-dir` file preset location overrides. |
| 2026-06-19 | `cargo test` | passed | 151 tests passed, including parsing explicit file preset locations for `preset list`, `preset run`, and `tui`, loading explicit file/dir locations, and rejecting missing explicit files. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after explicit file preset location override implementation; no warnings. |
| 2026-06-19 | `cargo run -- preset list --help`, `cargo run -- preset run --help`, `cargo run -- tui --help` | passed | Help output shows `--preset-file <FILE>` and `--preset-dir <DIR>` on all three shared-preset surfaces. |
| 2026-06-19 | `cargo run -- preset list --preset-dir examples/presets` | passed | Loaded built-in presets plus repository-managed Quectel and SORACOM file presets from the explicit directory. |
| 2026-06-19 | `cargo run -- preset run signal-quectel --preset-dir examples/presets --vid 0x0000 --pid 0x0000` | expected failure | The explicit directory loaded `signal-quectel`; execution then stopped at the fake USB selector with `no matching USB device found`. |
| 2026-06-19 | `cargo run -- preset list --preset-file examples/presets/quectel.toml` | passed | Loaded built-in presets plus the explicit Quectel file preset. |
| 2026-06-19 | `cargo run -- preset list --preset-file /definitely/missing/atctl-presets.toml` | expected failure | Missing explicit preset file returned an actionable read error instead of being silently ignored. |
| 2026-06-19 | `cargo run -- tui --preset-dir examples/presets --theme no-color`, then `q` in an agent PTY | passed | TUI started with explicit file preset directory data visible and restored the terminal on quit. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after TUI preset set label display correction. |
| 2026-06-19 | `cargo test tui::tests` | passed | 65 TUI tests passed, including default built-in-only preset set label suppression and mixed built-in/file preset distinction. |
| 2026-06-19 | `cargo test` | passed | 153 tests passed after TUI preset set label display correction. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after TUI preset set label display correction; no warnings. |
| 2026-06-19 | `cargo run -- tui --theme no-color`, then `q` in an agent PTY | passed | Default built-in-only TUI did not show preset set detail; terminal was restored on quit. |
| 2026-06-19 | `cargo run -- tui --theme no-color --preset-dir examples/presets`, then `q` in an agent PTY | superseded | Earlier mixed file preset TUI display was superseded by preset set grouping. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after TUI preset set grouping, category cleanup, and command ordering correction. |
| 2026-06-19 | `cargo test tui::tests` | passed | 67 TUI tests passed, including preset set group headers, no inline badges, preset set metadata exclusion from Categories, non-selectable preset set headers, and file preset entry-order preservation. |
| 2026-06-19 | `cargo test presets::` | passed | 12 preset tests passed, including curated built-in workflow order and repository file preset loading through the shared drop-in loader. |
| 2026-06-19 | `cargo test` | passed | 156 tests passed after TUI preset set grouping, category cleanup, and command ordering correction. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after TUI preset set grouping, category cleanup, and command ordering correction; no warnings. |
| 2026-06-19 | `cargo run -- tui --theme no-color --preset-dir examples/presets`, then `q` in an agent PTY | passed | Mixed preset set TUI started with `Built-in presets` group header visible; Categories showed workflow categories without `quectel` or `soracom`; terminal was restored on quit. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after adding blank separator rows before second and later TUI preset set group headers. |
| 2026-06-19 | `cargo test tui::tests` | passed | 67 TUI tests passed, including non-selectable blank separator rows before second and later preset set group headers and no blank row after the header. |
| 2026-06-19 | `cargo test` | passed | 156 tests passed after adding TUI preset set group separator rows. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after adding TUI preset set group separator rows; no warnings. |
| 2026-06-19 | `cargo run -- tui --theme no-color --preset-dir examples/presets`, scroll Commands, then `q` in an agent PTY | passed | Mixed preset set TUI started, command navigation remained usable after preset set separator row changes, and terminal was restored on quit. |
| 2026-06-19 | `cargo fmt --check` | passed | Formatting check after replacing the legacy preset TOML model with title/categories and preset set display. |
| 2026-06-19 | `cargo test presets::` | passed | 14 preset tests passed, including rejection of legacy `tags` and `source` TOML fields. |
| 2026-06-19 | `cargo test tui::tests` | passed | 67 TUI tests passed after preset set/title/category model cleanup. |
| 2026-06-19 | `cargo test` | passed | 158 tests passed after strict file preset TOML parsing and documentation/model cleanup. |
| 2026-06-19 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after preset set/title/category model cleanup; no warnings. |
| 2026-06-19 | `cargo run -- preset list --preset-dir examples/presets` | passed | Output shows `preset-set` and `categories`, uses TOML titles such as `Quectel commands` and `SORACOM commands`, and does not display `pack:*` labels. |
| 2026-06-19 | `cargo run -- tui --theme no-color --preset-dir examples/presets`, then `q` in an agent PTY | passed | TUI started with `Built-in presets` group header and restored the terminal on quit. |
| 2026-06-21 | `cargo fmt --check` | passed | Formatting check after renaming the built-in minimal AT response check preset to `modem-response`. |
| 2026-06-21 | `cargo test` | passed | 177 tests passed after updating the preset name, docs, CLI expectations, and TUI expectations. |
| 2026-06-21 | `cargo clippy --all-targets --all-features -- -D warnings` | passed | Lint check after the `modem-response` rename; no warnings. |
| 2026-06-21 | `cargo run -- preset list` | passed | Built-in preset list shows `modem-response` for the minimal `AT` response check and does not include the previous misleading preset name. |

## User Checkpoints

Checkpoint 1 is complete and approved. Checkpoint 2 is complete and approved.
Checkpoint 3 is complete and approved. Checkpoint 4 is complete and approved.
Checkpoint 5 is complete and approved. Checkpoint 6 is complete and approved.
Checkpoint 7 is complete and approved. Checkpoint 8 is complete and approved.
Checkpoint 9 is complete and approved. Checkpoint 10 is complete and approved.
Checkpoint 11 is complete and approved. Checkpoint 11 is TUI masked log viewer
work before PTY. Checkpoint 11.5 is complete and approved. Checkpoint 11.5 is
the required TUI status, preset set/loading, preset risk, repository-managed
file preset examples, ad-hoc input, Response copy, timeout control, explicit device
selection, runtime USB device discovery, and long-running AT command policy work
before PTY. PTY bridge work is Checkpoint 12. Sequence work is Checkpoint 12.6.
Release and Homebrew work is deferred to the final phase after application
features are complete. The intended normal distribution path is Homebrew
formula install with bottle support when available and source-build fallback
when a matching bottle is unavailable.

Checkpoint 11.5 implementation is complete and user approved. Checkpoint 12
implementation is complete and user approved. Checkpoint 12.6 implementation is
complete and user approved, including the user-run SMS, ping, TCP, candidate
refresh, and related TUI behavior review that intentionally required real modem
or external endpoint confirmation. Checkpoint 13 is pending for final release
and Homebrew workflow implementation.
Historical checkpoint review procedures below preserve the behavior that was
approved at the time. Current TUI shortcut behavior is governed by
`docs/SPEC.md` version 0.4.60, OQ-022, and OQ-023: `/`, `?`, and `q` are the global
letter shortcuts, secondary operations use stable Controls pane rows, Devices
view switching uses a Devices pane row, normal focus order is
Categories -> Commands / Sequences -> Controls -> Response -> Logs -> Devices,
Sequence inputs use a `Run Sequence` modal, and help is modal and concise.

Every user checkpoint report must include:

- Confirmation point: what the user is being asked to review or approve.
- Expected state: what should be true if the checkpoint is correct.
- Review procedure: concrete commands, files, or behavior to inspect.
- Pass criteria: what result means the checkpoint is acceptable.
- Fail criteria: what result means implementation must stop and be corrected.
- Scope boundary: what is intentionally not implemented or not being confirmed
  at this checkpoint.

### Checkpoint 1 Review Procedure

Confirmation point:

- Confirm that the initial Rust scaffold and non-hardware core behavior are
  acceptable before USB descriptor implementation starts.

Expected state:

- `cargo fmt --check` passes.
- `cargo test` passes with 17 unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo run -- --help` renders the CLI command tree.
- `cargo run -- config path` prints `~/.config/atctl/config.toml`.
- `cargo run -- send AT+CFUN=0 --yes` fails before USB access because
  `--risk-ack dangerous` is required.
- No real USB device access is required for this checkpoint.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- --help
cargo run -- config path
cargo run -- send AT+CFUN=0 --yes
```

Pass criteria:

- The first five commands exit successfully.
- The final command exits unsuccessfully with a message requiring
  `--risk-ack dangerous`.
- The user accepts that USB descriptor work may start next.

Fail criteria:

- Any format, test, or clippy command fails.
- `config path` prints a different path without a documented reason.
- `send AT+CFUN=0 --yes` reaches USB transport or does not reject missing
  risk acknowledgement.

Scope boundary:

- This checkpoint does not confirm USB descriptor inspection, endpoint
  auto-detection, real modem communication, TUI, PTY bridge, or release
  automation.

### Checkpoint 2 Review Procedure

Confirmation point:

- Confirm that USB descriptor inspection and descriptor-shape endpoint candidate
  reporting are acceptable before any AT probe, endpoint selection, or real send
  implementation starts.

Expected state:

- `cargo fmt --check` passes.
- `cargo test` passes with 24 unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo run -- devices` exits successfully.
- `cargo run -- inspect --interface 2 --bulk-in 0x85 --bulk-out 0x04`
  exits successfully and reports the manual override separately from descriptor
  candidates.
- If no matching AT operation target is visible, `devices` says
  `No USB modem / AT candidate devices found.` and points to
  `atctl devices --all-usb`.
- If no matching USB device is visible, `inspect` says
  `No matching USB devices found.`
- If a SORACOM Onyx / Quectel EG25-G is connected and visible, `devices`
  reports USB descriptor values such as `manufacturer=Quectel`,
  `product=EG25-G`, and explicit VID/PID plus bus/address selectors.
- `inspect` reports configurations, interfaces, alternate settings, endpoints,
  and descriptor-shape bulk IN/OUT candidates.
- Output must not claim that a descriptor-shape candidate is confirmed to be an
  AT command endpoint.
- User-facing `devices` output masks USB serial descriptor values.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- devices
cargo run -- inspect --interface 2 --bulk-in 0x85 --bulk-out 0x04
```

Optional real Onyx review, if the modem is connected:

```sh
cargo run -- devices --vid 0x2c7c --pid 0x0125
cargo run -- inspect --vid 0x2c7c --pid 0x0125
```

Pass criteria:

- Format, test, and clippy commands pass.
- USB listing and inspection commands exit successfully.
- With no visible modem, the commands report no matching device without error.
- With a visible Onyx, `inspect` reports its descriptor tree including
  interface alternate settings and endpoint descriptors.
- Any bulk IN/OUT pair is labelled as `Descriptor-shape AT candidate`, not as a
  confirmed endpoint.
- Manual `--interface`, `--bulk-in`, and `--bulk-out` inputs are labelled as a
  manual override and are not mixed with descriptor-derived candidates.

Fail criteria:

- Any verification command fails.
- The implementation sends an AT command, claims an endpoint by fixed Onyx
  mapping, claims descriptor-shape candidates are confirmed AT endpoints, omits
  alternate settings, or prints an unmasked USB serial descriptor.
- A connected Onyx is not listed with `--vid 0x2c7c --pid 0x0125`.

Scope boundary:

- This checkpoint does not claim an endpoint, claim an interface, send `AT`,
  claim real modem communication, detach kernel drivers, claim interfaces,
  implement TUI, implement PTY bridge, or implement release automation.

### Checkpoint 3 Review Procedure

Confirmation point:

- Confirm that the `atctl send` workflow is ready for a user-run real Onyx
  `AT` check before broader safety, presets, TUI, PTY bridge, or release work
  starts.

Expected state:

- `cargo fmt --check` passes.
- `cargo test` passes with 34 unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- Mock transport tests covered successful `send AT`, masked sensitive output,
  JSON output masking, AT `ERROR` handling, and the then-current raw-log
  placeholder rejection. Current raw diagnostic export behavior is covered by
  the current test suite.
- `cargo run -- send AT --vid 0x0000 --pid 0x0000` fails before sending with a
  no matching USB device message.
- `cargo run -- send AT+CFUN=0 --yes` fails before USB access because
  `--risk-ack dangerous` is required.
- USB transport selects endpoints by manual override or descriptor inspection
  plus safe `AT` probe; it does not use fixed Onyx endpoint constants.
- A real Onyx `AT` check is ready to be run by the user, but has not been
  claimed complete by the agent.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- send AT --vid 0x0000 --pid 0x0000
cargo run -- send AT+CFUN=0 --yes
```

Optional real Onyx review, if the modem is connected and the user is ready to
send the safe `AT` probe:

```sh
cargo run -- devices --vid 0x2c7c --pid 0x0125
cargo run -- inspect --vid 0x2c7c --pid 0x0125
cargo run -- send AT --vid 0x2c7c --pid 0x0125
```

If auto-detection fails but `inspect` shows a plausible bulk IN/OUT pair, retry
with explicit endpoints from the current `inspect` output:

```sh
cargo run -- send AT --vid 0x2c7c --pid 0x0125 --interface <N> --bulk-in <ENDPOINT> --bulk-out <ENDPOINT>
```

Pass criteria:

- Format, test, and clippy commands pass.
- The nonexistent VID/PID command exits unsuccessfully before sending.
- The dangerous command exits unsuccessfully before USB access.
- With a visible Onyx, `send AT` prints an `OK` response.
- If manual endpoint override is needed, the command succeeds only with values
  taken from the current `inspect` output.

Fail criteria:

- Any format, test, or clippy command fails.
- A dangerous command reaches USB access without a matching `--risk-ack`.
- `send AT` uses fixed endpoint constants instead of runtime selection.
- The real Onyx check cannot list the device, cannot inspect descriptors, or
  cannot get an `OK` response with either auto-detection or explicit endpoints.

Scope boundary:

- This checkpoint does not complete broad safety policy, presets, TUI, PTY
  bridge, release automation, raw session logging, persistent command safety
  hardening, or real hardware validation unless the user runs and approves the
  optional Onyx check.

### Checkpoint 4 Review Procedure

Confirmation point:

- Confirm that direct `atctl send` safety behavior is acceptable before release
  workflow and Homebrew material work starts.

Expected state:

- `docs/SPEC.md` is version `0.4.11`.
- `cargo fmt --check` passes.
- `cargo test` passes with 42 unit tests.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- Safe and sensitive read/test commands may run without confirmation.
- Sensitive output remains masked by default in text and JSON output.
- `--no-mask` affects display output only.
- Write, persistent, dangerous, and unknown non-read/test direct commands
  require confirmation before USB access.
- Interactive confirmation requires typing the exact classified risk level.
- Non-interactive automation bypass requires both `--yes` and matching
  `--risk-ack <risk>`.
- `--risk-ack` mismatch fails before USB access, even if the command is safe.
- At this historical checkpoint, the old `--raw-log` placeholder remained
  rejected and was not silently enabled. Current behavior is the resolved OQ-021
  `--raw-log-file <PATH> --raw-log-ack raw-log` design.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- send AT --risk-ack dangerous --vid 0x0000 --pid 0x0000
cargo run -- send ATE0 --vid 0x0000 --pid 0x0000
cargo run -- send AT+CFUN=0 --yes --risk-ack dangerous --vid 0x0000 --pid 0x0000
cargo run -- send AT+CFUN=0 --yes
```

Optional real Onyx review, if the modem is connected and the user wants to test
the interactive confirmation path with a low-impact write command:

```sh
cargo run -- send ATE0 --vid 0x2c7c --pid 0x0125
```

The prompt should show the command, risk, and reason. Type `write` only if you
accept sending `ATE0`.

Pass criteria:

- Format, test, and clippy commands pass.
- Mismatched `--risk-ack` fails before USB access.
- Write command without confirmation fails before USB access.
- `--yes --risk-ack dangerous` passes safety validation but still stops at the
  fake VID/PID device selection.
- `--yes` alone for a dangerous command fails before USB access.
- Optional real Onyx `ATE0` either prompts for `write` before sending or is not
  run.

Fail criteria:

- Any format, test, or clippy command fails.
- A write, persistent, dangerous, or unknown non-read/test command reaches USB
  without confirmation.
- `--yes` alone bypasses confirmation.
- `--risk-ack` mismatch reaches USB.
- Sensitive text or JSON output is unmasked by default.

Scope boundary:

- This checkpoint does not implement raw session logging, command history,
  presets, TUI safety dialogs, PTY bridge, or release automation.

### Checkpoint 5 Review Procedure

Historical note:

- This checkpoint recorded the initial preset implementation. Checkpoint 11.5
  supersedes the preset set model, built-in/vendor split, multi-file user
  preset loading, and effective-risk requirements. Do not use this historical
  checkpoint to reintroduce Quectel/SORACOM commands into built-in presets.

Confirmation point:

- Historical confirmation point: confirm that the then-current built-in
  presets, config loading, and single-file user preset loading were acceptable
  before command history, masked session logging, TUI, PTY bridge, or packaging
  work started. Current add-on loading no longer uses default XDG auto-load.

Expected state:

- `atctl preset list` lists the then-current built-in presets from
  `docs/SPEC.md`.
- The list shows preset name, command, risk level, and categories.
- Historical user presets could be loaded from `~/.config/atctl/presets.toml`
  or from `XDG_CONFIG_HOME` when set. Current file preset add-ons require
  explicit `--preset-file` or `--preset-dir`.
- Config loading supports `~/.config/atctl/config.toml` and honors
  `XDG_CONFIG_HOME` for config discovery.
- Invalid config or preset TOML returns an actionable error instead of
  silently ignoring the file.
- No config or preset file is created or overwritten automatically.
- Preset risk levels are explicit and are not inferred at run time by guessing.
- `atctl preset run <NAME>` uses the same masking and confirmation policy as
  direct `atctl send`.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- preset list
cargo run -- preset run modem-response --vid 0x0000 --pid 0x0000
```

Historical optional user-preset review with a temporary XDG config directory
superseded by the current explicit add-on loading boundary:

```sh
tmpdir="$(mktemp -d)"
mkdir -p "$tmpdir/atctl"
cat > "$tmpdir/atctl/presets.toml" <<'EOF'
[[presets]]
name = "custom-modem-response"
command = "AT"
risk = "safe"
categories = ["custom"]
EOF
XDG_CONFIG_HOME="$tmpdir" cargo run -- preset list
```

Pass criteria:

- Format, test, and clippy commands pass.
- `preset list` includes built-in presets with risk levels and categories.
- In this historical checkpoint, the optional temporary user preset appeared
  when `XDG_CONFIG_HOME` pointed to the temporary config directory. Current
  verification should use explicit `--preset-file` or `--preset-dir`.
- `preset run modem-response --vid 0x0000 --pid 0x0000` reaches the existing fake-device
  failure only after preset lookup and safety validation.
- No user config or preset file is created automatically.

Fail criteria:

- Any format, test, or clippy command fails.
- Built-in presets from the specification are missing or have wrong risk
  levels.
- Current `preset list` auto-loads preset add-ons from `XDG_CONFIG_HOME`.
- Invalid TOML is silently ignored.
- Preset execution bypasses masking or confirmation behavior.
- The implementation creates or overwrites config files without an explicit
  user initialization command.

Scope boundary:

- This checkpoint does not implement command history, masked session logging,
  TUI panes, TUI dialogs, PTY bridge behavior, release workflow files, GitHub
  Releases, Homebrew formulae, or Homebrew tap CI.

### Checkpoint 6 Review Procedure

Confirmation point:

- Confirm that command history and masked session logging are acceptable before
  TUI work starts.

Expected state:

- Successful `send` and `preset run` executions write command history and a
  masked session log.
- Command history and session logs are separate files.
- History is written as JSON Lines at:

  ```text
  ~/.local/state/atctl/history.jsonl
  ```

- Masked session logs are written under:

  ```text
  ~/.local/state/atctl/logs/
  ```

- `XDG_STATE_HOME` is honored when set.
- Saved command strings and responses are masked regardless of `--no-mask`
  display behavior.
- At this historical checkpoint, raw logging remained disabled and the old
  `--raw-log` placeholder was still rejected. Current behavior is the resolved
  OQ-021 raw diagnostic export design.
- `atctl logs list` lists existing history/session logs and does not create
  files when no logs exist.
- Log retention and rotation are not implemented in this checkpoint.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
tmpdir="$(mktemp -d)"
XDG_STATE_HOME="$tmpdir" cargo run -- logs list
# Historical checkpoint-only placeholder rejection check was superseded by
# OQ-021 raw diagnostic export.
```

Optional real Onyx logging review, if the modem is connected:

```sh
tmpdir="$(mktemp -d)"
XDG_STATE_HOME="$tmpdir" cargo run -- preset run modem-response --vid 0x2c7c --pid 0x0125
XDG_STATE_HOME="$tmpdir" cargo run -- logs list
find "$tmpdir/atctl" -type f -print
```

Pass criteria:

- Format, test, and clippy commands pass.
- Unit tests confirm masked session logs and response-free command history.
- `logs list` with an empty temporary `XDG_STATE_HOME` prints `No logs found.`
  and creates no files.
- At this historical checkpoint, the old `--raw-log` placeholder was rejected
  before USB access. Current raw diagnostic export uses
  `--raw-log-file <PATH> --raw-log-ack raw-log`.
- Optional real Onyx logging review creates one `history.jsonl` file and one
  `.session.log` file under the temporary state directory.

Fail criteria:

- Any format, test, or clippy command fails.
- Raw sensitive response values appear in saved masked session logs.
- Command history contains response bodies.
- `--no-mask` causes saved logs to become raw.
- At this historical checkpoint, accepting the old `--raw-log` placeholder or
  letting it reach USB access would have failed the checkpoint. Current raw
  diagnostic export behavior is governed by OQ-021.
- `logs list` creates files when there are no logs.

Scope boundary:

- This checkpoint does not implement raw logging, log retention, log rotation,
  TUI panes, TUI dialogs, PTY bridge behavior, release workflow files, GitHub
  Releases, Homebrew formulae, or Homebrew tap CI.

### Checkpoint 7 Review Procedure

Confirmation point:

- Confirm that the TUI skeleton and core panes are acceptable before wiring
  command execution and confirmation dialogs into the TUI.

Expected state:

- `Cargo.toml` includes `ratatui` and `crossterm`.
- `atctl tui` starts an alternate-screen TUI and restores the terminal on exit.
- The TUI displays these panes:
  - Devices
  - Categories
  - Commands
  - Response
  - History / Status
- `q` exits.
- `?` toggles help.
- Left/right moves focus between panes.
- Up/down changes category or command selection when those panes are focused.
- Enter previews the selected command without sending it.
- `c` clears the response pane.
- `l` focuses History / Status.
- `m`, `e`, `r`, and `s` show placeholder status messages instead of running
  unfinished behavior.
- No AT command is sent from the TUI in this checkpoint.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- tui
```

Inside the TUI:

```text
?              Toggle help
Left / Right   Move focus
Up / Down      Move selection in Categories or Commands
Enter          Preview selected command
c              Clear response pane
l              Focus History / Status
m              Show masking status
q              Quit
```

Pass criteria:

- Format, test, and clippy commands pass.
- Unit tests confirm required pane rendering and basic key handling.
- `atctl tui` renders the five required panes.
- Help toggles with `?`.
- `q` exits and the terminal returns to normal prompt behavior.
- Enter only previews a command; it does not send an AT command.

Fail criteria:

- Any format, test, or clippy command fails.
- `atctl tui` crashes on startup.
- The terminal remains in raw/alternate-screen mode after exit.
- Any required pane is missing.
- Enter sends an AT command before confirmation-dialog work is implemented.
- Help or quit key handling does not work.
- The user-approved colored focus and selection accents are removed or replaced
  without explicit approval.

Scope boundary:

- This checkpoint does not implement TUI command execution, TUI confirmation
  dialogs, edit-before-run, rerun, save-response, live USB refresh, raw output
  view, light/dark theme selection, PTY bridge behavior, release workflow
  files, GitHub Releases, Homebrew formulae, or Homebrew tap CI.

### Checkpoint 8 Review Procedure

Confirmation point:

- Confirm that TUI visual accessibility and theme foundations are correct
  before wiring command execution or confirmation dialogs into the TUI.

Expected state:

- `docs/SPEC.md` section 16.1 remains the normative source for TUI visual
  accessibility and theme behavior.
- TUI style choices are represented through semantic roles rather than
  scattered raw color decisions.
- Required semantic roles include at least focus, selected, status, muted text,
  safe risk, sensitive risk, write risk, dangerous risk, warning, and error.
- Non-color affordances remain available for state and meaning:
  - selection marker
  - pane borders/focus emphasis
  - risk labels such as `[safe]`, `[sensitive]`, and `[write]`
  - explicit confirmation/error text
- The current cyan/yellow colored baseline is preserved unless a concrete
  replacement palette is explicitly approved.
- Light/dark support is not claimed complete unless separate dark and light
  palettes are specified and their foreground/background pairs are checked
  against the contrast targets in `docs/SPEC.md` section 16.1.
- Color opt-out behavior is specified or implemented without removing non-color
  affordances.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Manual/visual review, when requested:

```sh
cargo run -- tui
NO_COLOR=1 cargo run -- tui
```

Inside the TUI:

```text
Left / Right   Move focus and confirm focus is not color-only
Up / Down      Move selection and confirm selection is not color-only
?              Toggle help
q              Quit
```

Pass criteria:

- Format, test, and clippy commands pass.
- Tests verify semantic style roles and approved baseline preservation.
- Focus and selection remain distinguishable without relying on color alone.
- Risk levels remain visible through text labels, not only color.
- Any claim of light/dark completion is backed by specified palettes and
  contrast checks.
- `NO_COLOR` or another color opt-out path, if implemented in this checkpoint,
  preserves non-color affordances.

Fail criteria:

- Raw color choices are spread through TUI rendering code instead of semantic
  roles.
- The current user-approved colored baseline is changed without explicit
  approval.
- Focus, selection, risk, warning, or error state depends on color alone.
- Light/dark support is claimed without specified palettes and contrast checks.
- Color opt-out removes the only indicator of state or meaning.

Scope boundary:

- This checkpoint does not send AT commands from the TUI.
- This checkpoint does not implement TUI command execution, confirmation
  dialogs, edit-before-run, rerun, save-response, live USB refresh, raw output
  view, PTY bridge behavior, release workflow files, GitHub Releases, Homebrew
  formulae, or Homebrew tap CI.

### Checkpoint 9 Review Procedure

Confirmation point:

- Confirm that TUI command execution and confirmation dialogs are acceptable
  before PTY bridge work starts.

Expected state:

- `Enter` on a safe or sensitive visible preset runs the command through the
  same send pipeline used by CLI and preset execution.
- Before transport execution starts, the TUI shows that a command is running
  and displays the preset name, AT command string, risk level, and expected
  effect.
- After completion or failure, the Response pane keeps the command context
  above the result.
- Command output shown in the Response pane remains masked by default.
- Session/history logging uses the existing masked logging path.
- Status and History are separate panes.
- Status shows the selected or active command context.
- `Enter` on a write, persistent, or unknown preset opens a confirmation dialog
  instead of sending immediately.
- Confirmation dialogs show command name, command string, risk level, and
  expected effect.
- The command is sent only when the input exactly matches the displayed risk
  level, such as `write`.
- Mismatched confirmation input rejects the command before USB access.
- At Checkpoint 9, dangerous commands were hidden because no explicit
  dangerous-preset product behavior had been approved. This is superseded by
  Checkpoint 12.5 for explicit modem functionality presets.
- `q` exits and restores the terminal after normal TUI use and after dialog
  rejection.
- `c` clears the Response pane without leaving stale characters from previous
  content.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Manual review with Onyx connected, when the user wants to confirm real TUI
execution:

```sh
cargo run -- tui
```

Inside the TUI:

```text
Up / Down      Select `modem-response`
Enter          Run the safe `modem-response` preset
Expected       Response pane shows `AT` / `OK`
Expected       Response pane also shows `Completed command`, `Command: AT`,
               `Risk: safe`, and the expected effect
Expected       Status pane shows selected or active command context

c              Clear the response pane
Expected       Previous response text disappears completely

Up / Down      Select `disable-command-echo`
Enter          Open the write confirmation dialog
abc + Enter    Reject confirmation; command is not sent
Enter again    Reopen the dialog if needed
write + Enter  Confirm and send only when you intentionally accept this write command
q              Quit
```

Pass criteria:

- Format, test, and clippy commands pass.
- Safe/sensitive command execution uses the shared CLI/preset send behavior.
- Running and completed command context is visible enough to identify which AT
  command produced the response.
- Write-risk command execution is blocked until exact risk-level confirmation.
- Wrong confirmation input does not send the command.
- At this checkpoint, the TUI does not expose dangerous commands by default.
  This criterion is superseded by Checkpoint 12.5 for explicit modem
  functionality presets that require typed dangerous confirmation.
- `c` clears the Response pane without leaving stale response fragments.
- Terminal raw mode and alternate screen are restored after quit.

Fail criteria:

- A confirmation-required command sends before the dialog is accepted.
- Wrong confirmation input sends a command.
- Dangerous commands send without exact typed risk confirmation.
- Sensitive output appears unmasked by default.
- The TUI appears frozen because no running-command context is visible during
  command execution.
- Response pane clearing leaves stale characters from previous output.
- The terminal remains in raw/alternate-screen mode after quitting.
- Any format, test, or clippy command fails.

Scope boundary:

- This checkpoint does not implement edit-before-run, rerun, save-response,
  live USB refresh, raw output view, PTY bridge behavior, release workflow
  files, GitHub Releases, Homebrew formulae, or Homebrew tap CI.

### Checkpoint 11 Review Procedure

Confirmation point:

- Confirm that the TUI masked log viewer is acceptable before PTY bridge work
  starts.

Expected state:

- Checkpoint 10 remains complete and approved.
- The History pane shows existing history/session logs with a visible selected
  row.
- `l` focuses the History pane.
- `Up` and `Down` move the selected log when History is focused.
- `Enter` on a selected log reads that existing file and shows masked content
  in the Response pane.
- Response content can be scrolled when Response is focused.
- `PageUp` and `PageDown` scroll Response faster.
- Response remains the AT command response pane. Opening a log temporarily
  shows masked log content in that pane without changing the pane's main role.
- Masked log content shows read-only line numbers in the Response pane.
- Response pane title shows the visible line range, total line count, and
  whether more content exists above/below.
- Opening a selected log moves focus to Response and keeps the History
  selection state visible.
- Normal focus movement cycles through interactive panes only:
  Categories, Commands, Response, History.
- Log viewing does not create new history/session log files.
- Session log content is displayed through the masked path. Raw values must not
  be revealed by log viewing.
- Status shows that a masked log is being viewed and identifies the selected
  log.
- PTY bridge behavior is still not implemented in this checkpoint.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Manual review with existing logs:

```sh
cargo run -- tui
```

Inside the TUI:

```text
l              Focus History
Up / Down      Select a history or session log
Enter          Open the selected log
Expected       Response shows masked log content, not only the file name
Expected       Response title shows visible lines, total lines, and top/bottom/more state
Expected       Opened log content includes read-only line numbers
Expected       Focus moves to Response after opening the selected log
Expected       Status says a masked log is being viewed
Expected       Raw sensitive values are not shown
Tab / Right    Move focus through Categories, Commands, Response, History
Response focus Up / Down      Scroll response content one line
Response focus PageUp/Down    Scroll response content faster
Response focus Home / End     Jump to top or bottom of response content
q              Quit
```

Pass criteria:

- Format, test, and clippy commands pass.
- History selection is visible without depending on color alone.
- Existing masked history/session log content can be read inside the TUI.
- Long opened log content can be scrolled inside the TUI.
- Response line range is shown in the Response pane itself and describes the
  visible range, not only the first visible line.
- Log-view line numbers are visible in the Response pane and do not imply an
  editable cursor.
- Opening a log moves focus to Response without losing History selection.
- Focus order does not include informational-only Devices or Status panes in
  the normal Tab cycle.
- Log viewing does not write raw values and does not create new logs.
- PTY bridge remains out of scope.

Fail criteria:

- The History pane only lists file names and cannot open masked content.
- Raw sensitive values appear through log viewing.
- Opening a log creates a new log file.
- History selection is invisible without color.
- Opened log content below the visible pane cannot be reached by keyboard.
- Response scroll status is shown only as a detached `line N of M` status line
  outside the Response pane.
- Log-view line numbers are missing or imply editable cursor behavior.
- Opening a log leaves focus on History and requires an unrelated focus move
  before scrolling the opened content.
- Tab/Right focus order jumps through informational-only panes before the
  interactive panes.
- Any format, test, or clippy command fails.

Scope boundary:

- This checkpoint does not implement raw-log viewing, raw-log creation,
  session-wide raw mode, PTY bridge behavior, release workflow files, GitHub
  Releases, Homebrew formulae, or Homebrew tap CI.

### Checkpoint 11.5 Review Procedure

Confirmation point:

- Confirm that TUI status layout, preset set/loading behavior,
  repository-managed file preset examples, effective preset risk, ad-hoc AT input,
  and Response copy behavior are acceptable before PTY bridge work starts.

Expected state:

- Checkpoint 11 remains complete and approved.
- The Status area is compact and non-interactive.
- Status shows concise state and command context without duplicating response
  or log bodies.
- Status is placed under Devices in the left column rather than as a full-width
  band.
- Devices plus Status occupy the same top-left column height previously used by
  Devices alone.
- Freed layout space is used for Response, Logs, or command navigation
  rather than an oversized Status area.
- The saved history/session list pane is titled `Logs`; row labels may still
  show `history:` and `session:`.
- The Devices pane lists visible matching USB targets or explicitly reports
  that none are visible.
- If no matching USB target is visible, preset execution and ad-hoc AT sending
  are disabled while help, quit, log viewing, scrolling, and copying already
  displayed Response text may remain available.
- If exactly one matching USB target is visible at startup, it is auto-selected,
  selected-device detail is shown, and command execution is available
  immediately.
- If multiple matching USB targets are visible at startup, no active execution
  device is selected automatically and command sending is blocked until the
  user explicitly selects one from Devices.
- `d` focuses or re-enters Devices selection; `Up` and `Down` move the
  highlighted candidate; `Enter` selects the highlighted device.
- After device selection, selected-device detail shows USB manufacturer when
  readable, USB product when readable, VID, PID, bus, and address.
- The selected VID, PID, bus, and address are used for TUI command execution.
- The user can select device A, run `modem-response`, select device B, and run
  `modem-response` again in the same TUI session.
- Devices, Categories, Commands, and Logs remain usable when their item count
  exceeds visible pane height. The selected row remains visible while moving
  with `Up`, `Down`, `PageUp`, `PageDown`, `Home`, and `End`.
- User command timeout defaults to 30 seconds for `send`, `preset run`, and
  TUI command execution.
- Presets can declare `timeout_secs`; `available-operators` declares 180
  seconds because `AT+COPS=?` can take 2 to 3 minutes.
- TUI Controls sets a temporary timeout override for subsequent executions, and
  entering `default` clears the override.
- Endpoint auto-detection probe timeout remains short and separate from user
  command timeout.
- While a TUI command is running, Status shows elapsed time, timeout,
  remaining time, and a timeout-budget progress indicator that proves the UI is
  still active.
- Normal running-command cancellation is not required for Checkpoint 11.5 and
  must not be presented as available unless modem-side stop or session resync
  semantics are specified.
- Product presets are organized around standard modem workflows:
  modem identity, SIM, network registration, signal, PDP/APN readiness,
  failure diagnostics, SMS readiness, and modem functionality.
- Product presets do not include Quectel-specific or SORACOM-specific commands.
- `examples/presets/quectel.toml` exists and contains Quectel-specific
  commands for SIM, signal, network, serving-cell, configuration inspection,
  network time, MBN list diagnostics, and modem power-down.
- `examples/presets/soracom.toml` exists and contains SORACOM APN setup as a
  confirmation-required write preset or template.
- Repository-managed example preset TOML files are loaded through the same
  explicit file preset loader used for add-on presets during verification.
- Default startup loads only product presets; file preset add-ons require
  explicit `--preset-file` or `--preset-dir` locations.
- Duplicate preset names across product presets and loaded file presets fail with an
  actionable error.
- CLI preset lists show preset set labels without depending on color; TUI
  keeps the default product-only view free of preset set labels and uses
  non-selectable source group headers plus `Source: <title>` detail when file
  presets are visible.
- Preset execution uses effective risk, not only the TOML-declared risk.
- A TOML preset cannot downgrade a classified write, persistent, dangerous,
  sensitive, or unknown command to a lower enforcement behavior.
- TUI provides a Controls action as an AT command input route for
  one-off commands that are not saved as presets.
- AT command input accepts ordinary AT command syntax, including quotes,
  commas, semicolons, equals signs, question marks, and command parameters.
- AT command input execution runs through the same classifier, confirmation,
  masking, logging, and transport path as preset execution.
- SMS send and other prompt-required multi-step commands are not treated as
  ordinary one-shot ad-hoc commands.
- The Response action menu copy action copies the current Response body without copying
  pane borders, pane titles, Status, Logs, or line-number UI.
- Response copy follows the visible masking state: masked by default, unmasked
  only while TUI session output masking is off.
- Response copy uses OSC 52 for clipboard write and does not read the
  clipboard, shell out to `pbcopy`, or add a clipboard dependency.
- PTY bridge behavior is still not implemented in this checkpoint.

Review procedure:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- preset list
cargo run -- tui
```

Manual review with explicit preset add-on loading:

```sh
cargo run -- preset list --preset-dir examples/presets
cargo run -- tui --preset-dir examples/presets
```

Manual review for duplicate rejection:

```sh
tmpdir="$(mktemp -d)"
mkdir -p "$tmpdir/presets.d"
cat > "$tmpdir/presets.d/10-a.toml" <<'EOF'
title = "Duplicate A"

[[presets]]
name = "duplicate"
command = "AT"
risk = "safe"
EOF
cat > "$tmpdir/presets.d/20-b.toml" <<'EOF'
title = "Duplicate B"

[[presets]]
name = "duplicate"
command = "AT+CGDCONT=1,\"IP\",\"example\""
risk = "safe"
EOF
cargo run -- preset list --preset-dir "$tmpdir/presets.d"
```

Expected TUI review actions:

```text
Open TUI with `--preset-dir examples/presets`
Expected       Default built-in-only TUI does not show preset set labels or preset set headers
Expected       Mixed preset set TUI leaves default built-in command rows unheaded and groups file presets with non-selectable source headers
Expected       Source headers following another row in the same kind group have one blank separator row before the header and no blank row after it
Expected       Quectel/SORACOM presets are distinguishable from built-in presets without category pollution
Expected       Status is compact under Devices and does not use a full-width band
Expected       Devices shows visible matching USB targets or an explicit no-device message
Commands focus
Up/Down        Move command selection one row
PageUp/Down    Move command selection by the visible command-row capacity
Home/End       Jump to the first or last command
Expected       Lower commands remain reachable and the selected row stays visible
Categories focus
PageUp/Down    Move category selection by the visible category-row capacity
Expected       Lower categories remain reachable when file presets add categories
Logs focus
PageUp/Down    Move saved-log selection by the visible log-row capacity
Expected       Lower saved logs remain reachable and can be opened
Expected       Status shows state and command context only, not a `Keys:` line
Expected       A one-row footer shows short context-sensitive key hints
Expected       Footer hints do not wrap into the Status, Response, Commands, or Logs panes
If no device is visible
Expected       Preset execution and ad-hoc AT sending are blocked
If exactly one device is visible
Expected       The device is auto-selected, detail is shown, and commands can run
If multiple devices are visible
Tab/Left/Right Focus Devices selection
Up/Down        Highlight a visible USB device
PageUp/Down    Move device selection by the visible device-row capacity
Enter          Select the highlighted USB device
Expected       Commands remain blocked before selection and become available after selection
Expected       Command execution uses the selected VID, PID, bus, and address
After running a command on device A
Tab/Left/Right Re-enter Devices selection
Up/Down        Highlight device B
Enter          Select device B
Run modem-response
Expected       The second command uses device B without rewriting device A's displayed response
Expected       Response/Logs/command navigation receive the useful space
Devices focus
Enter          Switch between operation targets and full-USB troubleshooting view using the pane action row
Expected       Diagnostic-only USB devices are visible in full-USB view but cannot be selected for AT sending
Run safe built-in preset
Expected       Response shows the modem response, Status shows concise context
Run available-operators or another long-running safe preset
Expected       available-operators uses a 180s timeout hint unless overridden
Tab/Left/Right Focus Controls
Up/Down        Highlight Timeout
Enter          Open temporary timeout input
Enter          240
Expected       Status shows the selected timeout as 240s before execution
Expected       Status updates while running with a compact timeout-budget label
Expected       Timeout-budget progress bar changes while the command runs
Run or attempt a write-risk preset
Expected       Confirmation is required before USB access
Controls focus
Up/Down        Highlight AT command
Enter          Open ad-hoc input
Enter          AT+CGDCONT=1,"IP","soracom.io"
Expected       The command is accepted as AT syntax and classified before USB access
Expected       Confirmation is required because the command changes APN context
Enter          AT+CSMS?
Expected       The command can run as a one-shot readiness check
Attempt SMS send multi-step command
Expected       It is not treated as an ordinary one-shot command
Run any command that produces a Response
Response focus
Enter          Open Response actions
Up/Down        Highlight Copy response
Enter          Copy the current Response body
Expected       Clipboard content contains the AT command and response body only
Expected       Clipboard content does not include pane borders, Status, or Logs
Expected       The copy-request result is reported without changing the Copy response row label
Up/Down        Highlight Save response
Enter          Save the current Response body
Expected       The action menu shows the Response output folder once as shared context before saving
Expected       Status reports the save result concisely, not as a long path dump
Response focus
Enter          Open Response actions
Up/Down        Highlight Open response folder
Enter          Request opening the Response output folder
Open a masked log from Logs
Response focus
Enter          Open Log view actions
Up/Down        Highlight Copy displayed log
Enter          Copy the masked log body from Response
Expected       Clipboard content does not include Response line-number UI
Up/Down        Highlight Close log view
Enter          Close the opened log view
Logs focus
Enter          Open Log actions
Up/Down        Highlight Open logs folder
Enter          Request opening the logs folder
Expected       Log view actions and Log actions show the Logs folder once as shared context
?              Open modal help
q              Close modal help without quitting
q              Quit
```

Pass criteria:

- Format, test, and clippy commands pass.
- Repository-managed Quectel and SORACOM TOML files exist.
- The example TOML files are loaded through the same explicit file preset
  loader path used by `--preset-dir`.
- `preset list` shows preset set labels; TUI uses source group headers and
  `Source: <title>` detail only when file presets are visible.
- `preset list` shows timeout hints.
- `available-operators` has a 180-second preset timeout hint.
- TUI Controls can set a temporary timeout override and `default` can clear it.
- TUI does not send device-dependent commands when no device is selected.
- TUI auto-selects a sole visible matching device and shows selected-device
  detail.
- TUI requires explicit device selection before command sending when multiple
  matching devices are visible.
- TUI allows reselection of a different visible device after a command
  completes, and subsequent commands use the newly selected device.
- Status does not contain generic keyboard shortcut boilerplate.
- A one-row footer shows context-sensitive key hints and omits lower-priority
  hints instead of wrapping.
- Built-in presets no longer contain Quectel-specific or SORACOM-specific commands.
- Duplicate preset names fail instead of silently overriding.
- Effective risk prevents TOML-declared risk from weakening command
  classification.
- Ad-hoc input accepts normal AT command syntax and applies classification
  before USB access.
- Confirmation-required ad-hoc commands do not send before confirmation.
- Prompt-required multi-step commands are not misrepresented as one-shot
  commands.
- Response action menu copy action copies only the current Response body or
  masked log body.
- Response copy does not include TUI chrome such as borders, pane titles,
  Status text, Logs text, or log line-number UI.
- Response copy does not expose raw values unless TUI session output masking is
  off.
- Global letter shortcuts are limited to `/`, `?`, and `q`; secondary actions
  are available through the relevant focused pane and `Enter`.
- Help is modal: while visible, ordinary pane actions do not run and `Esc`,
  `?`, or `q` close help.
- PTY bridge remains out of scope.

Fail criteria:

- Status remains an oversized pane or full-width band that mostly contains
  blank space.
- TUI appears frozen while a command is running.
- TUI running Status lacks elapsed time, timeout, remaining time, or a
  timeout-budget progress indicator.
- TUI cannot set or display a temporary command timeout.
- TUI requires global single-letter shortcuts for Controls actions such as
  ad-hoc input, timeout, raw export, save, copy, reveal/mask, or clear.
- `available-operators` still uses only the default 30-second timeout.
- TUI presents host-side read interruption as successful command cancellation
  without a specified session abort/reconnect/resync recovery path.
- Devices remains only a static placeholder when USB candidates are visible.
- A multiple-device TUI session silently chooses a device before explicit user
  selection.
- Preset execution or ad-hoc AT sending is possible when no matching device is
  visible or no active execution device has been selected.
- A sole visible device is not auto-selected at startup.
- Selected-device detail is not visible after selection.
- Multiple visible matching devices cannot be selected and reselected in the
  TUI.
- Full-USB troubleshooting view requires a global letter shortcut instead of a
  Devices pane action row.
- Devices, Categories, Commands, or Logs cannot reveal lower items when the
  focused list has more rows than visible pane height.
- `PageUp`, `PageDown`, `Home`, or `End` only work for Response and not for
  focused list panes.
- Status contains generic keyboard shortcut boilerplate such as `Keys: ...`.
- Keyboard hints wrap into Status or another content pane instead of staying in
  a one-row footer.
- Help lets background actions execute while the overlay is visible.
- The saved history/session list pane is still titled `History`.
- Built-in presets still include Quectel-specific or SORACOM-specific commands.
- Example preset TOML files are missing or are parsed only by a test-only path.
- Explicit `--preset-dir` loading ignores `*.toml` files in that directory.
- Duplicate preset names silently override each other.
- A user TOML risk declaration can downgrade classifier output.
- Preset set labels are missing from CLI preset lists or mixed TUI preset lists.
- Ad-hoc input rejects ordinary AT command syntax such as quoted APN strings.
- A write, persistent, dangerous, or unknown ad-hoc command sends before
  confirmation.
- SMS send or another prompt-required command is treated as a completed
  one-shot command.
- Response copy includes TUI borders, pane titles, Status, Logs, or
  line-number UI.
- Response copy exposes raw sensitive values while the Response is in masked
  view.
- PTY bridge work is started before this checkpoint is approved.

## Checkpoint 12 User Review

Checkpoint 12 review target:

- PTY bridge first implementation is acceptable for `screen` / `cu` style
  human-operated workflows.
- Bridge behavior follows the approved runtime design in `docs/SPEC.md` and
  OQ-019.
- Running-command interruption, host-side read abort, USB reconnect, and AT
  resync remain out of scope.

Agent-verified checks:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- devices --help
cargo run -- bridge --help
cargo run -- bridge --symlink /private/tmp/atctl-bridge-no-device-20260619-check --vid 0x0000 --pid 0x0000
ls -l /private/tmp/atctl-bridge-no-device-20260619-check
```

Expected agent-verified state:

- Format, tests, and clippy pass.
- `devices --help` shows `--all-usb` as the full USB troubleshooting view.
- `bridge --help` shows `--symlink`, `--replace-symlink`, USB selectors,
  endpoint overrides, and default `--timeout 30`.
- `bridge --help` shows the first-time workflow: run `atctl devices`, choose
  from current runtime AT operation target output, prefer `--bus <BUS>
  --address <ADDRESS>`, use VID/PID only when unique in the current output, and
  use `atctl devices --all-usb` only for full USB visibility troubleshooting.
- Fake VID/PID bridge startup fails before symlink creation.
- The fake symlink path does not exist after the failure.

Recommended real-device review procedure:

```sh
cargo run -- devices
cargo run -- devices --all-usb
cargo run -- inspect --bus <BUS> --address <ADDRESS>
cargo run -- bridge --symlink /tmp/atctl --bus <BUS> --address <ADDRESS>
```

Choose `BUS` and `ADDRESS` from the current `cargo run -- devices` operation
target output. Use `cargo run -- devices --all-usb` only to confirm full USB
visibility if the expected target is missing. Use `--vid` and `--pid` only if
that pair is unique in the same target output. The review must not assume
validation-target VID/PID values as prior knowledge.

For example, if `cargo run -- devices` prints:

```text
EG25-G 0x2c7c:0x0125 bus=1 address=4
```

then run:

```sh
BUS=1
ADDRESS=4

cargo run -- inspect --bus $BUS --address $ADDRESS
cargo run -- bridge --symlink /tmp/atctl --bus $BUS --address $ADDRESS
```

Open another terminal:

```sh
screen /tmp/atctl 115200
```

To quit `screen` after the review commands, press `Ctrl-A`, then `K`, then
`y`.

Inside `screen`:

```text
AT
ATI
AT+CIMI
ATE0
abc
ATE0
write
```

Expected real-device state:

- Bridge startup prints the symlink path, PTY target, `screen` command, and the
  note that `115200` is a compatibility value.
- `AT` returns the ordinary modem response, normally `AT` and `OK`.
- `ATI` returns modem information.
- `AT+CIMI` returns masked IMSI output by default.
- The first `ATE0` attempt opens a bridge confirmation prompt and does not send
  after `abc`.
- The second `ATE0` attempt sends only after typing `write`.
- Quitting the bridge with Ctrl-C removes `/tmp/atctl` if it still points to
  the bridge-created PTY.
- Quitting the `screen` client with `Ctrl-A`, then `K`, then `y` stops the
  bridge cleanly, runs symlink cleanup, and does not report `failed to write
  PTY` or `failed to read PTY` as a transport error.

Pass criteria:

- The bridge can be started against a selected visible USB modem.
- The symlink is created only after USB selection succeeds.
- Safe and sensitive commands execute from `screen`.
- Sensitive output is masked by default.
- Write-risk commands do not send before exact typed risk confirmation.
- Quitting the `screen` client is treated as normal bridge shutdown with
  symlink cleanup.
- Bridge cleanup removes the created symlink on normal Ctrl-C shutdown.
- At this historical checkpoint, no raw logging, session-wide raw mode, or
  session abort/reconnect/resync behavior was introduced. Current OQ-021 raw
  diagnostic export remains a separate explicit file export and does not add a
  PTY session-wide raw display mode or session abort/reconnect/resync behavior.

Fail criteria:

- Bridge creates or replaces a symlink before USB selection succeeds.
- Existing regular files or directories are overwritten.
- Sensitive PTY output is unmasked by default.
- Write, persistent, dangerous, or unknown commands send without exact typed
  risk confirmation.
- Quitting the PTY client reports `failed to write PTY`, `failed to read PTY`,
  or another transport error instead of cleanly stopping.
- Multiple-client behavior is claimed as supported.
- `screen /tmp/atctl 115200` is documented as physical UART speed.
- Bridge continues sending commands after a USB transport error or timeout.

## Checkpoint 12.6 User Review

Checkpoint 12.6 review target:

- Sequences are acceptable as the product feature for multi-step AT operations.
- Product-provided standard SMS send/read/reply Sequences are available without
  requiring the user to author TOML.
- User-authored Sequence TOML is available only when loaded from explicit
  `--sequence-file` / `--sequence-dir` paths.
- Quectel TCP/IP and SORACOM TCP endpoint checks are available as explicitly
  loaded repository-managed example Sequences, not as default vendor-neutral
  product Sequences.
- TUI Sequence support does not add another permanent pane. The existing layout
  remains Devices/Status, Categories, Commands / Sequences, Controls, Response,
  and Logs.

Implemented in this checkpoint:

- Shared Sequence model and TOML loader.
- Shared Sequence engine for prompt waits, payload writes, Ctrl-Z or ESC
  terminators, URC waits, final response waits, per-step timeouts, total
  timeout, masking, risk aggregation, raw diagnostic export, and transcripts.
- CLI `sequence list` and `sequence run`.
- TUI `Commands / Sequences` rendering, `Run Sequence` modal, Status step
  context, Response transcript, and Controls behavior.
- TUI executable-item display grammar keeps default command and Sequence rows
  unheaded, groups only non-default file/repository definitions by their TOML
  `title`, and uses `Source: <title>` for selected non-default details.
- Product-provided standard SMS send/read/reply Sequences, including
  reply-by-index through sender extraction from `AT+CMGR`.
- Repository-managed Quectel TCP/IP and SORACOM TCP endpoint example Sequence
  definitions under `examples/sequences`.
- Active input/review items for SMS send destination/body, SMS read index, SMS
  reply index/body, TCP destination, and TCP payload.
- Structured Sequence step results with `analysis`, decoded SMS body sections,
  TCP counter/read analysis, `evidence`-derived analysis, and success notes.
- PTY bridge prompt-capable manual multi-step behavior for prompt-required
  commands. After typed risk confirmation, the bridge waits for the prompt,
  accepts the next PTY line as payload, appends Ctrl-Z, and returns the final
  masked response.

Agent-verified checks:

```sh
cargo fmt --check
cargo test tui::tests
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- sequence list
cargo run -- sequence list --sequence-dir examples/sequences
cargo run -- sequence run sms-send-check --help
cargo run -- tui --help
rg -n "Preset set:|Sequence set:|Source: Built-in|Source: Product|Add-on:" src README.md docs examples -g '!target'
```

Current agent-verified state as of 2026-06-23:

- `cargo fmt --check` passed.
- `cargo test tui::tests` passed: 78 tests, including the maximum mixed
  Commands/Sequences plus non-default source title case.
- `cargo test` passed: 210 tests.
- `cargo clippy --all-targets --all-features -- -D warnings` passed.
- `sequence list` shows product-provided standard SMS Sequences, including
  `sms-reply-check` with `index,message(sensitive)`.
- `sequence list --sequence-dir examples/sequences` also shows the Quectel and
  SORACOM example Sequence sets without making Quectel or SORACOM generated
  categories.
- `sequence run sms-send-check --help` shows how Sequence values and risk
  acknowledgement are supplied.
- `tui --help` shows Sequence location flags for TUI loading.
- Mock transport tests cover prompt waits, payload writes, URC waits, SMS UCS2
  decode/masking, SMS reply sender derivation, TCP `QISEND` counter evidence,
  TCP `QIRD` no-data evidence, timeouts, masking, raw diagnostic export,
  transcript output, TUI Sequence input, CLI Sequence parsing/loading, and PTY
  prompt-required payload handling.
- Residual text search found old TUI labels only in negative assertions,
  prohibitive specification text, and historical verification records.

Recommended real-device review procedure:

```sh
cargo run -- devices
cargo run -- sequence list
cargo run -- tui
```

Inside the TUI:

- Confirm that the executable-item pane is `Commands / Sequences`.
- Confirm that standard SMS Sequences are selectable from the `sms` category.
- Confirm that default command and Sequence rows are not shown under
  `Product presets` or `Product Sequences` headers.
- Confirm that loaded file presets and repository-managed Sequence definitions
  use their TOML `title` values as non-selectable source headers without an
  `Add-on:` prefix.
- Select `sms-send-check` and confirm that a `Run Sequence` modal shows the
  current destination and message body before USB access.
- Select `sms-reply-check` and confirm that the modal shows the SMS storage index
  and reply body before USB access; the run should derive the reply destination
  from the `AT+CMGR` sender.
- Select `sms-read-message` and confirm that the modal reviews the SMS storage
  index and requires write-risk confirmation because the modem may mark unread
  material as read.
- Confirm that Status shows compact current step context while Response is the
  transcript surface.
- Confirm that Controls still provides existing operations such as timeout,
  raw export, copy/save, output masking, and clear response without becoming a
  separate Sequence-only pane.

Optional real-device or network/SMS review:

```sh
cargo run -- sequence run sms-send-check --bus <BUS> --address <ADDRESS> ...
cargo run -- sequence run sms-receive-check --bus <BUS> --address <ADDRESS>
cargo run -- sequence run sms-read-message --bus <BUS> --address <ADDRESS> --param index=<INDEX> ...
cargo run -- sequence run sms-reply-check --bus <BUS> --address <ADDRESS> --param index=<INDEX> --param message=<MESSAGE> ...
cargo run -- sequence run quectel-ping-check --sequence-dir examples/sequences --bus <BUS> --address <ADDRESS> --param host=<HOST>
cargo run -- sequence run quectel-tcp-send-check --sequence-dir examples/sequences --bus <BUS> --address <ADDRESS> ...
cargo run -- sequence run soracom-ping-check --sequence-dir examples/sequences --bus <BUS> --address <ADDRESS>
cargo run -- sequence run soracom-unified-endpoint-tcp-send-check --sequence-dir examples/sequences --bus <BUS> --address <ADDRESS> ...
```

Use actual recipient, message body, index, host, port, and payload values only
when the reviewer intentionally wants to send SMS or external data. These values
are sensitive and must remain masked by default after the pre-send review.
Quectel and SORACOM ping review requires received `+QPING:` replies. Quectel
and SORACOM TCP data-send review requires a reachable endpoint, response data,
or remote service logs if end-to-end data exchange is being verified.

Pass criteria:

- Sequence definitions load deterministically and duplicate names fail.
- Product-provided standard SMS send/read/reply Sequences do not require user
  TOML authoring.
- User and repository-managed Sequence definitions require explicit loading or
  configured user locations.
- TUI layout remains the approved topology without another permanent pane.
- Sequence input is collected before USB access.
- SMS receive/read output decodes supported bodies before masking and does not
  expose decoded bodies in normal Response, history, saved output, or JSON.
- SMS reply uses a reviewed SMS storage index and extracts the sender from
  `AT+CMGR`; it is not a duplicate manual-recipient SMS send.
- Risk confirmation and masking are enforced for SMS bodies, phone numbers,
  payloads, credentials, and raw diagnostic export.
- Response transcript separates sent material, modem response, decoded SMS,
  atctl analysis, notes, and result so a human can distinguish SMS submit
  evidence, TCP socket-open evidence, TCP send counter evidence, no-data `QIRD`
  evidence, and end-to-end receive evidence without treating derived analysis
  as modem output.
- CLI JSON Sequence output includes structured step `analysis` results and
  notes while preserving default masking.
- `send` and `preset run` remain one-shot surfaces and do not silently execute
  Sequence names.

Fail criteria:

- Quectel or other vendor-specific data-send Sequences become default
  vendor-neutral product actions without approval.
- Categories contain generated vendor/source values such as `quectel` or
  `soracom`.
- Sequence execution is implemented by concatenating AT commands into one
  one-shot send string.
- TUI adds another permanent pane or moves Sequence input into Status.
- Sensitive Sequence parameters or payloads are shown or logged raw by default.
- `OK` after a vendor socket-open command is described as complete external
  data-send success when the vendor command reports success through a later
  URC or when remote receive evidence is absent.

## Resume Instructions

When resuming in a new session:

1. Read `docs/SPEC.md`.
2. Read this file.
3. Check `docs/OPEN-QUESTIONS.md`; implementation must not continue if any
   new unresolved decision exists.
4. Inspect current files before editing.
5. Continue from the first `in progress` or `pending` checkpoint.
