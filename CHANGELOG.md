# Changelog

All notable project changes should be recorded in this file.

## 0.2.0 - 2026-07-13

### Added

- Added a repository-pinned Rust toolchain plus Dependabot, scheduled CI, local
  drift checks, and update commands for Rust and fixed maintenance tools.
- Added `--no-log` to direct send, preset execution, Sequence execution, and
  TUI startup to suppress new masked history and session logs for one
  invocation without disabling explicit raw diagnostic export.
- Added release archives with a versioned top-level directory containing the
  executable, the project MIT license, and generated third-party notices for
  target-specific Rust dependencies and the dynamically linked `libusb`
  dependency.

### Changed

- Updated the minimum supported Rust baseline from 1.96 to 1.97 and selected
  Rust 1.97.1 for development, CI, and release builds to include the upstream
  LLVM miscompilation fix.
- Standardized generated history and session logs on the XDG state directory,
  including `XDG_STATE_HOME` overrides and the `$HOME/.local/state` fallback.
- Changed default masking to classify product-known phone/SMS fields and
  definition-declared sensitive values by meaning and provenance rather than
  length, while retaining short operational values and one consistent masked
  artifact across normal output, JSON, TUI, PTY, history, and session logs.
- Documented the complete masked-log lifecycle, including aggregate history,
  per-execution session logs, manual retention and deletion, and separate raw
  diagnostic exports.
- Added state-aware TUI file actions: selected history and session logs can be
  opened in Response or revealed directly in Finder, and opened logs keep copy,
  reveal, and close actions.
- Replaced fixed-directory Response snapshots with explicit Response export.
  The TUI asks for a destination folder on every export, and `send`,
  `preset run`, and `sequence run` accept `--export-response <PATH>` without
  changing normal stdout. Exports follow foreground masking and never overwrite
  an existing file.
- Replaced scalar command-risk labels across CLI, TUI, presets, Sequences, PTY,
  JSON, and logs with product-derived execution reviews that separate
  assessment, effects, data handling, reversibility, consequences, and
  confirmation. Unknown operations now require distinct explicit
  authorization. The review distinguishes SMS read-status changes, Quectel
  receive-buffer reads, counter queries, and prompt/payload sends; one-shot
  surfaces reject prompt-required operations before USB access. Unmasked
  Response copy and export continue to require action-specific confirmation
  before clipboard or file output.
- Reworked execution-review presentation around user decisions: human CLI and
  TUI discovery rows now prioritize item identity and append the same
  exception-only `[confirm]` action-flow tag, align it in a conditional fixed
  trailing lane, and reclaim the lane when a result set has no confirmation
  rows. TUI uses a dedicated accessible confirmation-flow color with a bold
  no-color fallback, while human CLI output remains plain text;
  known and unknown confirmations use consequence-oriented sections; Sequence
  input, review, and warning phases use distinct focus/warning styling; and
  modal controls now use `Esc` consistently for cancel, close, or back. Lowercase
  `q` quits only from the idle base screen, remains ordinary text in visible
  input fields, and has no hidden effect in non-editable modals or while an
  operation is running.
- Standardized executable discovery on stable one-row command and Sequence
  shapes without repeated `AT:` or `Purpose:` delimiters, a read-only scrollable
  `d Details` view, complete confirmation tags that survive selection and
  constrained widths, actionable empty states, and safe resize fallback. TUI
  candidate-acquisition actions use the same tag. Every Sequence
  modal phase now pins the current value, candidate action, or confirmation
  decision first, keeps complete name/purpose and supporting context in a lower
  scrollable Details region, and leaves operation keys at the bottom.
  Multi-value input keeps a right-aligned `Value <current> of <total>` position
  through value and candidate-action phases, separates the active label from
  that counter, and states the exact next `Enter` outcome without coupling
  Details paging to value navigation. One `Enter` on a loaded candidate now
  accepts it and continues to the next value or final review; provider
  completion still returns to value selection without choosing a value.
- Added pre-publication release checks for Apple Silicon architecture, archive
  contents and permissions, checksums, dependency notices, and dynamic linkage
  to Homebrew `libusb`.

### Removed

- Removed the per-user `config.toml` contract and persistent USB/log defaults;
  USB overrides are explicit runtime options and log relocation uses
  `XDG_STATE_HOME`.
- Removed the ineffective `preset run --continue-on-error` option. Preset
  execution accepts one preset name and runs one AT command; named multi-step
  workflows use `sequence run`.

## 0.1.0 - 2026-07-05

Initial release of `atctl`.

### Release scope

- Release target: Apple Silicon macOS (`aarch64-apple-darwin`).
- Validated hardware recorded for this release: SORACOM Onyx LTE USB Dongle
  (Quectel EG25-G).
- Normal end-user install path: Homebrew tap formula
  `brew install uchimanajet7/atctl/atctl`.
- GitHub Release archives and checksums are release/manual artifacts, not the
  normal install path.

### Added

- Added CLI and TUI AT-command execution for USB cellular modems, including
  device discovery, USB inspection, selectable targets, direct `send`, preset
  execution, Sequence execution, and PTY bridge support.
- Added safety controls for AT-command work: risk classification, explicit
  confirmations for write/dangerous actions, output masking, sensitive value
  handling, and explicit raw diagnostic export.
- Added built-in commands, repository-managed preset examples, and
  repository-managed Sequence examples for modem identity, SIM/network/PDP
  checks, SMS send/list/read/reply workflows, Quectel ping/TCP checks, and
  SORACOM ping/Unified Endpoint TCP checks.
- Added structured Sequence transcripts that separate commands, payloads, modem
  responses, decoded SMS bodies, derived analysis, success notes, and final
  results.
- Added masked history/session logging, response saving, log viewing, and TUI
  action menus for response and log artifacts.
- Added project documentation for installation, safety, preset/Sequence
  definitions, troubleshooting, development, packaging, open decisions, and
  implementation status.
- Added MIT licensing, Cargo package metadata, explicit Cargo source package
  include rules, and a source repository GitHub Release workflow for the Apple
  Silicon macOS release artifact and checksum.
