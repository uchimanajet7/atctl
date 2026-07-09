# Development

This document defines the local development setup for `atctl`.

## Local Development Environment

Local development is documented for Apple Silicon Macs. Other platforms may
build later, but this document does not present them as validated local
development environments.

## Required Tools and Libraries

Install dependencies separately so their purpose is clear.

```sh
# Rust compiler and Cargo, needed to build atctl from source.
brew install rust

# Native USB access library used by atctl through the Rust rusb crate.
brew install libusb

# Build-time helper. Homebrew's pkgconf formula provides the pkg-config command
# used by libusb1-sys to locate libusb compiler and linker metadata.
brew install pkgconf
```

If you use `rustup` instead of Homebrew Rust, that is acceptable. `libusb` must
still be available through Homebrew for the initial Apple Silicon Mac setup.
The active Rust baseline is the `rust-version` declared in `Cargo.toml`.
Currently this is Rust 1.96 with Edition 2024. When using `rustup`, refresh the
stable toolchain before dependency maintenance:

```sh
rustup update stable
rustc -Vv
cargo -V
```

## Optional Tools

```sh
# Source control tool, needed if it is not already installed.
brew install git

# Optional task runner, only useful if the repository later adds a justfile.
brew install just

# Optional TOML formatter/checker for config examples and presets.
brew install taplo
```

## Clone

```sh
git clone https://github.com/uchimanajet7/atctl.git
cd atctl
```

## Build

```sh
cargo build --locked
```

## Test Without Hardware

The implementation must provide unit tests that do not require a physical modem.

```sh
cargo test --all-features --locked
```

Tests should cover at least:

- AT response parser
- Sensitive value masking
- Preset risk classification
- Config parsing
- CLI parsing
- Mock transport behavior

Product tests must verify product behavior, runtime contracts, and user-facing
outputs. Do not add coding-agent process checks, fixed-phrase policy checks, or
conversation-discipline checks to the product crate or the normal `cargo test`
suite. If a prose or policy check is needed, keep it in a separately approved
process tool outside the product runtime source and product test suite.

## Product Surface Gate

Before proposing, documenting, or implementing AT execution behavior, identify
the user-facing use case and check every relevant production surface:

- `atctl tui`
- `atctl send`
- `atctl preset run`
- `atctl bridge --symlink <PATH>`

The TUI is the main interactive product surface. The CLI commands and PTY bridge
are also production features. Do not describe them as validation-only,
test-only, auxiliary, fallback, or second-class entry points unless the current
specification or a direct user instruction says so.

For each relevant surface, record whether the behavior is already implemented,
specified but unimplemented, intentionally not applicable, or temporary staging.
Temporary staging is not product completeness. A feature that affects AT
execution, diagnostics, presets, raw diagnostic export, SMS, data-send, or
multi-step command handling must not be called complete while an applicable
production surface is silently omitted.

Implementation artifacts, internal data structures, file formats, execution
engines, and project terminology are not user responsibilities by default.
Before a proposal, document, or implementation plan says that the user must
author, create, maintain, supply, or operate something, identify whether that
responsibility is product-provided behavior, a repository-managed example,
user-authored extension, operator action, or implementation detail. Do not turn
an internal mechanism into a user prerequisite unless the current specification
or the user explicitly makes it one.

Built-in workflow definitions are product-provided execution definitions, not a
prerequisite that users must author before using standard SMS checks.
User-authored workflow definitions are an extension point for additional,
special, project-local, or verification workflows. Development notes, proposals,
and implementation plans must preserve that distinction.

When changing preset or Sequence loading, display, or execution, preserve origin
metadata while using one shared product contract after validation. Standard
product-provided items, repository-managed examples, and user-authored
extensions must not become one responsibility class, but they must not fork into
separate risk, masking, logging, raw export, duplicate-name, or execution
semantics.
Repository-managed examples and user-authored extensions must be loaded only
from explicit per-invocation file or directory flags. Do not add default
directory auto-loading for add-on Presets or Sequences; default startup must
show only product-provided definitions.

Use an internal definition/draft normalization boundary for product-provided and
TOML-loaded execution definitions when it improves readability or prevents
drift. Do not force product-provided built-ins through the runtime TOML file
loader merely for symmetry, and do not make users author definition files before
using standard product workflows.

Use the product-facing term `Sequence`. Presets are
one-shot AT command definitions. Sequences are multi-step AT actions that may
include prompt waits, payload writes, URC waits, per-step timeouts, and a
Response transcript. Product-provided standard SMS Sequences must not require a
user-authored TOML file before ordinary use. Repository-managed Quectel
data-send examples are loaded explicitly and must not be treated as default
vendor-neutral product behavior.

## Current Surface Evidence Gate

Before proposing or changing user-facing naming, labels, terminology, grouping,
or wording, check the current user-facing surfaces that the answer refers to.
For implementation notes and pull-request planning, treat "answer" as the
proposal, document, or implementation plan being written. Check:

- TUI pane titles, status text, help text, and footer text from current source
  or a current run when needed.
- CLI command names, subcommands, options, and help output from current source
  or current command output.
- Relevant specification and user-facing documentation.

Record current implemented or specified wording separately from proposed,
future, translated, or internal wording. Do not describe a term as current TUI,
CLI, or documentation wording unless the checked surface actually uses it. When
the user asks for evidence, or when external convention matters, cite
current-source references before recommending a concrete term.

## Whole-Surface UI Grammar Gate

TUI changes must not be designed as isolated conditional tweaks. Before changing
pane structure, list grouping, group headers, status rows, controls rows, labels,
or wording, define the whole-surface UI grammar for the affected pane or
workflow. The grammar must state each visible hierarchy level, what concept it
represents, when it is shown or hidden, and the allowed user-facing labels.

For the executable-item surface, check representative adjacent states together:
built-in-only commands, mixed built-in/file presets, mixed command/Sequence
results, and loaded repository-managed examples when relevant. Do not satisfy
separate local requirements by producing inconsistent visual hierarchy across
those states. A group header position or style must not switch between unrelated
meanings such as item kind and source set unless a stable parent hierarchy makes
that distinction clear.

If the current specification contains local display rules but lacks a combined
grammar for the affected surface, stop and report the specification gap before
implementation. Verification for TUI grouping or wording changes must include
cross-state render-buffer or snapshot coverage that compares the affected
adjacent states, not only a single local case.

## TUI Status Content Gate

Compact Status is current state and execution context only. Before changing
Status rows or wording, classify each proposed line as state/context,
operation output, action availability, action result, help, confirmation
explanation, or implementation detail.

Allowed compact Status content is limited to current state, active or selected
command/Sequence identity, relevant command text, current Sequence step, risk,
timeout/progress, meaningful output masking state, selected device context, raw
export state, viewed-log state, and concise completion or failure result.

Normal operation explanations, action semantics, confirmation rationale, help
text, keyboard hints, copy/save behavior descriptions, Sequence summaries,
Evidence/analysis notes, response bodies, transcripts, and implementation
details must not be added to compact Status. Put action availability in Controls
rows or the relevant pane action menu, action results in nearby action-surface
feedback, longer explanations in confirmation dialogs/help/docs, and modem or
analysis output in Response or an approved detail surface.

Do not render arbitrary execution-error strings or implementation detail fields
as compact Status rows such as `Detail:`. Completed and failed states may show a
short `Result:` row, but full failure reasons and troubleshooting detail belong
in Response or another approved detail surface.

If proposed Status text explains how another control works rather than what
state the product is currently in, stop and move it out of Status before
editing. Verification for Status wording changes must include render-buffer
coverage for the affected state and negative assertions for non-state
explanatory text such as `Copy:`, `Keys:`, `Summary:`, `Evidence:`, and
free-form `Detail:` error text.

## Sequence Output Origin Gate

Before changing Sequence transcript or JSON output, classify each output line or
field by origin: operator-sent command/payload, modem response, atctl-derived
decoding or analysis, Sequence success note, or execution result.

Text transcripts must keep those origins visually separate with stable section
labels:

- `Command:` or `Payload:` for operator-sent material.
- `Modem response:` for modem-returned lines.
- `Decoded SMS:` for decoded SMS body values.
- `Analysis:` for atctl-derived interpretation, including material generated
  from a Sequence definition `evidence` field.
- `Notes:` for Sequence success notes.
- `Result:` for final execution status and duration.

Section blocks in normal text transcripts must be separated by a single blank
line. Do not add decorative divider lines, and do not insert extra spacing into
the modem-returned content inside `Modem response:`.

When a command or Sequence has failed before normal response output exists, the
Response body must start with `Result: failed`, then a blank line, then the
specific failure text. This keeps the failed before normal response state
visible without requiring compact Status or color; the product must not rely on
compact Status or color alone for that failure signal.

Do not render the literal `Evidence:` prefix in normal Sequence text
transcripts, TUI Response, saved Response, history, or session logs. CLI JSON
Sequence output MUST expose derived interpretation as `analysis`, not
`evidence`.
Verification for Sequence output changes must include transcript and JSON
negative assertions for `Evidence:` / `evidence` and positive assertions for the
approved origin labels.

## Development Change Boundary

Before changing product-facing behavior, identify the user-facing use case, the
affected production surfaces, the product specification requirements, and the
verification owner. Product behavior belongs in `docs/SPEC.md` and product
tests. Coding-agent operating rules belong in `AGENTS.md`, incident records,
memory, or a separately approved process check, not in product source or normal
product tests.

## File Preset Development

Repository-managed file preset examples are part of the implementation surface,
not documentation-only samples. When multi-file preset loading is implemented,
the Quectel and SORACOM example TOML files must be checked through the same
loader path used for file presets, for example with
`cargo run -- preset list --preset-dir examples/presets` during verification.
The CLI list verification must include the `source-path` column, and CLI run
verification must cover external file preset notice output before USB access
even when `--yes --risk-ack <risk>` is supplied.

The example preset files must remain human-editable TOML and must declare risk
for every preset. Tests must ensure that declared risk does not downgrade the
command classifier's effective risk.

Built-in preset coverage should be checked against the SORACOM advanced
data-send/receive troubleshooting AT checkpoints when standard workflow
presets change. The current expected core coverage includes `ATI`, `AT+CIMI`,
`AT+WS46?`, `AT+WS46=?`, `AT+COPS?`, `AT+COPS=?`, `AT+COPS=3,2`,
`AT+COPS=0`, `AT+CREG?`, `AT+CGREG?`, `AT+CEREG?`, `AT+CREG=2`,
`AT+CGREG=2`, `AT+CEREG=2`, `AT+CEREG=3`, `AT+CEREG=5`, `AT+CSQ`,
`AT+CESQ`, `AT+CESQ=?`, `AT+CGDCONT?`, `AT+CGAUTH?`, `AT+CGAUTH=?`,
`AT+CGATT?`, `AT+CGACT?`, `AT+CGPADDR`, `AT+CGPADDR=?`, `AT+CGCONTRDP`,
`AT+CEER`, `AT+CMEE?`, `AT+CMEE=2`, `AT+CPAS`, and standard `AT+CFUN` modem
functionality presets. Vendor-specific modem control such as `AT+QPOWD` and
vendor-specific diagnostics such as `AT+QINISTAT`, `AT+QPINC?`, `AT+QSPN`,
`AT+QLTS`, `AT+QMBNCFG="List"`, and Quectel `AT+QCFG` network scan mode
presets must remain in repository-managed file preset examples, not built-in
presets.

## Sequence Development

Sequence implementation must be treated as application feature work before
release, not as packaging work. The implementation plan must cover these
surfaces together:

- TUI `Commands / Sequences` selection, `Run Sequence` modal, compact Status
  step/result context without Sequence summary text, Controls behavior, and
  Response step transcript.
- CLI `atctl sequence list` and `atctl sequence run <SEQUENCE>`.
- `atctl send` and `atctl preset run` staying one-shot surfaces with clear
  errors if a Sequence name is supplied there.
- PTY bridge prompt-capable manual operation for prompt-required commands, or a
  documented and approved product difference before completeness is claimed.

Tests should cover at least:

- Sequence TOML parsing for explicit `--sequence-file` and
  `--sequence-dir` locations.
- Sequence TOML parsing for pre-send `review` items, `success_notes`, and
  per-step `evidence` text rendered as `Analysis:`.
- Explicit `--sequence-file` and `--sequence-dir` loading.
- CLI `sequence list` `source-path` output and CLI `sequence run` external
  definition notice output before USB access, including the
  `--yes --risk-ack <risk>` path.
- Duplicate Sequence name rejection.
- Required parameter validation and sensitive parameter masking.
- Active input/review rendering for SMS send destination/body, SMS read index,
  SMS reply index/body, and TCP destination/PDP context/socket ID/payload/read
  length before USB access, while default Response output, logs, saved output,
  and JSON remain masked.
- Sequence value-resolution metadata for defaults, user-entered values,
  modem-confirmed values, selected SMS storage indexes, selected standard PDP
  context IDs, editable vendor-specific socket IDs, derived reply recipients,
  Sequence-provided destinations, before-running notes, and external
  prerequisites.
- CLI and TUI missing-value behavior using the same Sequence `default`,
  `source`, `candidate`, `hint`, and `before_running` metadata.
- TUI `source=select` and candidate-backed values as same-modal candidate
  selection, including SMS read/reply index candidates from known `AT+CMGL` /
  `sms-receive-check` message rows. Current product-known candidate names are
  `sms-message` for SMS storage indexes and `pdp-context` for standard
  `AT+CGACT?` / `AT+CGDCONT?` output. A hint-only implementation is not
  complete for candidate-backed values when the product knows a candidate
  acquisition path.
- TUI candidate sets are same-session values from explicitly executed commands
  or Sequences. Opening a `Run Sequence` modal must not perform hidden modem or
  network I/O to populate candidates, and render-buffer coverage should prove
  that candidate provenance and count are visible.
- TUI candidate action keyboard behavior: when no candidates are loaded for an
  active candidate-backed value, the modal shows selectable product actions that
  obtain candidates and `Enter` executes the selected action through the normal
  command/Sequence path instead of hidden I/O. When candidates are already
  loaded, the same product actions remain selectable as explicit refresh/load
  actions for that candidate source, so stale SMS or PDP candidates can be
  updated without leaving the modal. If the selected action requires risk
  confirmation, confirmation stays inside the same `Run Sequence` modal.
  Candidate action failure is reported as action failure, with full detail in
  Response and concise `Action` state in compact Status; it must not mark the
  selected Sequence body as failed.
- TUI candidate-selection keyboard behavior for candidate-backed values:
  candidate rows render in the `Run Sequence` modal, `Up` / `Down` move through
  candidate rows and visible candidate actions, the visible window follows the
  highlighted candidate when there are more candidates than fit, `Enter`
  selects a candidate or runs a highlighted candidate action, manual typing
  remains available, and repeated `Enter` continues the Sequence input flow
  after the value is resolved.
- SMS candidate rows must label modem-returned storage locations as storage
  indexes and must not infer or normalize 0-based or 1-based numbering. Any
  candidate-window range label must be visibly separate from the modem storage
  index value.
- TUI Sequence confirmation as a phase-based modal state: long Values or Review
  sections must not push the risk instruction or `Input:` line out of view.
  Render-buffer coverage should include a long TCP Sequence confirmation at the
  normal TUI modal size.
- SMS `+CMGL` / `+CMGR` response parsing, UCS2 decoded-body analysis in
  `Decoded SMS:`, default decoded-body masking, and reply recipient derivation
  from `AT+CMGR`.
- TCP `+QISEND:` counter analysis, `require_tcp_ack` control behavior for
  incomplete acknowledged/unacknowledged counters, fixed-length payload sending
  without SMS-style Ctrl-Z, and `+QIRD:` no-data/response-data analysis.
- Ping `+QPING:` response analysis and `require_ping_success` control behavior
  so terminal `OK` without at least one parsed received reply is not treated as
  reachability success. Coverage must include a command-accepted `OK` followed
  by later `+QPING:` result lines so prompt/URC waits are not tested only as
  single-buffer responses.
- Quectel TCP/IP PDP context state-aware execution: check `AT+QIACT?`, reuse
  an already active selected context, and send `AT+QIACT=<contextID>` only when
  the selected context is not active.
- Best-effort failure cleanup for product-managed Sequence resources such as
  an opened Quectel socket, with cleanup commands visible in the transcript
  without replacing the original failed step reason.
- Risk aggregation from declared Sequence risk, step commands, payloads,
  parameters, state-aware execution actions, cleanup actions, and known side
  effects.
- Prompt wait, payload write, Ctrl-Z or ESC terminator handling, final response
  wait, URC wait, and per-step timeout behavior through mock transports.
- Sequence loader validation for semantic success flags and wait markers:
  `require_ping_success` definitions must wait for `+QPING:`, and
  `require_tcp_ack` definitions must read `+QISEND:` counters.
- Prompt or URC waits must surface terminal error responses such as `ERROR`,
  `+CME ERROR`, `+CMS ERROR`, and `NO CARRIER` instead of hiding them behind a
  generic timeout when the modem has already returned the failure.
- Structured step results with `analysis` and success notes for CLI JSON
  output.
- Sequence text transcript negative assertions for the literal `Evidence:`
  prefix.
- Sequence text transcript positive assertions for single blank-line separation
  between origin sections and no decorative divider lines in normal output.
- Raw diagnostic export events for multi-step Sequence execution.
- TUI Sequence list rendering, modal input, running state, transcript display,
  copy/save/output masking behavior, and no additional permanent pane.

Repository-managed example Sequence files are part of the implementation
surface, not documentation-only samples. When Sequence loading is implemented,
the Quectel and SORACOM example TOML files must be checked through the same
loader path used for explicit Sequence add-on definitions, for example:

```sh
cargo run -- sequence list --sequence-dir examples/sequences
cargo run -- tui --sequence-dir examples/sequences
```

## Run With a Real USB Modem

```sh
cargo run -- devices
cargo run -- inspect
cargo run -- send AT
```

`cargo run -- devices` shows plausible AT operation targets by default. Use
`cargo run -- devices --all-usb` only when troubleshooting full USB visibility.

If endpoint auto-detection fails, inspect descriptors and specify the interface
and endpoints explicitly:

```sh
cargo run -- inspect
cargo run -- send AT --interface 2 --bulk-in 0x85 --bulk-out 0x04
```

The endpoint values above are examples. Do not hard-code them as universal.

## Run PTY Bridge With a Real USB Modem

The PTY bridge requires a visible USB modem and a free symlink path:

```sh
cargo run -- devices
cargo run -- bridge --symlink /tmp/atctl --bus <BUS> --address <ADDRESS>
screen /tmp/atctl 115200
```

Copy `BUS` and `ADDRESS` from the current `cargo run -- devices` operation
target output. If the expected target is missing, inspect
`cargo run -- devices --all-usb` to confirm USB visibility. Validation-target
VID/PID values can be used only when they uniquely identify one visible runtime
target.

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
screen /tmp/atctl 115200
```

For `screen`, `115200` is a terminal-tool compatibility value, not a physical
USB modem UART speed. To quit `screen`, press `Ctrl-A`, then `K`, then `y`.

Use `--replace-symlink` only for a stale symlink. Existing regular files and
directories must not be overwritten.

## Format and Lint

The normal Rust verification gate is:

```sh
cargo fmt --check
cargo check --all-targets --all-features --locked
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

These commands are product code gates. They must not contain or depend on
coding-agent process checks.

## Dependency Maintenance

Dependency updates are a source change. Direct dependency baseline changes
update `Cargo.toml` and `Cargo.lock` together. Compatible transitive refreshes
may update only `Cargo.lock`.

Install the maintenance tools separately so local checks match CI:

```sh
# RustSec advisory scanner.
cargo install cargo-audit

# Direct dependency drift report.
cargo install --locked cargo-outdated

# GitHub Actions workflow linter. Docker may be used instead by the script.
brew install actionlint
```

Check dependency and workflow maintenance state with:

```sh
scripts/maintenance/check-deps.sh
```

The check script runs:

- `cargo metadata --locked --format-version 1`
- `cargo audit`
- `cargo outdated --workspace --root-deps-only`
- `cargo tree --duplicates`
- `actionlint`, or the pinned `rhysd/actionlint:1.7.12` Docker image when
  `actionlint` is not installed but Docker is available

Use current Cargo and crates.io data before changing direct dependency
versions:

```sh
cargo search <crate-name> --limit 1
```

When a direct dependency is intentionally moved to a newer compatible release,
record the exact version in `Cargo.toml`; do not rely on a lockfile-only update
to communicate the product dependency baseline. Duplicate transitive
dependencies reported by `cargo tree --duplicates` should be reviewed, but they
are not automatically product defects when they come from upstream dependency
graphs that the product does not directly control.

For compatible lockfile refreshes, run:

```sh
scripts/maintenance/update-rust-deps.sh all
```

For a single package lockfile refresh, run:

```sh
scripts/maintenance/update-rust-deps.sh package <crate-name>
```

For a precise package version allowed by `Cargo.toml`, run:

```sh
scripts/maintenance/update-rust-deps.sh package <crate-name> <version>
```

After refreshing `Cargo.lock`, the update script runs the maintenance checks and
the normal Rust verification gate:

```sh
cargo fmt --check
cargo check --all-targets --all-features --locked
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```
