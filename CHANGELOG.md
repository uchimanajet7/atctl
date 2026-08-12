# Changelog

Notable changes to `atctl` are documented here for users and source builders.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/).

## 0.2.0 - 2026-08-08

This release makes logging, Response export, execution review, and Sequence
input more explicit, and removes persistent configuration in favor of runtime
options and XDG state.

### Added

- Added `--no-log` to `send`, `preset run`, `sequence run`, and TUI startup. It
  suppresses new masked history and session logs for that invocation without
  deleting existing logs or disabling an explicitly requested raw diagnostic
  export.

### Changed

- Source builds now require Rust 1.97 or later.
- Release archives now use a versioned top-level directory and include the
  executable, the MIT license, and target-specific third-party notices.
- Normal history and session logs can now be relocated only with
  `XDG_STATE_HOME`; otherwise they remain under `~/.local/state/atctl`. The
  former persistent `log_dir` setting is no longer used.
- Sensitive-data handling is now more explicit. Default masking recognizes
  product-known phone and SMS fields and definition-declared sensitive values
  by meaning while preserving short operational values across normal output,
  JSON, TUI, PTY, history, and session logs. Raw diagnostic export warnings now
  identify the wider range of potentially sensitive data they can contain.
- TUI log actions now open selected history and session logs in Response or
  reveal them in Finder; an opened log can be copied, revealed, or closed.
- Replaced fixed-directory Response snapshots with explicit Response export.
  The TUI asks for a destination folder on every export, and `send`,
  `preset run`, and `sequence run` accept `--export-response <PATH>` without
  changing normal stdout. Exports follow foreground masking and never overwrite
  an existing file.
- Execution confirmation now uses a structured review across CLI, TUI, Presets,
  Sequences, PTY, JSON, and logs. It explains effects, handled data,
  reversibility, and possible consequences; unknown operations require
  additional explicit authorization. Copying or exporting unmasked Response
  content requires confirmation for that action.
- TUI command and Sequence lists, details, input, and confirmation flows now use
  clearer confirmation indicators, consistent `Esc` and `q` behavior,
  scrollable and wrapped content, resize-safe layouts, and actionable empty
  states.
- Sequence input now shows each value with its source, default, and hint in the
  TUI and interactive CLI. Candidate-backed `sms-message` and `pdp-context`
  values can be obtained and selected without leaving the input flow; opening
  the flow performs no hidden modem I/O, and non-interactive errors show the
  exact retry parameters.

### Removed

- Removed per-user `config.toml` support and its persistent device, profile, UI,
  and log settings. Supply device and operation options when invoking `atctl`,
  and use `XDG_STATE_HOME` to relocate normal logs.
- Removed the ineffective `preset run --continue-on-error` option. Preset
  execution accepts one preset name and runs one AT command; named multi-step
  workflows use `sequence run`.

### Fixed

- CLI Preset and Sequence lists now emit complete, reusable names without
  terminal-width truncation; TUI executable rows use one terminal-cell-aware
  name column for the current result so command and purpose previews remain
  aligned, with previews shortened before names when width is constrained.
- Corrected Quectel TCP and UDP result interpretation. TCP acknowledgement is
  reported only for an explicit zero-length `QISEND` counter query; `QIRD`
  counters, normal reads, and UDP response headers are distinguished, and
  returned buffered data remains masked.

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
