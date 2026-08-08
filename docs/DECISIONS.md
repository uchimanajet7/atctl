# Product and Architecture Decisions

This document records accepted decisions that explain why `atctl`'s current
product, safety, UI, transport, and distribution contracts take their present
form. [SPEC.md](SPEC.md) is the sole normative owner for product and system
requirements. User and maintainer guides own procedures and reference material;
they do not create a parallel product contract. This file is not a
specification, open-question list, implementation plan, approval log, or
progress tracker.

Existing `OQ-*` identifiers are retained as stable decision identifiers. Each
record has status `accepted`. A later decision that changes an accepted choice
must identify the record it supersedes; accepted rationale must not be silently
rewritten or removed. Exact behavior belongs in the specification, so a record
should retain only the context, alternatives, decision, rationale,
consequences, status, and supersession needed to understand the choice.

## OQ-001: GitHub Owner

Status: accepted

Decision: The source repository owner is `uchimanajet7`, and the source
repository URL is `https://github.com/uchimanajet7/atctl`.

Rationale and consequences: One repository identity is used by package
metadata, documentation, release URLs, and Homebrew metadata.

Contract and guidance: [Cargo.toml](../Cargo.toml) and [SPEC.md](SPEC.md) project
metadata.

## OQ-002: License

Status: accepted

Decision: The project license is MIT.

Rationale and consequences: MIT permits use, modification, redistribution,
sublicensing, sale, and closed-source reuse when the notice is preserved; it
does not require downstream source disclosure. A future need for reciprocity or
explicit patent terms requires a new license decision.

Contract and guidance: [LICENSE](../LICENSE), [Cargo.toml](../Cargo.toml), and the
[SPEC.md](SPEC.md) license contract.

## OQ-003: Homebrew Tap

Status: accepted

Decision: The Homebrew tap repository is `uchimanajet7/homebrew-atctl`; the
user-facing tap name is `uchimanajet7/atctl`; the normal install command is
`brew install uchimanajet7/atctl/atctl`.

Rationale and consequences: Homebrew maps `<user>/<tap>` to the
`homebrew-<tap>` repository. Source code and tap metadata remain separate
repository responsibilities.

Contract and guidance: [INSTALL.md](INSTALL.md), [PACKAGING.md](PACKAGING.md), and the
[SPEC.md](SPEC.md) packaging contract.

## OQ-004: AT Execution Review and Confirmation Policy

Status: accepted

Decision: The product does not expose a scalar risk level. It derives an
execution review from the final operation and separates assessment
(`known`/`unknown`), effects, data handling, reversibility, consequences, and
confirmation. Known observations without a material effect may run without
confirmation. Known material operations require `run` interactively or
`--yes` non-interactively. Unknown operations show what is not established and
require `unknown` interactively or both `--yes --allow-unknown`
non-interactively. `--allow-unknown` is rejected for known operations.

Rationale and consequences: safety, sensitivity, state change, persistence,
and consequence are different dimensions and must not be compressed into one
ordered label. Unknown means the product cannot establish complete semantics;
it is not a promise that every harmful unknown command can be identified.

Contract and guidance: [SPEC.md](SPEC.md) execution-review requirements and
[SAFETY.md](SAFETY.md).

## OQ-005: Source Repository Release Artifacts

Status: accepted

Decision: Source releases build and publish the Apple Silicon macOS archive and
checksum through one manually dispatched GitHub Web workflow. The version and
dated [CHANGELOG.md](../CHANGELOG.md) section are put on `main` first; the
operator then selects `main` and enters `release_tag`. The input identifies the
tag name and version, not a source commit. The workflow requires the dispatched
ref to be `main` before the write-capable release job starts and uses the
dispatch-time `GITHUB_SHA` as the fixed source for validation, build, packaging,
tag handling, and GitHub Release publication. A pushed tag is not an automatic
release trigger. Homebrew publication remains separate tap-repository work.

Rationale and consequences: A single Web-operated entry point gives new and
existing tags the same operator workflow. Requiring `main` before a job receives
write permission prevents an accidental non-`main` selection from reaching tag
or Release creation. Keeping the dispatch-time SHA fixed makes every validation
and publication step refer to the same `main` snapshot without adding a
separate SHA-selection or approval record. Validation, build, packaging, or
release-note failures therefore do not leave a newly published tag without a
release. Draft-first asset upload prevents an incomplete public release.
Separating source artifacts from Homebrew publication prevents one release
action from silently changing another repository or distribution channel.

Contract and guidance: the source release workflow, [CHANGELOG.md](../CHANGELOG.md),
[PACKAGING.md](PACKAGING.md), and the [SPEC.md](SPEC.md) packaging contract.

## OQ-006: Source Repository Release Asset Naming

Status: accepted

Decision: Release assets use
`atctl-v{VERSION}-aarch64-apple-darwin.tar.gz` and the matching `.sha256` name.

Rationale and consequences: The target triple makes the platform explicit.
This is a project naming decision consistent with common Rust CLI practice, not
a formal GitHub or Rust filename standard.

Contract and guidance: [SPEC.md](SPEC.md) `REQ-PKG-016` and `REQ-PKG-017`, and
[PACKAGING.md](PACKAGING.md).

## OQ-007: Source Repository Checksum Content and Provenance

Status: accepted

Decision: Each release archive has one sha256sum-compatible checksum file.
Aggregate manifests, provenance, attestations, and SBOMs are not published
without a later decision.

Rationale and consequences: The initial release contract stays explicit and
verifiable without promising additional supply-chain artifacts whose generation
and verification lifecycle has not been defined.

Contract and guidance: the source release workflow, [SPEC.md](SPEC.md)
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

Contract and guidance: [INSTALL.md](INSTALL.md), [PACKAGING.md](PACKAGING.md), and the
[SPEC.md](SPEC.md) Homebrew requirements.

## OQ-009: Signing and Notarization

Status: accepted

Decision: Developer ID signing and Apple notarization are not required for the
Homebrew path or manual GitHub Release archives. They must be reconsidered
before direct downloads become a normal installation path.

Rationale and consequences: Promoting direct downloads changes Gatekeeper,
quarantine, credential, secret-management, and user-experience requirements.

Contract and guidance: [INSTALL.md](INSTALL.md), [PACKAGING.md](PACKAGING.md), and
[SPEC.md](SPEC.md) `REQ-PKG-029` through `REQ-PKG-031`.

## OQ-010: PTY Bridge Implementation Approach

Status: accepted

Decision: The PTY bridge uses `portable-pty` behind a thin bridge boundary. A
platform-specific implementation requires a new decision only if required
slave-path, symlink, terminal, cleanup, or signal behavior cannot be satisfied.

Rationale and consequences: The abstraction avoids coupling unrelated
transport, masking, logging, and safety behavior to one OS implementation
without promising Linux support.

Contract and guidance: [Cargo.toml](../Cargo.toml) and the [SPEC.md](SPEC.md) bridge
requirements.

## OQ-011: Exact Onyx / EG25-G Endpoint Mapping

Status: accepted

Decision: USB interface and endpoint selection is determined at runtime from
descriptors, candidates, an AT probe, or explicit override; fixed Onyx endpoint
constants are not normative.

Rationale and consequences: Endpoint values can vary by configuration,
alternate setting, firmware, host, and environment. Observed values are
evidence, not universal requirements.

Contract and guidance: [SPEC.md](SPEC.md) USB requirements and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-012: Cross-Surface Execution Review Presentation

Status: accepted

Decision: Human executable discovery uses one compact confirmation grammar
across CLI preset/Sequence lists, TUI `Commands / Sequences`, and TUI
candidate actions. A one-row item identifies the command or Sequence and
appends the literal ASCII tag `[confirm]` only when selection leads to an
explicit confirmation step. The tag describes action flow, not severity or a
scalar risk class.

Complete definition and execution-review information belongs in read-only CLI
`show` and TUI `d Details`. The action-time confirmation is derived again
from the final resolved operation and owns consequences, uncertainty, and the
exact acknowledgement. Compact Status remains current state and execution
context rather than a duplicate review surface.

TUI Sequence input is phase-based: it keeps the active value and next action
visible, keeps long supporting review content scrollable, and distinguishes
value progress from paging. Accepting a candidate does not run the selected
Sequence. All modal surfaces use `Esc` for close, cancel, or back; lowercase
`q` quits only from the idle base screen and remains ordinary text in a
visible input field. Text and structure carry meaning in dark, light, and
no-color modes; color is reinforcement only.

Rationale and consequences: A stable one-row grammar supports scanning mixed
commands and Sequences without repeating full review prose. The explicit word
`[confirm]` states the next interaction without implying warning severity and
remains usable without color. Separating discovery, Details, current Status,
and action-time confirmation gives each surface one job. Re-deriving the final
review prevents an unresolved Sequence or edited command from authorizing a
different operation. Phase-critical input and confirmation controls remain
available even when supporting content is long.

Exact lane widths, shortening, semantic section order, modal titles, pinned
regions, colors, contrast values, narrow-state behavior, and verification
matrices are product requirements in [SPEC.md](SPEC.md), not a second decision
record. Any change must still evaluate the affected CLI/TUI surfaces and
adjacent rendered states together.

Current-source basis:

- [Command Line Interface Guidelines](https://clig.dev/) supports concise,
  consistent subcommand output and action-boundary confirmation.
- [WCAG 2.2 Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)
  requires meaning to remain available without color.
- [W3C Forms Tutorial](https://www.w3.org/WAI/tutorials/forms/) and
  [labels or instructions guidance](https://www.w3.org/WAI/WCAG22/Understanding/labels-or-instructions.html)
  support staged input with adjacent instructions.
- [Fluent 2 dialog guidance](https://fluent2.microsoft.design/components/web/react/core/dialog/usage)
  keeps important dialog content prominent while supporting content scrolls.
- [USWDS step indicator guidance](https://designsystem.digital.gov/components/step-indicator/)
  distinguishes current-step progress from page navigation.
- [Ratatui Table](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html)
  and [Block title](https://ratatui.rs/recipes/widgets/block/) documentation
  provide the layout primitives used by the specified terminal grammar.

Record maintenance: On 2026-08-07 this record was condensed to remove duplicated
normative layout details and repeated comparison prose. The accepted decision
did not change; the exact behavior and verification requirements remain in
[SPEC.md](SPEC.md).

Contract and guidance: [SPEC.md](SPEC.md) CLI/TUI discovery, execution review,
confirmation, Status, theme, modal, and verification requirements.

## OQ-013: TUI Output Masking

Status: accepted

Decision: TUI output masking is on by default. Foreground unmasking requires
`unmask` and applies only to current-session Response display, copy, and
explicit Response export. It never changes masking for generated history,
session logs, or raw diagnostic export.

Rationale and consequences: Operators can inspect and explicitly export
sensitive diagnostic output without silently persisting it to generated logs.

Contract and guidance: [SPEC.md](SPEC.md) masking requirements and
[SAFETY.md](SAFETY.md).

## OQ-014: TUI History / Logs Pane Completion Scope

Status: accepted

Decision: The TUI provides read-only in-app review of existing masked history
and session logs, using Response as a temporary scrollable log-view surface. It
does not create, delete, prune, rotate, or expose raw logs while viewing.

Rationale and consequences: A filename list alone does not complete the
log-review task; the review path must remain masked and must not create new data
as a side effect.

Contract and guidance: [SPEC.md](SPEC.md) log requirements and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-015: TUI Preset / Ad-Hoc Input

Status: accepted

Decision: Product presets cover vendor-neutral modem workflows;
vendor/provider commands use explicitly loaded repository examples or user
files; the product derives the execution review and definition metadata cannot
override it; one-off AT input remains available; prompt-required multi-step
operations use Sequences.

Rationale and consequences: This keeps ordinary modem workflows usable while
preserving origin, review responsibility, and safety for external executable
definitions.

Contract and guidance: [SPEC.md](SPEC.md) preset and TUI requirements,
[PRESETS.md](PRESETS.md), and [SAFETY.md](SAFETY.md).

## OQ-016: TUI Device Pane and Long-Running Command Timeout Control

Status: accepted

Decision: The TUI Devices pane shows live candidates. User AT operations default
to 30 seconds, known long-running definitions may provide item hints, and
`available-operators` uses 240 seconds. CLI precedence is explicit value, item
hint, then product default where an item hint applies. TUI precedence is session
value, item hint, then product default. The TUI shows the effective numeric value
without source/default/override qualifiers; blank input clears the session value.
USB writes and endpoint probes retain separate fixed three-second bounds.

Rationale and consequences: Device and effective timeout state must be
actionable and visible without making the normal interface explain internal
precedence. Sequence execution uses one total deadline, and PTY prompt/payload
execution shares one command budget so the selected duration is not doubled.

Contract and guidance: [SPEC.md](SPEC.md) TUI device, Status, Controls, and timeout
requirements.

## OQ-017: TUI Explicit Device Selection Gate

Status: accepted

Decision: One visible matching device may be selected automatically; multiple
devices require explicit selection; reselection affects later commands only; no
built-in known-device or closed support list controls selection.

Rationale and consequences: Silent selection is acceptable only when
unambiguous. Runtime USB identity and explicit selection avoid misdirected
commands and false compatibility claims.

Contract and guidance: [SPEC.md](SPEC.md) TUI device-selection requirements.

## OQ-018: Running-Command Interruption

Status: accepted

Decision: The TUI does not present a normal running-command Cancel action or
claim that stopping a host read cancels modem-side execution. It uses visible
progress, the current timeout, and conflicting-send blocking instead.

Rationale and consequences: Host-side waiting can stop while the modem
continues and later emits output; calling that successful cancellation would be
misleading.

Contract and guidance: [SPEC.md](SPEC.md) `REQ-TUI-008D` through `REQ-TUI-008F`, and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-019: PTY Bridge Runtime Behavior

Status: accepted

Decision: The PTY bridge resolves its target before symlink creation, protects
existing files and symlinks, cleans up only its own symlink, handles
line-oriented input and typed execution confirmation, and supports one bridge loop
rather than concurrent clients.

Rationale and consequences: The bridge must preserve normal target-selection
and safety behavior while remaining predictable for `screen`/`cu`-style
clients.

Contract and guidance: [SPEC.md](SPEC.md) bridge requirements and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-020: Device Listing Default Scope

Status: accepted

Decision: `atctl devices` shows descriptor-based plausible AT operation
targets; `--all-usb` provides full troubleshooting visibility; no product-name
or VID/PID allow-list defines support; the TUI uses the same distinction.

Rationale and consequences: Descriptor filtering reduces irrelevant USB noise
but is not an AT probe or compatibility guarantee. Full visibility remains
available when the conservative filter omits a device.

Contract and guidance: [SPEC.md](SPEC.md) device and TUI discovery requirements, and
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

Contract and guidance: [SPEC.md](SPEC.md) raw-export requirements and
[SAFETY.md](SAFETY.md).

## OQ-022: TUI Shortcut Reduction and Controls Pane

Status: accepted

Decision: The primary TUI flow is Categories to Commands/Sequences to `Enter`;
global letter shortcuts remain limited; secondary actions belong to their
owning panes; help is modal; pane topology and focus order remain stable.

Rationale and consequences: Workflow-first navigation, local action feedback,
and stable hierarchy reduce shortcut burden and preserve accessibility without
turning Controls or Help into dense inventories.

Contract and guidance: [SPEC.md](SPEC.md) TUI Controls, help, layout, and navigation
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

Contract and guidance: [SPEC.md](SPEC.md) Sequence requirements,
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

Contract and guidance: [SPEC.md](SPEC.md) state-path, CLI, TUI, and logging
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

Contract and guidance: [SPEC.md](SPEC.md) Response-export requirements,
[SAFETY.md](SAFETY.md), and [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## OQ-026: Source Change CI Quality Gate

Status: accepted

Decision: The source repository provides one **Rust quality gate** job for every
pull request targeting `main`, every push to `main`, and manual dispatch. The
job runs the documented normal Rust verification gate on the GitHub-hosted
`macos-26` Apple Silicon runner, verifies `arm64`, uses read-only repository
permission and immutable Action revisions, and does not use pull-request path
filters.

Rationale and consequences: Contributors and maintainers receive the same
automatic result for source, test, workflow, and documentation changes without
depending on a remembered local check. A target-specific runner exercises the
supported Apple Silicon macOS build environment, and the absence of path filters
reports a result for every pull request. Maintainers review failed runs and
correct the change before release. The release workflow retains an independent
pre-publication run of the gate and stops publication when it fails.

Contract and guidance: [SPEC.md](SPEC.md) verification requirements,
[DEVELOPMENT.md](DEVELOPMENT.md) maintainer procedure, and
[CONTRIBUTING.md](../CONTRIBUTING.md) contributor verification guidance.

## OQ-027: Release Archive and Third-Party Notices

Status: accepted

Decision: The Apple Silicon GitHub Release archive contains one versioned
top-level directory with exactly the `atctl` executable, the project MIT
`LICENSE`, and `THIRD-PARTY-NOTICES.txt`. Rust dependency notices are generated
from the locked, non-development `aarch64-apple-darwin` dependency graph by the
pinned `cargo-about` tool. The notice also identifies the separately linked
native `libusb` dependency and includes the LGPL 2.1 license text. Release
packaging verifies that the arm64 Mach-O binary dynamically links to Homebrew
`libusb` and fails before publication for an unexpected or static linkage
result. The cargo-about pin is owned once by the repository maintenance-tool
version file, checked against crates.io by local and scheduled maintenance, and
updated through the repository version-update script.

Rationale and consequences: A manually downloaded archive is self-contained
for executable identity and license review without mixing the MIT project
license with third-party terms. Target-specific, reproducible notice generation
keeps dependency license disclosure aligned with the shipped binary. Dynamic
link verification preserves the documented Homebrew runtime dependency and
prevents an unnoticed vendored `libusb` fallback from changing distribution
obligations.

Contract and guidance: [SPEC.md](SPEC.md) release packaging requirements and
[PACKAGING.md](PACKAGING.md) maintainer procedure.

## OQ-028: Semantic and Provenance-Based Masking

Status: accepted

Decision: Default masking classifies product-known sensitive fields and
definition-declared sensitive template positions before applying a conservative
long-identifier fallback. Known phone, MSISDN, SMS sender, and SMS recipient
values are masked regardless of length, while unclassified short operational
values remain visible. Bounded execution creates raw and masked forms before
output routing; normal CLI, preset, Sequence, TUI, PTY, JSON, history, session
log, copy, and Response-export consumers use the applicable form. Explicit
unmasked foreground output and acknowledged raw diagnostic export keep their
separate purposes.

Rationale and consequences: A digit-count threshold cannot protect short phone
values, but masking every short number would remove signal, index, context,
message-reference, port, timeout, count, and result information needed for this
product's operation and troubleshooting. Field semantics and definition
provenance resolve the known cases without a new UI, user-authored regex system,
phone-number dependency, remote service, or per-surface behavior fork. Unknown
vendor fields retain an explicit limit: short values are not guessed from text
alone, while the existing long-identifier fallback remains available.

Contract and guidance: [SPEC.md](SPEC.md) masking requirements and
[SAFETY.md](SAFETY.md).

## OQ-029: TUI Text Reflow and Stable-Row Layout

Status: accepted

Decision: Ratatui `Paragraph` is the single canonical TUI engine for reflowed
text, rendered-line measurement, and visual-row scrolling. atctl does not keep
a separate character-based wrapper. Complete prose and diagnostic content uses
word-boundary wrapping with explicit newlines, styles, indentation, and Unicode
extended grapheme clusters preserved. Commands, Sequences, candidates,
candidate actions, saved-log entries, device/category/control rows, compact
Status, the footer, and titles retain their one-row structure; atctl uses
display-width and grapheme-safe ellipsis only where the owning surface permits
shortening. Opened logs scroll by rendered visual row while their title and
line-number gutter continue to identify original source-log lines.

Rationale and consequences: The TUI previously combined Ratatui wrapping with
a second `chars()`-based pre-layout function. The two implementations disagreed
about word boundaries, zero-width characters, and grapheme clusters, and the
same disagreement affected modal geometry, Help, Details, Sequence phases, and
scroll calculations. Reusing the renderer for both measurement and display
removes that duplicated maintenance burden. The experimental Ratatui
rendered-line API remains isolated behind one internal adapter. A user-facing
wrap setting and a separate complete Unicode line-breaking engine would add
configuration and maintenance without improving the fixed reading and
navigation roles defined for atctl.

Contract and guidance: [SPEC.md](SPEC.md) `REQ-TUI-TEXT-001` through
`REQ-TUI-TEXT-003`, modal/Details/Response requirements, and saved-log viewing
requirements; [DEVELOPMENT.md](DEVELOPMENT.md) for maintainer verification.

## OQ-030: Documentation Semantic Ownership

Status: accepted

Supersedes: none

Context: The public documentation had grown around individual changes. Product
requirements, research background, decision rationale, user procedures,
maintainer commands, and release steps were repeated across documents. That
made a correct local edit insufficient to prevent stale command names or
options elsewhere. The project is maintained and distributed at no charge by
an individual, so every additional document, synchronization rule, and review
surface has a continuing maintenance cost.

Alternatives considered:

- Keep every document self-contained. Rejected because volatile facts acquire
  multiple owners and must be rediscovered on every change.
- Split the material into a new documentation site or management system.
  Rejected because it adds publication and synchronization work without
  reducing the product's existing source-of-truth problem.
- Impose a fixed word or file-size limit on the specification. Rejected because
  required behavior remains required even when it is detailed; size alone does
  not identify incorrect ownership.

Decision: Use one semantic owner for each kind of information and allow only
controlled projections needed by a reader at the point of use.

- [SPEC.md](SPEC.md) owns required product and system behavior, interfaces,
  safety controls, packaging outcomes, acceptance criteria, and traceability.
- [INSTALL.md](INSTALL.md), [SAFETY.md](SAFETY.md), and
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md) own operator procedures and
  guidance for their named tasks. [PRESETS.md](PRESETS.md) additionally owns
  the authoring guide and user-facing inventory/schema projections. Required
  product-provided definitions, definition fields, and loader/execution
  behavior remain owned by [SPEC.md](SPEC.md).
- [DEVELOPMENT.md](DEVELOPMENT.md) owns maintainer development and verification
  procedure. [PACKAGING.md](PACKAGING.md) owns packaging and release procedure.
- This file owns significant decision context, alternatives, rationale,
  consequences, status, and supersession.
- [README.md](../README.md) and [README-ja.md](../README-ja.md) remain concise
  landing pages with overview, quick start, minimum point-of-use safety, and
  navigation. [CHANGELOG.md](../CHANGELOG.md) remains curated released-version
  history. [CONTRIBUTING.md](../CONTRIBUTING.md) remains a contribution entry
  point.
- Short quick-start commands, safety warnings, examples, and translated landing
  content may be repeated when that repetition is necessary at the point of
  action. Such text is a projection of its owner, not an independent contract,
  and must be checked with the owner and current user-facing implementation.
- Existing `REQ-*`, `OQ-*`, and release-history identities remain stable when
  material is reorganized. A documentation move must not silently change
  product behavior or an accepted decision.

Rationale and consequences: The structure separates reference, task,
explanation, and normative contract without multiplying public artifacts. It
keeps detailed requirements where implementers and verifiers need them, while
removing repository maps, release runbooks, authoring examples, and UI
explanations from documents that do not own them. Review must therefore check a
changed semantic owner and every intentional projection together. There is no
target word count: the success criteria are unambiguous ownership, current
links and terminology, preserved traceability, and no contradictory duplicate.

Current-source basis, checked 2026-08-07:

- [ISO/IEC/IEEE 29148](https://www.iso.org/standard/72089.html) and
  [NASA requirements guidance](https://www.nasa.gov/reference/appendix-c-how-to-write-a-good-requirement/)
  support clear, concise, traceable requirements with rationale and assumptions
  distinguishable from the requirement itself.
- [Diátaxis](https://diataxis.fr/start-here/) separates tutorials, how-to
  guides, reference, and explanation by reader need.
- [GitHub's content model](https://docs.github.com/en/contributing/style-guide-and-content-model/about-the-content-model)
  and its guidance on
  [combining content types](https://docs.github.com/en/contributing/style-guide-and-content-model/about-combining-multiple-content-types)
  support distinct content purposes while allowing deliberate combinations
  when one complex task needs them.
- [GitHub's README guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
  treats the README as the project overview, getting-started, help, and
  contribution entry point rather than the complete long-form documentation.
- [Open Source Guides for maintainers](https://opensource.guide/best-practices/)
  emphasizes documenting processes and keeping that documentation current,
  including for part-time or volunteer maintenance.
- [Microsoft's architecture decision record guidance](https://learn.microsoft.com/en-us/azure/well-architected/architect-role/architecture-decision-record)
  treats decision records as concise, append-only context and consequences,
  not a replacement design guide.
- [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) treats a changelog as
  curated notable release history rather than a dump of commits or current
  product documentation.
- Current widely used Rust CLI repositories such as
  [ripgrep](https://github.com/BurntSushi/ripgrep),
  [just](https://github.com/casey/just), and
  [fd](https://github.com/sharkdp/fd) keep README landing content, release
  history, contribution guidance, and longer user/reference material in
  distinct artifacts. These are comparable maintenance patterns, not a source
  of atctl product requirements.

Contract and guidance: [SPEC.md](SPEC.md) sections 1 and 2 define the current
normative boundary; the role-specific documents named above own their
procedures and controlled projections.
