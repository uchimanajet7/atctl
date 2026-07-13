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
The repository pins Rust 1.97.0 in `rust-toolchain.toml` for local and CI use,
and declares the corresponding Rust 1.97 baseline in `Cargo.toml`. With
`rustup`, running Rust commands from the repository selects that pinned
toolchain and installs its declared Rustfmt and Clippy components when needed.
Verify the selected versions with:

```sh
rustc -Vv
cargo -V
```

## Optional Tools

```sh
# Source control tool, needed if it is not already installed.
brew install git

# Optional task runner, only useful if the repository later adds a justfile.
brew install just

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
- Sensitive value masking
- Preset risk classification
- Config parsing
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
- File preset loading, source display, risk, and examples:
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

When changing file preset loading, listing, execution, or risk handling, verify:

- Both repository-managed TOML files load through the same path used for other
  file presets.
- CLI list output includes the `source-path` column.
- CLI run output shows the external file preset notice before USB access,
  including the `--yes --risk-ack <risk>` path.
- Duplicate names fail with an actionable error.
- Every example preset declares risk, and declared risk cannot downgrade the
  command classifier's effective risk.
- The TOML remains human-editable.

When changing standard or vendor-specific preset coverage, compare the result
with SPEC sections 17.3 through 17.5. Those sections own the standard workflow
command inventory, the vendor-neutral boundary, and the Quectel and SORACOM
example requirements.

## Sequence Development

Before changing Sequence behavior, review `REQ-SEQ-ENGINE-*`, `REQ-CLI-SEQ-*`,
`REQ-TUI-SEQ-*`, `REQ-SEQ-FILE-*`, and `REQ-TEST-007` through `REQ-TEST-009` in
[`SPEC.md`](SPEC.md). Cover these production surfaces together when they are
affected:

- TUI `Commands / Sequences` selection, `Run Sequence` modal, compact Status
  step/result context without Sequence summary text, Controls behavior, and
  Response step transcript.
- CLI `atctl sequence list` and `atctl sequence run <SEQUENCE>`.
- `atctl send` and `atctl preset run` staying one-shot surfaces with clear
  errors if a Sequence name is supplied there.
- PTY bridge prompt-capable manual operation for prompt-required commands as
  defined by the specification.

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
  the six exact risk labels without suffix words, separated output-masking
  state, masked copy/export, unmasked `copy` and `export` confirmation, exact
  destination identity, and no additional permanent pane.

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

After the workflow has completed successfully at least once, the repository's
GitHub rules for `main` must require the **Rust quality gate** status check.
The workflow file produces the check; the GitHub repository rule is what blocks
a merge while the check is pending or failing.

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

Check dependency and workflow maintenance state with:

```sh
scripts/maintenance/check-deps.sh
```

The check script runs:

- `cargo metadata --locked --format-version 1`
- current stable Rust, repository toolchain, cargo-about, and actionlint pin
  comparison against their authoritative upstreams
- `cargo audit`
- `cargo outdated --workspace --root-deps-only`
- `cargo tree --duplicates`
- repository maintenance and release shell syntax checks
- `cargo fetch --locked --target aarch64-apple-darwin` before offline notice
  generation
- pinned, target-specific third-party notice generation twice with an exact
  output comparison
- `actionlint`, or the pinned `rhysd/actionlint` Docker image defined by
  `scripts/maintenance/tool-versions.env` when `actionlint` is not installed
  but Docker is available

Dependabot checks Cargo dependencies, `rust-toolchain.toml`, and remote GitHub
Actions every week. `scripts/maintenance/check-version-drift.sh`, which is also
run by the scheduled maintenance workflow, covers the exact Rust release and
the pinned cargo-about and actionlint versions. It fails with the exact update
command when one of those upstreams publishes a newer stable release. The
scheduled workflow also treats a newer direct Cargo dependency as a failing
drift signal, while a normal local run prints the direct-dependency report.

Update a fixed tool version with one of these commands:

```sh
scripts/maintenance/update-tool-versions.sh rust <version>
scripts/maintenance/update-tool-versions.sh cargo-about <version>
scripts/maintenance/update-tool-versions.sh actionlint <version>
```

The Rust update command changes both `rust-toolchain.toml` and the matching
`Cargo.toml` baseline. The other commands update the single version source used
by local scripts and CI. Run `scripts/maintenance/check-deps.sh` afterward; a
Rust baseline change may also permit newer compatible transitive packages, so
review `cargo update --dry-run` and use the dependency update script when the
lockfile should be refreshed.

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
