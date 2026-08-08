# Development

This is the maintainer guide for local setup, repository navigation, product
changes, and verification commands. [SPEC.md](SPEC.md) owns normative product
and verification requirements. [PACKAGING.md](PACKAGING.md) owns the release
and packaging runbook.

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

## Repository Map

Use this map for maintainer navigation; it describes the current tree and is
not a product requirement.

```text
src/
  main.rs, lib.rs    executable and library entry points
  app/              shared operation service and application errors
  at/               command, parser, response, review, and masking logic
  cli.rs, cli/       CLI definitions and CLI tests
  log/               history, session-log, and raw-export writers
  presets/           product presets plus file-definition model and loader
  sequences/         product Sequences plus definition, loader, and engine
  transport/         transport traits, USB/PTY adapters, and test support
  tui/               TUI state, rendering, clipboard, theme, and tests
  usb/               discovery, descriptors, endpoint selection, and transport
  paths.rs           generated-state path resolution
  response_export.rs normal Response export
examples/
  presets/           repository-managed file-preset examples
  sequences/         repository-managed Sequence examples
scripts/
  maintenance/       dependency and notice maintenance commands
  release/           release artifact construction and verification
.github/workflows/   CI, dependency review, maintenance, and release workflows
docs/                role-specific public documentation
```

When paths change, update this navigation map. Architecture and behavior remain
owned by [SPEC.md](SPEC.md), not by the directory names above.

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
- TUI reflow, stable one-row surfaces, Unicode boundaries, and visual-row
  scrolling: `REQ-TUI-TEXT-001` through `REQ-TUI-TEXT-003`, together with the
  owning modal, Details, Response, and saved-log requirements.
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

## Documentation Changes

Treat a documentation change as one semantic-owner change plus any necessary
reader-facing projections:

1. Identify the owner in [SPEC.md section 1](SPEC.md#1-purpose) and edit that
   document first.
2. Search every public document and current CLI/TUI source for the affected
   command, option, label, path, requirement identifier, and example.
3. Update only the projections needed for quick start, point-of-use safety,
   user procedure, translation, or release history. Link to the owner for the
   complete contract instead of copying it.
4. Preserve existing `REQ-*` and `OQ-*` identifiers. Do not rewrite an accepted
   decision silently or edit an already released changelog entry as if it were
   current documentation.
5. Check all relative links, heading anchors, English/Japanese landing-page
   parity, current help text, and the complete diff. Run the normal Rust gate
   when a changed document is covered by repository tests or when source and
   documentation changed together.

There is no documentation word-count gate. Remove material when another file
owns it; retain detail when it is necessary to specify, operate, or verify the
product.

### Changelog Entries

`CHANGELOG.md` is the curated, user-facing history for released versions. Follow
[Keep a Changelog 2.0.0](https://keepachangelog.com/en/2.0.0/): summarize
notable effects for users and source builders instead of copying commits, pull
requests, or the implementation history.

Add or update an entry when a change has at least one of these effects:

- adds a capability or action available to a user;
- changes observable commands, defaults, paths, output, or interaction;
- deprecates or removes behavior, requires migration, or breaks prior use;
- fixes a user-visible defect or security issue; or
- changes installation, supported platforms or runtimes, source-build
  requirements, or release artifacts.

Do not add an entry solely for an internal library or layout engine, parser or
model contract, refactoring, tests, formatting, CI or workflow maintenance,
tool pin, documentation synchronization, reference-version maintenance, or a
dependency update with no user-visible effect. Describe the resulting user
effect when an internal or dependency change does affect users; do not describe
the maintenance mechanism.

Use only the non-empty Keep a Changelog headings `Added`, `Changed`,
`Deprecated`, `Removed`, `Fixed`, and `Security`. Use `Changed` for intentional
behavior changes and `Fixed` when prior behavior was incorrect. Mark an
incompatible item with `**Breaking:**` under `Changed` or `Removed`, state what
stops working, and give the shortest sufficient migration. Link to an existing
procedure when the migration needs more detail.

Write one bullet for one user-observable result and its migration, not for one
commit or file. Group changes that produce the same result across applicable
surfaces; split items when users need different actions or migrations. There is
no item-count or word-count target. A one- or two-sentence release summary is
optional when it helps readers understand the release as a whole.

During development, decide whether each completed change is a changelog
candidate. An internal-only change needs no entry. Before release, compare the
release tag diff with the draft section and review all applicable user-facing
areas:

- installation, release artifacts, supported platforms, and source builds;
- `send`, `preset run`, `sequence run`, TUI, and PTY behavior;
- options, defaults, configuration, and paths;
- logging, Response export, raw diagnostic export, and masking;
- execution review, confirmation, and security behavior;
- Presets, Sequences, SMS, PDP, and TCP workflows; and
- removals, migrations, and user-visible fixes.

Use the actual publication date in the released-version heading. Do not
silently revise an already released entry; follow the documentation change
rules above when a correction is necessary. The release workflow checks only
the matching version, date shape, and non-empty content. Relevance, grouping,
wording, and completeness require this maintainer review and are not delegated
to an automated commit or pull-request summary.

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

Before changing Sequence behavior, identify the owning requirements instead of
copying their expected output into this guide.

| Change area | Owning requirements |
|---|---|
| Engine steps, deadlines, prompt/URC waits, transcripts, and result origins | `REQ-SEQ-ENGINE-*` |
| CLI list, show, input, execution, JSON, and export | `REQ-CLI-SEQ-*` |
| TUI discovery, Details, values, candidates, confirmation, Status, and Response | `REQ-TUI-SEQ-*` plus the applicable `REQ-TUI-GRAMMAR-*` and `REQ-TUI-TEXT-*` |
| Definition fields, defaults, hints, candidates, cleanup, and examples | `REQ-SEQ-FILE-*` |
| Cross-surface execution review and masking | `REQ-REVIEW-*`, `REQ-MASK-*`, and the applicable logging/export requirements |
| Required automated and render-buffer coverage | `REQ-TEST-007` through `REQ-TEST-010` |

Apply the relevant groups across the complete production surface. In
particular, verify CLI `sequence list`, `sequence show`, and `sequence run`;
the TUI `Commands / Sequences`, Details, value/candidate phases,
confirmation, Status, and Response; and the one-shot or manual boundaries of
`send`, `preset run`, and PTY bridge when a change can affect them.

Use mock transports and render-buffer tests for the normative cases in
[SPEC.md](SPEC.md). The normal gate in [Format and Lint](#format-and-lint) runs
those tests. Add focused checks for every changed requirement, including
negative assertions where the contract prohibits a field, label, hidden I/O,
or origin mix. Do not treat a single happy-path render or CLI output as
cross-surface completion.

Repository-managed Sequence files are maintained implementation and verification
inputs. Load both sets through the normal explicit loader:

```sh
cargo run -- sequence list --sequence-dir examples/sequences
cargo run -- tui --sequence-dir examples/sequences
```

The authoring schema, standard Sequence inventory, and example interpretation
are documented for users in [PRESETS.md](PRESETS.md). Their normative engine,
review, masking, and output behavior remains in [SPEC.md](SPEC.md).

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

## Release and Packaging

Use [PACKAGING.md](PACKAGING.md) for the complete source-release, local artifact
verification, Cargo source-package, Homebrew tap, and failure-recovery
procedures. The required release outcomes remain in [SPEC.md](SPEC.md) section
22.

Before entering that runbook, finish the intended source change, update the
matching `Cargo.toml` version and dated [CHANGELOG.md](../CHANGELOG.md) section
when releasing, and run the normal gate in [Format and Lint](#format-and-lint).
Do not duplicate the GitHub Web workflow steps or packaging commands here; one
release procedure prevents the development and packaging guides from drifting.

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
