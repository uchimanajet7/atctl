# Changelog

All notable project changes should be recorded in this file.

## Unreleased

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

- Updated the Rust compiler and Cargo baseline from 1.96 to 1.97.0 and refreshed
  compatible locked dependencies.
- Standardized generated history and session logs on the XDG state directory,
  including `XDG_STATE_HOME` overrides and the `$HOME/.local/state` fallback.
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
- Unified TUI risk presentation on one exact classification label per command
  or Sequence, with masking state, expected effects, and confirmation actions
  shown separately. Unmasked Response copy and export now require explicit
  action-specific confirmation before clipboard or file output.
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
