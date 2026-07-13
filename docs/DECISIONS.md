# Product and Architecture Decisions

This document records accepted decisions that explain why `atctl`'s current
product, safety, UI, transport, and distribution contracts take their present
form. Normative requirements remain in [SPEC.md](SPEC.md) and the topic-specific
user and maintainer documents. This file is not an open-question list,
implementation plan, approval log, or progress tracker.

Existing `OQ-*` identifiers are retained as stable decision identifiers. Each
record has status `accepted`. A later decision that changes an accepted choice
must identify the record it supersedes; accepted rationale must not be silently
rewritten or removed.

## OQ-001: GitHub Owner

Status: accepted

Decision: The source repository owner is `uchimanajet7`, and the source
repository URL is `https://github.com/uchimanajet7/atctl`.

Rationale and consequences: One repository identity is used by package
metadata, documentation, release URLs, and Homebrew metadata.

Normative owner: [Cargo.toml](../Cargo.toml) and [SPEC.md](SPEC.md) project
metadata.

## OQ-002: License

Status: accepted

Decision: The project license is MIT.

Rationale and consequences: MIT permits use, modification, redistribution,
sublicensing, sale, and closed-source reuse when the notice is preserved; it
does not require downstream source disclosure. A future need for reciprocity or
explicit patent terms requires a new license decision.

Normative owner: [LICENSE](../LICENSE), [Cargo.toml](../Cargo.toml), and the
[SPEC.md](SPEC.md) license contract.

## OQ-003: Homebrew Tap

Status: accepted

Decision: The Homebrew tap repository is `uchimanajet7/homebrew-atctl`; the
user-facing tap name is `uchimanajet7/atctl`; the normal install command is
`brew install uchimanajet7/atctl/atctl`.

Rationale and consequences: Homebrew maps `<user>/<tap>` to the
`homebrew-<tap>` repository. Source code and tap metadata remain separate
repository responsibilities.

Normative owner: [INSTALL.md](INSTALL.md), [PACKAGING.md](PACKAGING.md), and the
[SPEC.md](SPEC.md) packaging contract.

## OQ-004: Direct `atctl send` Safety Policy

Status: accepted

Decision: Safe reads may run directly; sensitive reads remain masked; write,
persistent, dangerous, and unknown state-changing commands require explicit
confirmation. Non-interactive execution requires both `--yes` and a matching
`--risk-ack <risk>`.

Rationale and consequences: `--yes` suppresses prompting but does not identify
the reviewed risk. A matching acknowledgement prevents stale scripts from
silently accepting a changed risk classification.

Normative owner: [SPEC.md](SPEC.md) direct-send and risk requirements and
[SAFETY.md](SAFETY.md).

## OQ-005: Source Repository Release Artifacts

Status: accepted

Decision: Source releases build and publish the Apple Silicon macOS archive and
checksum through one manually dispatched GitHub Web workflow. The workflow
validates the version and any existing tag, verifies and packages the selected
commit, and prepares the matching [CHANGELOG.md](../CHANGELOG.md) section before
the final operation creates a missing tag, uploads both assets through a draft,
and publishes the GitHub Release. A pushed tag is not an automatic release
trigger. Homebrew publication remains separate tap-repository work.

Rationale and consequences: A single Web-operated entry point gives new and
existing tags the same operator workflow and prevents validation, build,
packaging, or release-note failures from leaving a newly published tag without
a release. Draft-first asset upload prevents an incomplete public release.
Separating source artifacts from Homebrew publication prevents one release
action from silently changing another repository or distribution channel.

Normative owner: the source release workflow, [CHANGELOG.md](../CHANGELOG.md),
[PACKAGING.md](PACKAGING.md), and the [SPEC.md](SPEC.md) packaging contract.

## OQ-006: Source Repository Release Asset Naming

Status: accepted

Decision: Release assets use
`atctl-v{VERSION}-aarch64-apple-darwin.tar.gz` and the matching `.sha256` name.

Rationale and consequences: The target triple makes the platform explicit.
This is a project naming decision consistent with common Rust CLI practice, not
a formal GitHub or Rust filename standard.

Normative owner: [SPEC.md](SPEC.md) `REQ-PKG-016` and `REQ-PKG-017`, and
[PACKAGING.md](PACKAGING.md).

## OQ-007: Source Repository Checksum Content and Provenance

Status: accepted

Decision: Each release archive has one sha256sum-compatible checksum file.
Aggregate manifests, provenance, attestations, and SBOMs are not published
without a later decision.

Rationale and consequences: The initial release contract stays explicit and
verifiable without promising additional supply-chain artifacts whose generation
and verification lifecycle has not been defined.

Normative owner: the source release workflow, [SPEC.md](SPEC.md)
`REQ-PKG-018` through `REQ-PKG-020`, and [PACKAGING.md](PACKAGING.md).

## OQ-008: Homebrew Distribution Strategy

Status: accepted

Decision: Homebrew is the normal installation path. Bottle-backed installation
is preferred, source build remains a fallback, `libusb` is a runtime dependency,
and Rust plus `pkgconf` are source-build dependencies. Bottle and Formula
automation belong to the tap repository.

Rationale and consequences: This preserves one install command while avoiding
an unnecessary Rust build when a matching bottle exists and retaining a
maintainable source verification path.

Normative owner: [INSTALL.md](INSTALL.md), [PACKAGING.md](PACKAGING.md), and the
[SPEC.md](SPEC.md) Homebrew requirements.

## OQ-009: Signing and Notarization

Status: accepted

Decision: Developer ID signing and Apple notarization are not required for the
Homebrew path or manual GitHub Release archives. They must be reconsidered
before direct downloads become a normal installation path.

Rationale and consequences: Promoting direct downloads changes Gatekeeper,
quarantine, credential, secret-management, and user-experience requirements.

Normative owner: [INSTALL.md](INSTALL.md), [PACKAGING.md](PACKAGING.md), and
[SPEC.md](SPEC.md) `REQ-PKG-029` through `REQ-PKG-031`.

## OQ-010: PTY Bridge Implementation Approach

Status: accepted

Decision: The PTY bridge uses `portable-pty` behind a thin bridge boundary. A
platform-specific implementation requires a new decision only if required
slave-path, symlink, terminal, cleanup, or signal behavior cannot be satisfied.

Rationale and consequences: The abstraction avoids coupling unrelated
transport, masking, logging, and safety behavior to one OS implementation
without promising Linux support.

Normative owner: [Cargo.toml](../Cargo.toml) and the [SPEC.md](SPEC.md) bridge
requirements.

## OQ-011: Exact Onyx / EG25-G Endpoint Mapping

Status: accepted

Decision: USB interface and endpoint selection is determined at runtime from
descriptors, candidates, an AT probe, or explicit override; fixed Onyx endpoint
constants are not normative.

Rationale and consequences: Endpoint values can vary by configuration,
alternate setting, firmware, host, and environment. Observed values are
evidence, not universal requirements.

Normative owner: [SPEC.md](SPEC.md) USB requirements and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-012: TUI Risk Visual Differentiation

Status: accepted

Decision: Each TUI command and Sequence row shows exactly one text risk label:
`[safe]`, `[sensitive]`, `[write]`, `[persistent]`, `[dangerous]`, or
`[unknown]`. Risk labels do not contain masking state, confirmation
instructions, persistence descriptions, severity restatements, or review
instructions. Output masking state, expected effect, and required
acknowledgement are presented separately in their owning surfaces. Dark, light,
and no-color themes preserve the text labels.

Rationale and consequences: A risk label must retain one stable meaning across
Commands, Status, and confirmation surfaces. Mixing `MASKED`, `CONFIRM`,
`PERSISTS`, `DANGER`, and `REVIEW` into the same label position combined state,
instructions, effects, and severity. Risk meaning must also survive selection
styling, terminal theme differences, and disabled color. Exact labels and
palette values remain normative in [SPEC.md](SPEC.md).

Normative owner: [SPEC.md](SPEC.md) TUI risk and theme requirements.

## OQ-013: TUI Output Masking

Status: accepted

Decision: TUI output masking is on by default. Foreground unmasking requires
`unmask` and applies only to current-session Response display, copy, and
explicit Response export. It never changes masking for generated history,
session logs, or raw diagnostic export.

Rationale and consequences: Operators can inspect and explicitly export
sensitive diagnostic output without silently persisting it to generated logs.

Normative owner: [SPEC.md](SPEC.md) masking requirements and
[SAFETY.md](SAFETY.md).

## OQ-014: TUI History / Logs Pane Completion Scope

Status: accepted

Decision: The TUI provides read-only in-app review of existing masked history
and session logs, using Response as a temporary scrollable log-view surface. It
does not create, delete, prune, rotate, or expose raw logs while viewing.

Rationale and consequences: A filename list alone does not complete the
log-review task; the review path must remain masked and must not create new data
as a side effect.

Normative owner: [SPEC.md](SPEC.md) log requirements and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-015: TUI Preset / Ad-Hoc Input

Status: accepted

Decision: Product presets cover vendor-neutral modem workflows;
vendor/provider commands use explicitly loaded repository examples or user
files; effective risk cannot be downgraded by declared metadata; one-off AT
input remains available; prompt-required multi-step operations use Sequences.

Rationale and consequences: This keeps ordinary modem workflows usable while
preserving origin, review responsibility, and safety for external executable
definitions.

Normative owner: [SPEC.md](SPEC.md) preset and TUI requirements,
[PRESETS.md](PRESETS.md), and [SAFETY.md](SAFETY.md).

## OQ-016: TUI Device Pane and Long-Running Command Timeout Control

Status: accepted

Decision: The TUI Devices pane shows live candidates. User commands default to
30 seconds, known long-running presets may provide hints such as 180 seconds for
`AT+COPS=?`, and effective timeout precedence is session override, preset hint,
then default.

Rationale and consequences: Device and timeout state must be actionable and
visible instead of remaining static placeholders or hidden execution settings.

Normative owner: [SPEC.md](SPEC.md) TUI device, Status, Controls, and timeout
requirements.

## OQ-017: TUI Explicit Device Selection Gate

Status: accepted

Decision: One visible matching device may be selected automatically; multiple
devices require explicit selection; reselection affects later commands only; no
built-in known-device or closed support list controls selection.

Rationale and consequences: Silent selection is acceptable only when
unambiguous. Runtime USB identity and explicit selection avoid misdirected
commands and false compatibility claims.

Normative owner: [SPEC.md](SPEC.md) TUI device-selection requirements.

## OQ-018: Running-Command Interruption

Status: accepted

Decision: The TUI does not present a normal running-command Cancel action or
claim that stopping a host read cancels modem-side execution. It uses visible
progress, timeout, override, and conflicting-send blocking instead.

Rationale and consequences: Host-side waiting can stop while the modem
continues and later emits output; calling that successful cancellation would be
misleading.

Normative owner: [SPEC.md](SPEC.md) `REQ-TUI-008D` through `REQ-TUI-008F`, and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-019: PTY Bridge Runtime Behavior

Status: accepted

Decision: The PTY bridge resolves its target before symlink creation, protects
existing files and symlinks, cleans up only its own symlink, handles
line-oriented input and typed risk confirmation, and supports one bridge loop
rather than concurrent clients.

Rationale and consequences: The bridge must preserve normal target-selection
and safety behavior while remaining predictable for `screen`/`cu`-style
clients.

Normative owner: [SPEC.md](SPEC.md) bridge requirements and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-020: Device Listing Default Scope

Status: accepted

Decision: `atctl devices` shows descriptor-based plausible AT operation
targets; `--all-usb` provides full troubleshooting visibility; no product-name
or VID/PID allow-list defines support; the TUI uses the same distinction.

Rationale and consequences: Descriptor filtering reduces irrelevant USB noise
but is not an AT probe or compatibility guarantee. Full visibility remains
available when the conservative filter omits a device.

Normative owner: [SPEC.md](SPEC.md) device and TUI discovery requirements, and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-021: Raw Log Diagnostic Export

Status: accepted

Decision: Raw diagnostic export requires an explicit destination and separate
`raw-log` acknowledgement, is available across AT execution surfaces, never
starts automatically, refuses overwrite, and writes lossless JSONL/base64
exchanges.

Rationale and consequences: Raw output can be necessary for final diagnosis
but can persist sensitive identifiers, credentials, messages, payloads, and
network data; it is separate from normal masked logs and foreground unmasking.

Normative owner: [SPEC.md](SPEC.md) raw-export requirements and
[SAFETY.md](SAFETY.md).

## OQ-022: TUI Shortcut Reduction and Controls Pane

Status: accepted

Decision: The primary TUI flow is Categories to Commands/Sequences to `Enter`;
global letter shortcuts remain limited; secondary actions belong to their
owning panes; help is modal; pane topology and focus order remain stable.

Rationale and consequences: Workflow-first navigation, local action feedback,
and stable hierarchy reduce shortcut burden and preserve accessibility without
turning Controls or Help into dense inventories.

Normative owner: [SPEC.md](SPEC.md) TUI Controls, help, layout, and navigation
requirements.

## OQ-023: Sequences for Multi-Step AT Operations

Status: accepted

Decision: `Sequence` is the product term for named multi-step operations.
Standard SMS Sequences are product-provided; vendor/provider data-send
Sequences are explicitly loaded examples; all loaded origins share applicable
safety, masking, transcript, and execution contracts; the TUI uses the existing
executable pane and the PTY bridge supports prompt-capable manual operation.

Rationale and consequences: Presets and direct send remain one-shot. Sequence
output must distinguish submit/socket evidence from end-to-end receipt, and
multi-step support must not fragment production surfaces or add a permanent TUI
pane.

Normative owner: [SPEC.md](SPEC.md) Sequence requirements,
[PRESETS.md](PRESETS.md), [SAFETY.md](SAFETY.md), and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-024: Runtime Options and XDG State Instead of User Configuration

Status: accepted

Decision: `atctl` does not load a per-user product configuration file. USB
target, interface, and endpoint overrides remain explicit per invocation or TUI
selection. Masked history and session logs remain enabled by default, their
location follows `XDG_STATE_HOME`, and `--no-log` disables new normal log
artifacts for one supported execution invocation.

Rationale and consequences: Automatic device discovery and explicit runtime
selection cover the normal and multi-device workflows without hidden persistent
USB defaults. XDG already provides the standard state-location override, and a
single per-invocation logging exception does not justify a separate TOML
configuration contract. Raw diagnostic export remains a separately selected and
acknowledged operation.

Normative owner: [SPEC.md](SPEC.md) state-path, CLI, TUI, and logging
requirements; [SAFETY.md](SAFETY.md) for logging and raw-export boundaries.

## OQ-025: Normal Response Export

Status: accepted

Decision: A normal Response is exported only through an explicit destination
choice. The TUI uses `Export response...` and asks for a destination folder on
every invocation. `send`, `preset run`, and `sequence run` use the common
`--export-response <PATH>` option without replacing stdout. Export follows the
selected foreground masking mode, creates a new file without overwrite, and
remains separate from generated masked logs and raw diagnostic export. PTY
bridge session transcripts are recorded by the selected terminal client because
the bridge is a continuous session rather than one bounded Response.

When the TUI displays a Response that differs from its masked form, copy and
export identify the content as unmasked. Copy requires exact acknowledgement
`copy`. Export requires destination selection, displays the exact final path,
and requires exact acknowledgement `export` before writing. Masked copy and
export retain their direct paths without this additional confirmation.

Rationale and consequences: The operator can identify the artifact, masking
state, format, and destination before persistence. Bounded execution surfaces
share one Response contract, while continuous bridge recording retains its
terminal-session semantics. Existing files in the former XDG Response directory
are retained but no longer define the normal export destination.

Normative owner: [SPEC.md](SPEC.md) Response-export requirements,
[SAFETY.md](SAFETY.md), and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-026: Source Change CI Quality Gate

Status: accepted

Decision: The source repository provides one **Rust quality gate** job for every
pull request targeting `main`, every push to `main`, and manual dispatch. The
job runs the documented normal Rust verification gate on the GitHub-hosted
`macos-26` Apple Silicon runner, verifies `arm64`, uses read-only repository
permission and immutable Action revisions, and does not use pull-request path
filters. After the job has completed successfully, the GitHub repository rules
for `main` require that named check before merge.

Rationale and consequences: Contributors and maintainers receive the same
automatic result for source, test, workflow, and documentation changes without
depending on a remembered local check. A target-specific runner exercises the
supported Apple Silicon macOS build environment, and the absence of path filters
keeps the required-check result from remaining pending on an otherwise eligible
pull request. The release workflow retains an independent pre-publication run of
the same gate. GitHub rule configuration remains a separate repository-owner
operation after the workflow has produced a selectable successful check.

Normative owner: [SPEC.md](SPEC.md) verification requirements,
[DEVELOPMENT.md](DEVELOPMENT.md) maintainer procedure, and
[CONTRIBUTING.md](../CONTRIBUTING.md) contributor verification guidance.
