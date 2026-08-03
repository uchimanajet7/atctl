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

Normative owner: [SPEC.md](SPEC.md) execution-review requirements and
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

## OQ-012: Cross-Surface Execution Review Presentation

Status: accepted

Decision: Human executable discovery uses one confirmation grammar across
`atctl preset list`, `atctl sequence list`, TUI `Commands / Sequences`, and TUI
candidate-acquisition actions. A command row uses `name command`; a Sequence row
uses `name purpose-preview`. A row appends the ASCII action-flow tag `[confirm]`
only when execution requires explicit confirmation. The tag is not a warning,
risk score, or effects classification, and normal rows omit it. The complete
tag is preserved before a name or preview is shortened. When a result set has
at least one tagged row, human CLI lists, TUI `Commands / Sequences`, and TUI
candidate-action sets reserve one separator cell plus the complete tag as a
fixed trailing lane for every row. Untagged rows leave the lane blank; a result
set with no tagged row reclaims the lane. Command or purpose detail is shortened
before the name, and the tag is never split. TUI selection keeps the same row
height. CLI `show` and TUI `d Details` are read-only full-review
surfaces and perform no device I/O. They present the structured execution review
once instead of adding separate confirmation or risk summary rows. Human
`show` and TUI Details use the same semantic section order: Overview,
Definition, Before running, Inputs, and Execution; empty sections are omitted.
The TUI Details title identifies the current section and visible range. A
required confirmation surface leads with consequence-oriented groups and the
exact confirmation word. Known and unknown reviews use different headings so
unknown effects cannot look complete.
Unknown reviews state `Effects are not fully known.` without an ambiguous
standalone icon. Compact Status remains limited to current state and execution
context and does not duplicate selection-time review content.
Sequence value input is focused on the selected Sequence's active value,
directly relevant assistance, candidates, errors, and exact next action; it does
not embed a second complete-definition view. Candidate-action and final
Sequence reviews use an explicit scrollable Review region with a visually
separated, pinned execution instruction and current confirmation input.
Final Sequence review includes all resolved values, applicable before-running
notes, operation summary, and execution consequences.
For a Sequence with multiple parameters, value input and candidate-action
confirmation keep `Value <current> of <total>` in a fixed right-aligned
modal-border title opposite the current phase. The body leads with the active
parameter label and value, without a second counter or generic `Current value:`
heading. The operation footer states whether `Enter` advances to the next
numbered value, reviews the Sequence, accepts a candidate and continues to the
next value or review in the same keystroke, or reviews a candidate action before
confirmation; a confirmation-free candidate action says that it runs.
Accepting a candidate never runs the selected Sequence, and candidate-action
completion never selects a returned value automatically. Review paging never
changes that position. Single-value Sequences and the final
review/confirmation phase omit the counter. Areas too small to show complete
decision-critical content stop at a resize message instead of presenting
partial executable or confirmation text.
At action time, direct `send`, `preset run`, `sequence run`, TUI command and
Sequence execution, TUI candidate actions, edited TUI commands, and PTY bridge
execution derive the review from the final operation. Known material operations
require `run`; unknown operations require `unknown`. Non-interactive CLI
execution requires `--yes` for known operations and
`--yes --allow-unknown` for unknown operations. Structured JSON retains the
execution review and does not add the human presentation tag.
Confirmation uses structure, text markers, and emphasis that remain usable in
dark, light, and no-color modes; color is not the only signal. TUI rows use a
dedicated bold ConfirmationFlow color, `#BE95FF` on the dark background and
`#6929C4` on the light background, while no-color mode keeps bold
`[confirm]`. The selected name and marker retain selection styling without
overwriting the tag role. Human CLI lists use the same text and trailing-lane
grammar without adding TUI theme color. Help and
all other atctl-owned modal surfaces use `Esc` as the single close, cancel, or
back key. Help also accepts `?` because that key opens the overlay. Lowercase
`q` quits only from the idle base screen, remains ordinary text while a visible
input field is active, and has no effect in a non-editable modal or while an
operation is running. The idle base screen does not use `Esc` as a second quit
key and does not add a routine quit confirmation.

Rationale and consequences: a stable one-row list supports scanning many mixed
commands and Sequences without selection changing the surrounding layout.
Removing repeated `AT:` and `Purpose:` labels and the CLI-only `ATTENTION`
column leaves more of the command or purpose visible on constrained human list
surfaces. The ASCII `[confirm]` tag states the next interaction exactly and
remains legible across terminal fonts, dark/light themes, and no-color mode
without implying warning severity. Details and `show` provide complete
information without forcing long prose into every row. Pinning phase-critical
controls prevents long context from hiding the next action. The confirmation
surface owns detailed effects and uncertainty; compact Status does not repeat
them. Re-deriving the action-time review prevents an unresolved Sequence row or
edited command from authorizing a different final operation. One visible root
quit key and one modal cancellation key avoid competing meanings. Keeping `q`
as text only when an input field is visible also prevents hidden state changes
in review-only modals and preserves commands, paths, parameters, and
acknowledgements that contain `q`.

Current-source basis:

- [IBM Carbon notification guidance](https://carbondesignsystem.com/components/notification/usage/)
  reserves warning treatment for potentially undesirable or unexpected
  consequences, so the ordinary confirmation-flow tag does not use warning
  styling.
- [USWDS icon guidance](https://designsystem.digital.gov/components/icon/)
  requires symbols to be conventional and understandable; the explicit ASCII
  word avoids an ambiguous standalone icon.
- [Apple button guidance](https://developer.apple.com/design/human-interface-guidelines/buttons),
  [GNOME writing guidance](https://developer.gnome.org/hig/guidelines/writing-style.html),
  and the [WAI-ARIA menu pattern](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/)
  associate an ellipsis with further input or a dialog, while atctl already
  uses ellipsis for truncation; `[confirm]` avoids overloading it.
- [Unicode Standard Annex #11](https://www.unicode.org/reports/tr11/)
  documents context-dependent terminal widths for some symbols; ASCII gives
  the list a deterministic width budget.
- [Command Line Interface Guidelines](https://clig.dev/) recommends displaying
  only as much information as users need, keeping behavior consistent across
  subcommands, and confirming dangerous operations; this supports a compact
  common discovery token with complete action-time review.
- [Ratatui Table documentation](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html)
  supports explicit fixed-width columns and spacing, and
  [K9s custom views](https://github.com/derailed/k9s#resource-custom-columns)
  expose stable left/right column alignment; these support treating the
  repeated confirmation suffix as one trailing lane instead of appending it
  after variable preview text.
- [IBM Carbon tag guidance](https://carbondesignsystem.com/components/tag/usage/)
  recommends concise text labels and distinct component colors for scanning;
  atctl keeps the literal label as the meaning and uses color only as a
  secondary TUI differentiator.
- [Docker `system prune`](https://docs.docker.com/reference/cli/docker/system/prune/)
  and [Terraform `apply`](https://developer.hashicorp.com/terraform/cli/commands/apply)
  place consequences and approval at the action boundary rather than repeating
  full warning prose in discovery lists.
- [GitHub CLI `gh repo delete`](https://cli.github.com/manual/gh_repo_delete)
  uses an explicit non-interactive acknowledgement option, supporting atctl's
  separate interactive acknowledgement and non-interactive authorization
  contracts.
- [WCAG 2.2 Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)
  requires information conveyed by color to remain available through another
  visual means; `[confirm]` and the full unknown-effects sentence remain
  meaningful in no-color mode. WCAG 2.2 Contrast Minimum requires 4.5:1 for
  normal text; the specified ConfirmationFlow colors provide approximately
  5.60:1 in dark mode and 7.41:1 in light mode.
- [W3C Forms Tutorial](https://www.w3.org/WAI/tutorials/forms/) recommends short,
  logically staged forms that expose the information required for the current
  transaction, and
  [W3C Labels or Instructions](https://www.w3.org/WAI/WCAG22/Understanding/labels-or-instructions.html)
  requires enough adjacent guidance to complete an input without cluttering the
  surface with unnecessary instruction.
- [GOV.UK question pages](https://design-system.service.gov.uk/patterns/question-pages/)
  starts with one current question, while
  [IBM Carbon form guidance](https://carbondesignsystem.com/patterns/forms-pattern/)
  places the input label above the input and essential helper text immediately
  with that field. These support leading the Sequence value phase with the
  current parameter instead of preceding it with the complete context record.
- [Fluent 2 dialog guidance](https://fluent2.microsoft.design/components/web/react/core/dialog/usage)
  keeps dialogs focused, leads body content with the most important information,
  and keeps supporting body content scrollable while persistent regions remain
  available. The
  [Charmbracelet Huh](https://github.com/charmbracelet/huh) terminal form model
  likewise organizes interactive terminal forms around current fields and
  logical groups rather than placing the active field after the complete form
  context.
- [USWDS step indicator guidance](https://designsystem.digital.gov/components/step-indicator/)
  places the current step and total near the task heading and makes the current
  step visually prominent. Its
  [pagination guidance](https://designsystem.digital.gov/components/pagination/)
  explicitly directs sequential-process progress to a step indicator instead of
  page navigation, matching atctl's separation of value position from Details
  paging.
- [IBM Carbon progress indicator guidance](https://carbondesignsystem.com/components/progress-indicator/usage/)
  distinguishes linear multistep progress from
  [progress bars](https://carbondesignsystem.com/components/progress-bar/usage/)
  for loading or processing. atctl therefore uses a compact textual field
  position, not a loading bar or a full visual stepper.
- [Ratatui Block title guidance](https://ratatui.rs/recipes/widgets/block/)
  supports multiple left-, center-, and right-aligned titles on one border, so
  the fixed position can remain separate from both the active field body and
  the scrollable Details label without a new pane or dependency.

Normative owner: [SPEC.md](SPEC.md) CLI/TUI discovery, execution review,
confirmation, Status, and theme requirements.

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
files; the product derives the execution review and definition metadata cannot
override it; one-off AT input remains available; prompt-required multi-step
operations use Sequences.

Rationale and consequences: This keeps ordinary modem workflows usable while
preserving origin, review responsibility, and safety for external executable
definitions.

Normative owner: [SPEC.md](SPEC.md) preset and TUI requirements,
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
progress, the current timeout, and conflicting-send blocking instead.

Rationale and consequences: Host-side waiting can stop while the modem
continues and later emits output; calling that successful cancellation would be
misleading.

Normative owner: [SPEC.md](SPEC.md) `REQ-TUI-008D` through `REQ-TUI-008F`, and
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
filters.

Rationale and consequences: Contributors and maintainers receive the same
automatic result for source, test, workflow, and documentation changes without
depending on a remembered local check. A target-specific runner exercises the
supported Apple Silicon macOS build environment, and the absence of path filters
reports a result for every pull request. Maintainers review failed runs and
correct the change before release. The release workflow retains an independent
pre-publication run of the gate and stops publication when it fails.

Normative owner: [SPEC.md](SPEC.md) verification requirements,
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

Normative owner: [SPEC.md](SPEC.md) release packaging requirements and
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

Normative owner: [SPEC.md](SPEC.md) masking requirements and
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

Normative owner: [SPEC.md](SPEC.md) `REQ-TUI-TEXT-001` through
`REQ-TUI-TEXT-003`, modal/Details/Response requirements, and saved-log viewing
requirements; [DEVELOPMENT.md](DEVELOPMENT.md) for maintainer verification.
