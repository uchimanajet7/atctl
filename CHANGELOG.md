# Changelog

All notable project changes should be recorded in this file.

## Unreleased

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
