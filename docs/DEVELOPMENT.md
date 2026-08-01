# Development

This document defines the local development setup for `atctl`.

## Local Development Environment

Local development is documented for Apple Silicon Macs. Other platforms may
build later, but this document does not present them as validated local
development environments.

## Required Tools and Libraries

Install dependencies separately so their purpose is clear.

```sh
# Rust toolchain manager, needed to use the repository-pinned compiler,
# Cargo, Rustfmt, and Clippy versions.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Native USB access library used by atctl through the Rust rusb crate.
brew install libusb

# Build-time helper. Homebrew's pkgconf formula provides the pkg-config command
# used by libusb1-sys to locate libusb compiler and linker metadata.
brew install pkgconf
```

The Rust installer provides `rustup`, `rustc`, and Cargo. `libusb` must still be
available through Homebrew for the initial Apple Silicon Mac setup.
`Cargo.toml` declares the minimum supported Rust version, while
`rust-toolchain.toml` independently selects the complete compiler release used
for local development, CI, and release builds. Running Rust commands from the
repository selects that toolchain and installs its declared Rustfmt and Clippy
components when needed. Verify the active toolchain and selected versions with:

```sh
rustup show active-toolchain
rustc -Vv
cargo -V
```

## Optional Tools

```sh
# Source control tool, needed if it is not already installed.
brew install git

# Optional TOML formatter/checker for preset and Sequence definitions.
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

Run the hardware-independent unit and documentation tests without a physical
modem:

```sh
cargo test --all-features --locked
```

Tests should cover at least:

- AT response parser
- Semantic/provenance-sensitive value masking, including short phone fields
  next to visible short operational values
- Product-derived execution review and authorization
- CLI parsing
- Mock transport behavior

The normative test requirements are `REQ-TEST-001` through `REQ-TEST-009` in
[`SPEC.md`](SPEC.md). Run the change-specific checks described below in addition
to the normal Rust verification gate when the affected area requires them.

## Before Product-Facing Changes

[`SPEC.md`](SPEC.md) is the normative source for product behavior and
verification requirements. Before changing user-facing behavior or wording:

1. Identify the user-facing use case and the affected production surfaces:
   `atctl tui`, `atctl send`, `atctl preset run`, `atctl sequence run`, and the
   PTY bridge where applicable.
2. Inspect the current implementation before editing. For TUI changes, check the
   affected pane titles, rows, help, footer, and representative rendered states.
   For CLI changes, check the current command names, options, help, and output.
   Check the corresponding user documentation as well.
3. Record the exact SPEC requirements that own the behavior. Do not restate a
   different product contract in this guide.
4. Select verification that covers every affected surface and adjacent state.
   If the specification does not define a consistent result, resolve that
   specification gap before implementing the change.

Use these requirement groups when selecting the implementation and verification
scope:

- Shared execution and loaded-definition behavior: `REQ-ARCH-003` and
  `REQ-ARCH-006` through `REQ-ARCH-008`.
- File preset loading, source display, execution review, and examples:
  `REQ-CLI-PRESET-001` through `REQ-CLI-PRESET-011`,
  `REQ-CLI-PRESET-RUN-001` through `REQ-CLI-PRESET-RUN-007`, and
  `REQ-PRESET-SET-001` through `REQ-PRESET-SET-005`.
- TUI structure, grouping, and adjacent-state coverage:
  `REQ-TUI-GRAMMAR-001` through `REQ-TUI-GRAMMAR-005` and
  `REQ-TEST-009`.
- Compact Status and failed-before-response behavior: `REQ-TUI-013A`,
  `REQ-TUI-014`, and the related `REQ-TUI-014*` requirements.
- Sequence transcript and JSON output origins: `REQ-SEQ-ENGINE-004A`,
  `REQ-SEQ-ENGINE-004B`, and `REQ-SEQ-ENGINE-007A`.
- Sequence input, candidate assistance, and modal behavior: the
  `REQ-TUI-SEQ-*` and `REQ-SEQ-FILE-*` requirement groups.

## Development Change Boundary

Keep the implementation, specification, and verification responsibilities
separate:

- `SPEC.md` owns normative product and technical requirements.
- Product tests verify the corresponding runtime contracts and user-visible
  output.
- This guide provides the commands and change-specific checks used by
  maintainers.

When a change affects several production surfaces, implement and verify the
shared behavior at the common layer required by the specification. Do not mark
the change complete after checking only one applicable surface.

## File Preset Development

Repository-managed file preset examples are part of the maintained
implementation and verification surface. Load the Quectel and SORACOM examples
through the normal file preset loader:

```sh
cargo run -- preset list --preset-dir examples/presets
```

When changing file preset loading, listing, execution, or review handling, verify:

- Both repository-managed TOML files load through the same path used for other
  file presets.
- Human CLI list output stays on the compact `NAME` / `AT COMMAND` grammar,
  appends `[confirm]` only when required, aligns it in the conditional fixed
  trailing lane, reclaims that lane when no row requires confirmation, groups
  each file source once, and preserves complete source metadata in `list
  --json` and `preset show`.
- CLI run output shows the external file preset notice before USB access,
  including the `--yes` and `--yes --allow-unknown` paths.
- Duplicate names fail with an actionable error.
- Example presets contain no operator-declared risk field; the product derives
  the review from the final command, and a removed `risk` field is rejected.
- The TOML remains human-editable.

When changing standard or vendor-specific preset coverage, compare the result
with SPEC sections 17.3 through 17.5. Those sections own the standard workflow
command inventory, the vendor-neutral boundary, and the Quectel and SORACOM
example requirements.

## Sequence Development

Before changing Sequence behavior, review `REQ-SEQ-ENGINE-*`, `REQ-CLI-SEQ-*`,
`REQ-TUI-SEQ-*`, `REQ-SEQ-FILE-*`, and `REQ-TEST-007` through `REQ-TEST-010` in
[`SPEC.md`](SPEC.md). Cover these production surfaces together when they are
affected:

- TUI `Commands / Sequences` one-row command/Sequence grammar, sectioned
  read-only `d Details`, focused `Run Sequence` value input, scrollable final
  review with pinned run instruction/input, bottom footer, phase identity, and
  parameter/candidate navigation, compact Status step/result context without
  Sequence purpose text, Controls behavior, and Response step transcript.
- CLI `atctl sequence list` and `atctl sequence run <SEQUENCE>`.
- `atctl send` and `atctl preset run` staying one-shot surfaces with clear
  errors if a Sequence name is supplied there.
- PTY bridge prompt-capable manual operation for prompt-required commands as
  defined by the specification.

Tests should cover at least:

- TUI command and Sequence rows using the same physical height, without
  repeated `AT:` / `Purpose:` labels, in product-only, mixed-kind, and loaded
  definition states. Confirmation-required rows must append the complete
  `[confirm]` action-flow tag in the fixed trailing lane, confirmation-free
  rows must leave that lane blank, a confirmation-free result set must reclaim
  it, and selection styling must remain independent from the dedicated
  ConfirmationFlow tag style.
- Human `preset list` and `sequence list` output must use the same exception-only
  `[confirm]` tag and conditional trailing-lane alignment as TUI rows, without
  an `ATTENTION` column, TUI theme color, or complete warning prose. TUI
  candidate-acquisition actions must derive and display the same tag using the
  same action-set alignment rule.
- `d Details` for commands and Sequences without a visible device, complete
  wrapped identity, non-default source/file disclosure, hidden-field search
  matches, line/page scrolling, `q` no-op, `Esc` close, and exact discovery
  state restoration.
- Empty category and search-result recovery text plus the safe resize fallback
  when the complete confirmation tag, Details, or pinned confirmation content
  cannot fit.
- Complete Sequence Name/Purpose in parameter, candidate, candidate-action
  confirmation, no-confirmation review, known confirmation, unknown
  confirmation, and error states. For multi-parameter input, verify the
  right-aligned `Value <current> of <total>` modal title in value input and
  candidate-action confirmation, its omission for single-value and final-review
  phases, the active label above the value without a body counter, and the exact
  `Enter` outcome in the modal footer. Long context must page independently
  without changing the active value position while the active value/candidate or
  confirmation instruction/Input remains visible.
- Dark and light render-buffer checks for the dedicated ConfirmationFlow colors,
  including contrast of at least 4.5:1 against their specified backgrounds, and
  a no-color check that retains the complete bold ASCII tag without relying on
  hue.

- Sequence TOML parsing for explicit `--sequence-file` and
  `--sequence-dir` locations.
- Sequence TOML parsing for pre-send `review` items, `success_notes`, and
  per-step `evidence` text rendered as `Analysis:`.
- Explicit `--sequence-file` and `--sequence-dir` loading.
- CLI `sequence list` compact output with the shared `[confirm]` grammar,
  fixed trailing-lane alignment, lane reclamation, `list --json`, complete read-only
  `sequence show`, and CLI `sequence run` external definition notice output
  before USB access, including the `--yes` and `--yes --allow-unknown` paths.
- Duplicate Sequence name rejection.
- Required parameter validation and sensitive parameter masking.
- Raw/masked execution-pair coverage across direct `send`, preset execution,
  Sequence command/payload templates, TUI Response/Status/copy/export, PTY
  output, JSON, history, and session logs. Tests must prove that a known short
  phone/MSISDN field is hidden on each applicable path, an adjacent short
  operational value remains visible, unclassified short vendor output is not
  guessed, and explicit unmasked foreground/raw diagnostic paths remain raw.
- Normal log-writer and saved-log-view tests must consume the already-masked
  operation/review/response artifact and reject context-losing remasking in a
  later consumer.
- Sequence list review from the unresolved definition boundary, followed by a
  newly derived CLI/TUI confirmation review after operator values and defaults
  are bound; execution output and logs must retain that resolved review.
- Runtime-derived values may remain unresolved before the producing modem step,
  but the resulting command must stay within the authorized effects and data
  handling immediately before its transport write.
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
  `source`, `candidate`, candidate-provider references, `hint`, and
  `before_running` metadata. Candidate providers are existing executable item
  references, not copied command definitions: `sms-message` resolves through
  product Sequence `sms-receive-check`; `pdp-context` resolves through product
  Presets `active-pdp-contexts` and `pdp-contexts`.
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
  active candidate-backed value, the modal shows the referenced existing
  Presets or Sequences and `Enter` executes the selected item through its normal
  path instead of hidden I/O. When candidates are already loaded, the same
  references remain selectable as explicit refresh/load actions for that
  candidate source. If the selected provider requires confirmation,
  confirmation stays inside the same `Run Sequence` modal. Provider failure is
  reported as action failure, with full detail in Response and concise
  `Action` state in compact Status; it must not mark the selected Sequence body
  as failed.
- Interactive CLI Sequence input must offer the same manual-entry and referenced
  provider choices. Non-interactive and JSON Sequence execution must stop with
  provider and `--param` guidance and must not perform hidden acquisition. Tests
  for direct `send`, `preset run`, and PTY must preserve their direct topology:
  those entry points do not resolve Sequence parameters or run candidate
  providers.
- TUI candidate-selection keyboard behavior for candidate-backed values:
  candidate rows render in the `Run Sequence` modal, `Up` / `Down` move through
  candidate rows and visible candidate actions, the visible window follows the
  highlighted candidate when there are more candidates than fit, `Enter`
  accepts a candidate and continues to the next value or final review in the
  same keystroke, or runs a highlighted candidate action. Candidate action
  completion returns to the modal without selecting a value. Manual typing
  remains available.
- SMS candidate rows must label modem-returned storage locations as storage
  indexes and must not infer or normalize 0-based or 1-based numbering. Any
  candidate-window range label must be visibly separate from the modem storage
  index value.
- TUI Sequence confirmation as a phase-based modal state: long Values or Review
  sections must not push the confirmation instruction or `Input:` line out of view.
  Render-buffer coverage should include a long TCP Sequence confirmation at the
  normal TUI modal size.
- SMS `+CMGL` / `+CMGR` response parsing, UCS2 decoded-body analysis in
  `Decoded SMS:`, default decoded-body masking, and reply recipient derivation
  from `AT+CMGR`.
- TCP `+QISEND:` counter analysis only for a zero-length counter-query step
  declaring `require_tcp_ack`, negative coverage that a generic three-field
  `+QISEND:` response is not labelled as TCP, control behavior for incomplete
  acknowledged/unacknowledged counters, fixed-length payload sending without
  SMS-style Ctrl-Z, and command-aware `+QIRD:` receive-counter,
  no-data/response-data analysis. Coverage must distinguish a three-numeric-field
  `AT+QIRD=<id>,0` response from a normal read and from a UDP SERVICE
  length/remote-IP/remote-port header.
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
- Execution-review aggregation from step commands, payloads, sensitive
  parameters, state-aware execution actions, cleanup actions, and known or
  unestablished effects.
- Prompt wait, payload write, Ctrl-Z or ESC terminator handling, final response
  wait, URC wait, and per-step timeout behavior through mock transports.
- Sequence loader validation for semantic success flags and wait markers:
  `require_ping_success` definitions must wait for `+QPING:`, and
  `require_tcp_ack` definitions must send the
  `AT+QISEND=<connectID>,0` counter-query form and read `+QISEND:` counters.
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
- TUI Sequence list rendering with identity-first rows and `[confirm]`
  action-flow tags only for confirmation-required items, phase-specific modal
  input/review styling, structured known/unknown confirmation content, running
  state, transcript display,
  separated output-masking state, masked copy/export, unmasked `copy` and
  `export` confirmation, whole-surface modal controls, exact destination
  identity, and no additional permanent pane. Modal verification must prove
  that `Esc` is the common cancel/close/back key; `q` quits only from the idle
  base screen, is ordinary text only in a visible editable field, and cannot
  close Help or action menus, mutate review-only state, or quit during execution.
  Event handling, modal-body instructions, and footer hints must be checked as
  one contract. Render-buffer coverage must compare adjacent list, input,
  review, confirmation, action-menu, running, and dark/light/no-color states.

Repository-managed example Sequence files are maintained implementation and
verification inputs. Load the Quectel and SORACOM examples through the same
loader path used for other explicit Sequence definitions:

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

To verify normal masked bridge transcript recording, let GNU Screen own the
continuous-session log:

```sh
screen -L -Logfile "$HOME/Documents/atctl-bridge-session.log" \
  /tmp/atctl 115200
```

`Ctrl-A`, then `H` toggles Screen logging during the session. This is distinct
from the atctl raw diagnostic export path and acknowledgement.

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

## Source Change CI

The GitHub Actions **CI** workflow runs the normal Rust verification gate on an
Apple Silicon macOS runner for:

- Every pull request targeting `main`
- Every push to `main`
- A maintainer-requested manual run from the workflow page

The workflow has no path filter, so the **Rust quality gate** check reports for
every pull request, including documentation-only changes. It installs the
project's `libusb` and `pkgconf` build prerequisites, verifies that the runner
architecture is `arm64`, and then runs the commands from [Format and
Lint](#format-and-lint).

Review the reported **Rust quality gate** result and correct failed runs before
release. The release workflow independently runs the same gate and stops
publication when the gate fails.

## Source Repository Release Workflow

The source repository release workflow is operated from GitHub Actions and
publishes GitHub Release artifacts for the source repository. It does not update
the Homebrew tap, publish bottles, or create Homebrew Formula pull requests.

For the GitHub Web UI release path:

1. Open `https://github.com/uchimanajet7/atctl/actions`.
2. Select the **Release** workflow.
3. Select **Run workflow**.
4. Select the branch or commit that should be released.
5. Enter `release_tag`, for example `v0.2.0`.
6. Run the workflow.

The workflow validates that the requested version matches `Cargo.toml`, checks
any existing tag against the selected commit, runs the normal Rust verification
gate, builds the Apple Silicon macOS binary against Homebrew `libusb`, generates
target-specific third-party notices, verifies binary architecture and dynamic
linkage, validates the exact archive tree and modes, creates and verifies the
`.sha256` file, and extracts the matching `CHANGELOG.md` section before starting
publication. The final GitHub CLI operation creates a missing tag at the
selected commit, uploads both assets through a draft release, and then publishes
the GitHub Release. An existing tag is accepted only when it points to the
selected commit and is not moved, overwritten, or deleted.

To create the same release artifacts locally, install the additional pinned
license tool and build against Homebrew `libusb` in a new target directory:

```sh
brew install libusb pkgconf
source scripts/maintenance/tool-versions.env
cargo install --locked cargo-about --version "${CARGO_ABOUT_VERSION}" --features cli

release_target_dir="$(mktemp -d)"
release_output_dir="$(mktemp -d)"
CARGO_TARGET_DIR="${release_target_dir}" \
  cargo build --release --locked --target aarch64-apple-darwin
scripts/release/package-release.sh \
  0.2.0 \
  aarch64-apple-darwin \
  "${release_target_dir}/aarch64-apple-darwin/release/atctl" \
  "${release_output_dir}"
```

The script verifies the arm64 Mach-O identity, dynamic Homebrew `libusb`
linkage, generated license notices, exact archive contents and modes, project
license identity, and checksum before moving completed assets into the output
directory. Existing destination assets are never overwritten.

The GitHub Web workflow is the only automatic source-release entry point.
Pushing a tag does not start it. If any validation, verification, build,
packaging, or release-note step fails, the workflow creates no new tag or
release. If the final GitHub publication operation fails, inspect any resulting
draft and tag before retrying; the workflow does not delete remote release state
automatically.

Do not manually create the GitHub Release page first. The release workflow owns
tag creation when needed and GitHub Release creation so release notes come from
the curated `CHANGELOG.md` section and do not require manual copying.

## Dependency Maintenance

Dependency updates are a source change. Direct dependency baseline changes
update `Cargo.toml` and `Cargo.lock` together. Compatible transitive refreshes
may update only `Cargo.lock`.

Install the maintenance tools separately so local checks match CI:

```sh
# Load the repository-owned cargo-about and actionlint pins.
source scripts/maintenance/tool-versions.env

# RustSec advisory scanner.
cargo install cargo-audit

# Target-specific Rust dependency license report generator.
cargo install --locked cargo-about --version "${CARGO_ABOUT_VERSION}" --features cli

# Direct dependency drift report.
cargo install --locked cargo-outdated

# GitHub Actions workflow linter. Docker may be used instead by the script.
brew install actionlint
```

Dependabot checks Cargo dependencies, `rust-toolchain.toml`, and remote GitHub
Actions every week and creates update pull requests. Those pull requests
present candidates and supporting release information; the maintainer decides
what to adopt, applies the update locally, reviews the complete diff, and
pushes it.

Run the single local dependency-update command after reviewing the candidates:

```sh
scripts/maintenance/update-dependencies.sh
```

The command:

- resolves the latest complete stable Rust release from the official Rust
  channel manifest and updates `rust-toolchain.toml`
- leaves the minimum supported Rust version in `Cargo.toml` unchanged
- updates the repository-owned cargo-about and actionlint pins from their
  authoritative upstreams
- runs `cargo update` for every direct or transitive version currently allowed
  by `Cargo.toml`
- selected Rust, MSRV, lockfile, fixed-tool, and workflow consistency checks
- `cargo audit`
- `cargo outdated --workspace --root-deps-only`
- `cargo tree --duplicates`
- deterministic target-specific third-party notice generation
- `actionlint`
- `cargo fmt --check`
- `cargo check --all-targets --all-features --locked`
- `cargo test --all-features --locked`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `git diff --check`, local status, and a diff summary

`cargo update` updates the lockfile only within the requirements already
declared by `Cargo.toml`. When a reviewed Dependabot update or another explicit
compatibility decision changes a direct dependency baseline, make that
`Cargo.toml` change visible before running the same command. The update command
does not parse or rewrite direct dependency requirements itself.

Dependabot supplies remote GitHub Actions changes. Keep each external Action
pinned to a full commit SHA with its release tag comment on the same line; the
local command validates the selected references but does not independently
rewrite them.

If the command fails, inspect its output and the current working-tree diff
before deciding how to continue. It does not automatically restore, stash, or
discard an existing or partial update. It also never commits, pushes, merges,
tags, publishes, or creates a GitHub Release. After it succeeds, review every
changed machine-owned file and generated notice, commit and push the intended
update, and confirm CI before making the separate release decision.

Duplicate transitive dependencies remain a review signal, not automatically a
product defect when the upstream graph controls them.
