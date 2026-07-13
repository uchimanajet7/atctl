# atctl Product and Technical Specification

- Document version: 0.4.116
- Date: 2026-07-12
- Implementation language: Rust
- Product name: `atctl`
- Repository name: `atctl`
- Command name: `atctl`
- Homebrew formula name: `atctl`
- Homebrew tap repository: `uchimanajet7/homebrew-atctl`
- Homebrew user-facing tap name: `uchimanajet7/atctl`
- Supported runtime: macOS on Apple Silicon
- Documented validation hardware: SORACOM Onyx LTE USB Dongle / Quectel EG25-G

## 1. Purpose

This document defines the normative product behavior and technical requirements
for `atctl`. It is the authoritative source for product scope, interfaces,
safety, packaging contracts, and verification requirements.

The specification covers:

- Product purpose, scope, and non-scope
- Platform and dependency policy
- Architecture and module boundaries
- USB/libusb transport behavior
- AT command parsing and completion behavior
- CLI and TUI behavior
- Presets, configuration, masking, safety, logging, and packaging
- Verification expectations
- Accepted product and architecture decisions

Supporting operational documents live outside this specification:

- `README.md`
- `CHANGELOG.md`
- `docs/INSTALL.md`
- `docs/PRESETS.md`
- `docs/DEVELOPMENT.md`
- `docs/PACKAGING.md`
- `docs/TROUBLESHOOTING.md`
- `docs/SAFETY.md`
- `docs/DECISIONS.md`

## 2. Requirements Basis

This specification states required product behavior, prohibited behavior,
technical constraints, and verification criteria.

The structure follows these current requirements-engineering principles:

- Requirements must be understandable by implementers and verifiers.
- Requirements must be clear, unambiguous, complete for the specified scope,
  internally consistent, and testable.
- Assumptions and unresolved values must be explicit.
- Requirements must not hide product decisions as implementation details.
- Supporting documents should separate user installation, development,
  packaging, troubleshooting, and safety guidance from the normative product
  specification.

Normative source categories:

- Requirements engineering and normative terminology: ISO/IEC/IEEE 29148,
  NASA requirements guidance, RFC 2119, and RFC 3339.
- AT command and modem behavior: 3GPP TS 27.005, 3GPP TS 27.007,
  3GPP TS 23.038, 3GPP TS 23.040, ITU-T V.250, and vendor command manuals.
- USB transport and discovery: USB-IF class codes, USB descriptor references,
  libusb documentation, and CDC ACM implementation guidance.
- Executable definitions and generated state: TOML, XDG Base Directory, and
  the explicit file and directory loading requirements in this specification.
- TUI behavior and accessibility: WCAG 2.2, WAI guidance, terminal and Ratatui
  documentation, and current interaction-design references.
- Packaging, release, and dependency maintenance: Cargo, Homebrew, GitHub
  Actions, Dependabot, RustSec, actionlint, and Keep a Changelog.
- Sensitive-data and evidence handling: OWASP logging guidance and the
  applicable modem, TCP, Quectel, and SORACOM technical references.

Supporting references appear in section 25, in role-specific operational
documents, and beside the requirements they support.

## 3. Normative Terms

The key words `MUST`, `MUST NOT`, `REQUIRED`, `SHOULD`, `SHOULD NOT`, `MAY`,
and `OPTIONAL` are to be interpreted as described by RFC 2119 when written in
uppercase.

Lowercase uses of those words are ordinary English.

## 4. Product Summary

`atctl` is an Apple Silicon Mac-first Rust CLI/TUI for sending, managing, and
presetting AT commands for USB cellular modems.

The product exists because macOS may detect a USB cellular modem as a USB
device without exposing a usable `/dev/cu.*` serial device for AT command
access. `atctl` communicates directly with USB interfaces and bulk endpoints
using `libusb` through Rust `rusb`.

The documented validation hardware is:

```text
Device: SORACOM Onyx LTE USB Dongle
Internal modem: Quectel EG25-G
USB vendor ID: 0x2c7c
USB product ID: 0x0125
```

`atctl` MUST NOT be branded or architected as Onyx-only. SORACOM Onyx /
Quectel EG25-G is the documented validation hardware used during development,
not a closed supported-device list. The product assumes no pre-known device
inventory: normal device discovery MUST work from USB devices visible at
runtime and user-provided selectors such as VID, PID, bus, and address.

The TUI is the main interactive product surface for human-operated modem
troubleshooting and AT command work. CLI commands and PTY bridge are production
surfaces as well; they MUST NOT be treated as validation-only, test-only,
convenience-only, fallback-only, or second-class entry points.

When a product behavior affects AT execution, diagnostics, presets, raw
diagnostic export, SMS, data-send, or multi-step command handling, the
specification and implementation MUST evaluate the relevant production surfaces
together:

```text
TUI
atctl send
atctl preset run
atctl bridge --symlink <PATH>
```

Surface interaction details MAY differ. For example, the TUI can use forms and
dialogs, CLI commands can use flags and standard output, and PTY bridge can use
terminal-compatible prompts. Those interaction differences MUST NOT be used to
silently omit a required product capability, risk control, masking behavior, raw
export behavior, or diagnostic workflow from a relevant surface.

Implementation artifacts, internal data structures, file formats, execution
engines, and project terminology are not user responsibilities by default.
Before a requirement says that the user must author, create, maintain, supply,
or operate something, the specification MUST identify the actor that owns that
responsibility: product-provided behavior, repository-managed example,
user-authored extension, operator action, or implementation detail. Product
documentation MUST NOT turn an internal mechanism into a user prerequisite
unless this specification defines it as one.

Product-provided standard definitions, repository-managed example definitions,
and user-authored extension definitions MUST preserve their origin and review
responsibility. Loading them into one executable item set MUST NOT erase that
responsibility boundary. Once loaded and validated, they MUST use the applicable
shared product contract for listing, selection, risk handling, confirmation,
masking, logging, raw diagnostic export, and execution so equivalent user
actions do not drift into separate product semantics.

Sequences are product-facing multi-step AT actions. Built-in workflow
definitions are product-provided execution definitions, not a prerequisite that
users must author before using standard SMS or other standard multi-step
checks. User-authored workflow definitions are an extension point for
additional, special, project-local, or verification checks. Repository-managed
vendor Sequence examples are product-maintained examples that are available
only when loaded through the same kind of explicit definition loading as file
presets. Product documentation MUST NOT describe ordinary SMS,
data-send, or other standard multi-step checks as requiring the user to create a
definition file first.

## 5. Decided Naming

The following product identifiers are fixed. Changing them requires a revision
to this specification:

```text
Product name: atctl
GitHub repository: atctl
Command name: atctl
Homebrew formula: atctl
```

## 6. Scope

### 6.1 In Scope

The product scope includes:

- Rust-based CLI and TUI
- USB device detection through `libusb` / `rusb`
- SORACOM Onyx / Quectel EG25-G detection
- USB configuration, interface, alternate setting, and endpoint inspection
- Bulk IN / bulk OUT endpoint communication
- AT command send and response read
- Raw response display with masking enabled by default
- Basic response status detection
- Standard workflow AT command presets
- Repository-managed vendor and carrier file preset examples
- User-defined presets through TOML single-file and drop-in configuration
- Command risk classification
- Confirmation flow for write, persistent, and dangerous presets
- TUI AT command input
- TUI current response copy
- Sensitive value masking
- Session logs and command history
- TUI for device, preset, command, response, and status workflows
- PTY bridge mode for terminal-tool compatible operation
- Homebrew packaging

### 6.2 Out of Scope

The product MUST NOT include the following unless a later revision to this
specification defines them:

- Complete multi-modem validation
- Claims of broad modem compatibility without validation evidence
- QMI, MBIM, or PPP data-session management
- OS-level mobile network connection management
- ModemManager replacement behavior
- Firmware update workflows
- Carrier certification or radio test tooling
- Automatic APN-changing command execution
- Background daemon or service mode
- GUI application
- Intel Mac, Linux, Windows, universal binary, or cross-compilation validation

## 7. Platform Policy

REQ-PLAT-001: The supported runtime and validation platform MUST be
macOS on Apple Silicon.

REQ-PLAT-002: Documentation MUST describe the project as Apple Silicon
Mac-first.

REQ-PLAT-003: Linux, Intel Mac, Windows, and universal binary support MUST NOT
be claimed unless explicitly validated and documented.

REQ-PLAT-004: Implementation SHOULD avoid unnecessary macOS-specific coupling
inside transport-agnostic layers, but portability MUST NOT be used to expand
the validation scope.

## 8. Dependency Policy

### 8.1 Runtime Dependency

`atctl` depends on native `libusb`, not `usblib`.

REQ-DEP-001: The Homebrew formula MUST declare `libusb` as a runtime dependency.

REQ-DEP-002: End-user installation docs MUST present the normal install flow
with the fully qualified formula name:

```sh
brew install uchimanajet7/atctl/atctl
```

They MAY also show the equivalent tapped form, `brew tap uchimanajet7/atctl`
followed by `brew install atctl`.

REQ-DEP-003: End-user docs MUST explain that Homebrew should install `libusb`
automatically through the `atctl` formula.

REQ-DEP-004: Manual `libusb` installation MUST be documented only as a fallback
or source-development prerequisite, not as the normal end-user flow.

### 8.2 Development and Build Dependencies

Developers need:

- Rust toolchain: compiler and Cargo
- `libusb`: native USB access library
- `pkgconf`: Homebrew formula that provides the `pkg-config` command used by
  native build scripts to locate `libusb`
- `git`: source control, if cloning the repository

REQ-DEP-004A: The Rust compiler baseline MUST be declared in `Cargo.toml` with
`rust-version`. The current baseline is Rust 1.96 with Edition 2024.

REQ-DEP-004B: Dependency maintenance MUST update `Cargo.toml` and `Cargo.lock`
together when direct dependency baselines change. Lockfile-only updates MAY be
used for compatible transitive refreshes, but they MUST NOT be presented as a
direct dependency baseline change.

REQ-DEP-004C: Dependency maintenance MUST use current Cargo/crates.io
information before changing direct dependency versions and MUST run the normal
Rust verification gate before the update is reported complete.

REQ-DEP-004D: The source repository MUST carry Dependabot version-update
configuration for both Cargo dependencies and GitHub Actions references.

REQ-DEP-004E: Pull requests that change Cargo dependencies or GitHub Actions
workflow dependencies MUST run a dependency review workflow. Vulnerabilities at
moderate severity or higher MUST fail the review.

REQ-DEP-004F: The source repository MUST carry a scheduled and manually
triggerable dependency-maintenance workflow that checks the Cargo lockfile,
RustSec advisories, direct dependency drift, duplicate dependency tree signal,
and GitHub Actions workflow syntax.

REQ-DEP-004G: The scheduled dependency-maintenance workflow MUST use a
non-zero minute value in its cron expression instead of scheduling exactly at
the start of an hour.

REQ-DEP-004H: Local dependency maintenance MUST use the same project scripts as
the scheduled dependency-maintenance workflow. Direct dependency baseline
changes still MUST be represented explicitly in `Cargo.toml`; automation MUST
NOT hide a direct baseline change as only a lockfile refresh.

REQ-DEP-004I: Remote GitHub Actions used by source-repository workflows MUST be
pinned by full-length commit SHA. The same line SHOULD keep the corresponding
version tag comment so Dependabot can update the documented version reference
when a newer action version is available.

REQ-DEP-005: Development docs MUST list dependency install commands separately
with comments explaining purpose.

REQ-DEP-006: Development docs MUST NOT use a single unexplained combined line
such as `brew install rust libusb pkg-config`.

REQ-DEP-007: For Homebrew source builds, the formula SHOULD declare:

```ruby
depends_on "rust" => :build
depends_on "pkgconf" => :build
depends_on "libusb"
```

Rationale: `rusb` uses `libusb1-sys`, and `libusb1-sys` expects native `libusb`
to be discoverable through `pkg-config`.

## 9. External Technical Basis

The following external facts are part of the current implementation basis:

- SORACOM documentation states that Onyx uses Quectel EG25-G internally.
- SORACOM troubleshooting documents APN check and APN set commands using
  `AT+CGDCONT?` and `AT+CGDCONT=1,"IP","soracom.io"`.
- SORACOM's advanced data-send/receive troubleshooting reference, last updated
  2025-04-23, lists AT command checkpoints for modem/SIM recognition, carrier
  selection visibility, signal strength, network registration, APN/PDP state,
  PS attach, and session activation:
  `ATI`, `AT+CIMI`, `AT+COPS?`, `AT+COPS=?`, `AT+CSQ`, `AT+CREG?`,
  `AT+CGREG?`, `AT+CEREG?`, `AT+CGDCONT?`, `AT+CGATT?`, and `AT+CGACT?`.
  Source: https://users.soracom.io/ja-jp/guides/diagnostic/advanced/
- 3GPP TS 27.007 defines standard AT commands used as adjacent manual
  troubleshooting checks for the same path, including `AT+WS46?`,
  `AT+WS46=?`, `AT+CESQ`, `AT+CESQ=?`, `AT+CGAUTH?`, `AT+CGAUTH=?`,
  `AT+CGPADDR`, and `AT+CGPADDR=?`.
- SORACOM APN documentation lists APN values for multiple subscription types,
  including `soracom.io`, `du.soracom.io`, and `m-airsim.jp`, and SORACOM CHAP
  documentation describes group-level CHAP usernames and passwords.
- Quectel's EG25-G product page identifies the module as LTE Cat 4 with USB
  2.0 high-speed interface and references AT command documentation.
- Quectel QCFG documentation defines `AT+QCFG` commands, including network scan
  mode query and configuration forms used by Quectel modem troubleshooting.
- `libusb` provides device enumeration, opening, interface claiming, endpoint
  I/O, kernel driver active/detach handling, and auto-detach behavior.
- `rusb` is a Rust safe wrapper around native `libusb`.
- `ratatui` and `crossterm` are suitable libraries for a Rust terminal UI.
- `portable-pty` provides a cross-platform Rust PTY abstraction with a native
  PTY system, master/slave PTY pairs, reader/writer access on the master side,
  and a slave tty name suitable for compatibility workflows.
- The `ctrlc` crate with its `termination` feature provides a Rust signal
  handler path for SIGINT, SIGTERM, and SIGHUP cleanup on Unix-like systems.
- Nielsen Norman Group's filter and list-order guidance treats filter
  categories and values as user-facing decision aids that should be
  appropriate, predictable, jargon-free, and prioritized. It also recommends
  logical, ordinal, importance, or frequency-based ordering when items have an
  inherent workflow order instead of defaulting to alphabetical order.
  Sources:
  https://www.nngroup.com/articles/filter-categories-values/
  https://www.nngroup.com/articles/alphabetical-sorting-must-mostly-die/
- VS Code Marketplace, JetBrains Marketplace, GitHub CLI extension search, and
  `kubectl get` output sorting all expose explicit user-facing ordering,
  ranking, or sort controls rather than relying on incidental internal load
  order for list presentation.
  Sources:
  https://code.visualstudio.com/docs/configure/extensions/extension-marketplace
  https://plugins.jetbrains.com/docs/marketplace/plugins-ranking.html
  https://cli.github.com/manual/gh_extension_search
  https://kubernetes.io/docs/reference/kubectl/
- TOML v1.0.0 arrays of tables are inserted into arrays in the order
  encountered, which makes `[[presets]]` entry order a suitable author-controlled
  order within a preset file.
  Source: https://toml.io/en/v1.0.0

## 10. Architecture

The design MUST separate USB transport from AT command logic and user
interfaces.

```text
CLI / TUI / PTY bridge
  |
Application service layer
  |
AT command engine
  |
Transport trait
  |-- UsbAtTransport    rusb / libusb
  |-- SerialTransport    optional extension
  `-- PtyBridge          advanced mode over transport
```

REQ-ARCH-001: The AT command engine MUST depend on a transport trait, not on
`rusb` directly.

REQ-ARCH-002: USB descriptor inspection MUST be implemented in a USB module,
not inside CLI rendering code.

REQ-ARCH-003: Preset loading, risk classification, masking, and logging MUST be
usable by both CLI and TUI code paths.

REQ-ARCH-004: PTY bridge behavior MUST build on the same core USB transport and
AT send workflow used by the CLI.

REQ-ARCH-005: PTY bridge code MUST keep a thin implementation boundary so that
future Linux support can be investigated without rewriting unrelated command,
transport, masking, logging, or safety logic.

REQ-ARCH-006: Shared AT execution behavior MUST be implemented below the
surface layer when practical, so TUI, CLI, and PTY bridge behavior do not drift
into separate product semantics.

REQ-ARCH-007: Loaded product-provided, repository-managed, and user-authored
execution definitions MUST converge before user-facing listing, selection, risk,
masking, logging, raw diagnostic export, and execution logic where the behavior
is applicable. The shared representation MUST carry origin metadata so source
labels, explicit-loading boundaries, duplicate-name errors, and review
responsibility remain visible.

REQ-ARCH-007A: Product-provided built-in definitions MAY be authored as Rust or
data-like internal definitions. They MUST NOT be forced through the runtime file
preset or Sequence TOML loader merely for implementation symmetry. Product
definitions and TOML file definitions MUST instead normalize through an internal
Definition or Draft conversion boundary before becoming shared `Preset` or
`Sequence` execution models. The conversion boundary MUST attach or preserve
origin metadata so product-provided, repository-managed, and user-authored
responsibility classes remain visible where the product shows source context.

REQ-ARCH-008: A feature is product-complete only when the relevant CLI, TUI, and
PTY bridge surfaces satisfy the requirements in this specification or this
specification explicitly defines a surface-specific difference.

## 11. Repository Structure

The implementation uses the following responsibility boundaries:

```text
atctl/
  Cargo.toml
  Cargo.lock
  LICENSE
  README.md
  README-ja.md
  CHANGELOG.md
  docs/
    SPEC.md
    INSTALL.md
    DEVELOPMENT.md
    PACKAGING.md
    TROUBLESHOOTING.md
    SAFETY.md
    PRESETS.md
    DECISIONS.md
  src/
    main.rs
    lib.rs
    cli.rs
    cli/
      tests.rs
    paths.rs
    app/
      mod.rs
      errors.rs
    usb/
      mod.rs
      device.rs
      descriptor.rs
      endpoint.rs
      transport.rs
    transport/
      mod.rs
      traits.rs
      usb.rs
      pty.rs
      test_support.rs
    at/
      mod.rs
      command.rs
      parser.rs
      response.rs
      risk.rs
      mask.rs
    presets/
      mod.rs
      builtin.rs
      definition.rs
      loader.rs
      model.rs
    sequences/
      mod.rs
      builtin.rs
      definition.rs
      engine.rs
      engine/
        tests.rs
      loader.rs
      model.rs
    tui/
      mod.rs
      clipboard.rs
      response_state.rs
      theme.rs
      tests.rs
    log/
      mod.rs
      history.rs
      raw.rs
      session.rs
  examples/
    presets/
    sequences/
```

The project license is MIT. The repository includes the root `LICENSE` file
with the recorded copyright holder, and `Cargo.toml` declares
the standard SPDX license expression `license = "MIT"`. See
`docs/DECISIONS.md` OQ-002 for the recorded license decision.

## 12. Transport Trait

The exact Rust API may change during implementation, but the conceptual
contract MUST remain:

```rust
pub trait AtTransport {
    fn open(&mut self) -> Result<(), AtctlError>;
    fn close(&mut self) -> Result<(), AtctlError>;
    fn write_command(&mut self, command: &str) -> Result<(), AtctlError>;
    fn read_response(&mut self, timeout: Duration) -> Result<Vec<u8>, AtctlError>;
    fn transact(&mut self, command: &str, timeout: Duration) -> Result<AtResponse, AtctlError>;
}
```

REQ-TR-001: `write_command` MUST append exactly one carriage return (`\r`) if
the caller did not provide a command terminator.

REQ-TR-002: `transact` MUST write the command, read until completion or timeout,
and return structured status plus raw bytes.

REQ-TR-003: Transport implementations MUST release resources on drop or
shutdown where the platform API allows it.

## 13. USB Transport

USB transport responsibilities:

- Enumerate USB devices
- Match optional user-provided VID/PID selectors
- Open device handles
- Read device and configuration descriptors
- Inspect interfaces, alternate settings, and endpoints
- Select or validate a candidate bulk IN / bulk OUT endpoint pair
- Claim the selected interface
- Send command bytes to bulk OUT
- Read response bytes from bulk IN
- Release the interface on shutdown

REQ-USB-001: The implementation MUST support explicit VID/PID selection.

REQ-USB-002: The implementation MUST support explicit interface and endpoint
overrides.

REQ-USB-003: The implementation MUST support endpoint auto-detection by scanning
interfaces and alternate settings for bulk IN / bulk OUT pairs.

REQ-USB-004: The specification and implementation MUST NOT define a fixed
interface, bulk IN endpoint, or bulk OUT endpoint value as the normative
SORACOM Onyx / Quectel EG25-G mapping.

REQ-USB-005: When multiple matching USB devices exist, the implementation MUST
provide enough identifying information to let the user select one. Bus number,
device address, port path, product string, manufacturer string, serial number
when available, VID, and PID SHOULD be displayed.

REQ-USB-006: Interface claim failures MUST produce actionable errors that
mention likely causes such as another process using the interface, device
disconnect, permission/access denial, or kernel driver ownership.

REQ-USB-007: The implementation MUST inspect and report alternate settings.
If an alternate setting is needed for endpoint access, the behavior MUST be
explicit and logged.

REQ-USB-008: The implementation SHOULD call or expose `rusb`/`libusb` kernel
driver active and auto-detach capabilities when supported, but MUST NOT assume
that detach behavior works on every macOS environment.

REQ-USB-009: Device reset MUST NOT be performed automatically. Modem
functionality changes, modem restart commands, and vendor-specific power-down
commands MAY exist only as explicit user-selected dangerous actions with typed
confirmation.

### 13.1 Endpoint Auto-Detection

Auto-detection MUST be conservative:

1. Enumerate candidate interfaces and alternate settings.
2. Identify pairs containing one bulk IN endpoint and one bulk OUT endpoint.
3. Prefer candidates whose class/subclass/protocol and endpoint layout match
   known AT-command interfaces when that metadata is available.
4. Claim one candidate at a time.
5. Probe with the safe command `AT\r`.
6. Read until `OK`, `ERROR`, or timeout.
7. Release failed candidates before trying the next candidate.
8. Report the selected interface and endpoints.

REQ-USB-010: Auto-detection MUST NOT use write/configuration commands as probes.

REQ-USB-011: Auto-detection MUST stop after the first candidate that returns a
valid AT final result unless the user requested a diagnostic mode that reports
all candidates.

REQ-USB-012: If auto-detection fails, the error MUST instruct the user to run
`atctl inspect` and retry with explicit `--interface`, `--bulk-in`, and
`--bulk-out` options.

REQ-USB-013: Endpoint selection MUST be a runtime result of descriptor
inspection plus AT probe, or an explicit manual override.

REQ-USB-014: Endpoint values observed on real hardware MAY be documented only as
evidence or observation, not as required implementation constants.

REQ-USB-015: Observed endpoint evidence SHOULD include device identity, VID/PID,
operating system, observation date, `atctl inspect` output, successful `AT`
probe result, and selected interface/endpoint pair.

## 14. AT Response Handling

REQ-AT-001: The AT response parser MUST detect these final statuses:

- `OK`
- `ERROR`
- `+CME ERROR:`
- `+CMS ERROR:`

REQ-AT-002: Detection MUST treat final result codes as line-oriented AT results,
not as arbitrary substring matches inside payload text.

REQ-AT-003: The parser MUST preserve raw response bytes for display and logging
after masking rules are applied.

REQ-AT-004: The parser MUST tolerate command echo.

REQ-AT-005: The parser SHOULD classify known final result codes such as
`NO CARRIER` as non-OK terminal results when encountered.

REQ-AT-006: URCs and delayed intermediate lines MUST NOT cause the parser to
drop data. If the parser cannot fully classify URCs, it MUST preserve
them in raw output and document the limitation.

REQ-AT-007: The parser MUST expose timeout as a distinct status from AT `ERROR`.

Default timeouts:

```text
USB write timeout: 3 seconds
Endpoint auto-detection AT probe timeout: 3 seconds
User AT command timeout: 30 seconds
```

Preset definitions MAY declare a recommended per-command timeout. A preset
timeout is not a transport default; it is a command-specific time budget for
known long-running commands.

REQ-AT-008: Write, persistent, and dangerous commands MUST NOT be automatically
retried.

REQ-AT-009: Optional retry for safe read-only commands MAY be added later, but
it MUST be disabled by default until explicitly specified.

### 14.1 Multi-Step AT Sequence Handling

Sequences are named multi-step AT operations. They are required for workflows
that cannot be represented as one command followed by one final response, such
as SMS sending, prompt/body/Ctrl-Z commands, vendor socket commands that return
delayed URCs, and data-send checks that need a request plus a received response.

REQ-SEQ-ENGINE-001: The Sequence engine MUST execute inside one shared AT
transport session. It MUST preserve step order, masking context, raw diagnostic
export context, risk enforcement, and timeout accounting across the entire
Sequence.

REQ-SEQ-ENGINE-002: The Sequence engine MUST support these step primitives:

```text
send AT command line
wait for final result
wait for prompt such as ">"
send payload bytes
send control terminator such as Ctrl-Z or ESC
wait for URC or intermediate line pattern
capture response text for later validation or display
```

REQ-SEQ-ENGINE-003: Prompt waits and URC waits MUST be first-class Sequence
steps. They MUST NOT be approximated by concatenating multiple AT command lines
into a one-shot `send` command string.

REQ-SEQ-ENGINE-004: The Sequence engine MUST distinguish final result status,
prompt events, URCs, intermediate lines, timeout, and transport errors in the
step transcript. It MUST preserve raw bytes for raw diagnostic export while
normal display, history, and session logs remain masked by default. Explicit
Response export follows the selected foreground masking mode.

REQ-SEQ-ENGINE-004A: Sequence text transcripts MUST separate output by origin
using stable section labels. Operator-sent commands MUST be shown under
`Command:` and operator-sent payload material under `Payload:`. Modem-returned
lines MUST be shown under `Modem response:`. Decoded SMS body values MUST be
shown under `Decoded SMS:`. Product-derived interpretation, including text
generated from a Sequence definition `evidence` field, MUST be shown under
`Analysis:`. Sequence success notes MUST be shown under `Notes:`. The final
status and duration MUST be shown under `Result:`. Text transcripts MUST
separate each section block with a single blank line so boundaries are visible
without making the user parse run-together lines. They MUST NOT use decorative
divider lines in normal transcripts, and they MUST preserve modem response
line breaks inside the `Modem response:` section.

REQ-SEQ-ENGINE-004B: Every Sequence transcript line and JSON field MUST have a
defined origin. Verification MUST cover the required origin labels and the
absence of `Evidence:` text and JSON `evidence`. CLI JSON Sequence output MUST
expose derived interpretation as `analysis`, not `evidence`.

REQ-SEQ-ENGINE-005: Sequence risk MUST be computed from declared Sequence risk,
step command classification, payload sensitivity, parameter sensitivity, and
known side effects. A Sequence definition MUST NOT be able to downgrade the risk
required by any step.

REQ-SEQ-ENGINE-006: Sequence timeout handling MUST support a total Sequence
timeout and per-step timeout hints. A user-supplied command timeout is an
override for the execution budget, not a guarantee that a modem-side operation
can be cancelled safely.

REQ-SEQ-ENGINE-007: Sequence output MUST include enough context for a human to
verify what was attempted and what evidence was returned. For example,
`+CMGS` followed by `OK` is evidence that an SMS submit operation was accepted
by the modem/network path, not proof that the destination handset displayed the
message. SMS `+CMGL` and `+CMGR` output MUST be parsed into message status,
sender, timestamp, raw body, decoded body when possible, and decode status.
Normal Response, Response export, session logs, history, and JSON output MUST
keep sender and decoded body values masked by default. Explicit `--no-mask`
foreground output and Response export MAY expose those values. Raw diagnostic
export MAY expose the underlying exchange only through its separate sensitive
output controls.

REQ-SEQ-ENGINE-007A: Normal Sequence text transcripts, TUI Response, saved
Response, history, and session logs MUST NOT render the literal `Evidence:`
prefix. The word `evidence` remains a domain concept and a Sequence definition
field name, but user-visible transcript output MUST label atctl-derived
interpretation as `Analysis:` so it is not mistaken for modem-originated output
or a modem response line.

REQ-SEQ-ENGINE-008: SMS body decoding MUST run before normal-output masking so
masking does not corrupt UCS2 hex or other encoded bodies before the product can
interpret them. The supported decode path MUST handle UCS2 hexadecimal
SMS text as UTF-16BE. Plain text bodies MAY be reported as text. If a body
cannot be decoded with a supported path, the transcript MUST state that decode
status instead of guessing a lossy body.

REQ-SEQ-ENGINE-009: A Sequence MAY derive later step template values from an
earlier AT response only when that derivation is product-specified and visible
in the transcript. The required derived value is `sms_sender`, extracted
from `AT+CMGR` for SMS reply-by-index. Derived sensitive values MUST be masked
under the same normal-output rules as user-entered sensitive parameters.

REQ-SEQ-ENGINE-010: For Quectel TCP/IP socket checks, `+QIOPEN: <id>,0` is
evidence that the socket opened, `SEND OK` is evidence that the module accepted
the payload for sending, `AT+QISEND=<id>,0` counters are TCP/socket
acknowledgement evidence, and a received echo or application response read
through `QIRD` or remote endpoint evidence is needed to verify end-to-end data
exchange. `+QIRD: 0` means there is no buffered receive data and MUST NOT be
presented as application response evidence. Because TCP is a byte stream,
product wording MUST NOT imply that one socket write equals one remote
application message unless an application framing format is also in use.
When a TCP Sequence step declares that TCP acknowledgement is required, atctl
MUST treat `+QISEND:` counters with remaining unacknowledged payload bytes as
an incomplete send condition, retry acknowledgement queries within the step
timeout, and fail the Sequence rather than closing the socket and reporting
`Result: OK` if the payload is still not acknowledged.

REQ-SEQ-ENGINE-011: If a Sequence changes modem state only to perform a check,
the state change MUST be explicit in the Sequence summary and risk. Optional
restore steps MAY be supported, but restore behavior MUST be visible and MUST
not be described as guaranteed if the modem or transport can fail before the
restore step runs.

## 15. CLI Specification

CLI commands are production product surfaces. They are not merely validation,
test, or automation hooks for the TUI. CLI behavior may use command-line flags,
standard input, standard output, and exit codes, but it must preserve the same
product semantics, safety rules, masking defaults, and diagnostic guarantees as
the corresponding TUI workflow when the same capability is exposed in both
surfaces.

Required command groups:

```sh
atctl devices
atctl inspect
atctl send <COMMAND>
atctl preset list
atctl preset run <NAME>
atctl sequence list
atctl sequence run <SEQUENCE>
atctl tui
atctl bridge --symlink <PATH>
atctl logs list
```

### 15.1 Common USB Options

Commands that access a device SHOULD accept:

```text
--vid <VID>
--pid <PID>
--bus <BUS>
--address <ADDRESS>
--interface <N>
--bulk-in <ENDPOINT>
--bulk-out <ENDPOINT>
--timeout <SECONDS>
```

REQ-CLI-001: VID, PID, and endpoint parser behavior MUST accept hexadecimal
values such as `0x2c7c` and `2c7c`.

REQ-CLI-002: If more than one matching device exists and the user did not
select one, the command MUST fail with a device-selection error rather than
guessing.

### 15.2 `atctl devices`

REQ-CLI-DEV-001: By default, `atctl devices` MUST show only USB devices that
are plausible `atctl` operation targets based on current runtime USB
descriptors. This default view MUST reduce unrelated USB noise such as hubs,
LAN adapters, webcams, microphones, and billboard devices.

REQ-CLI-DEV-002: `atctl devices` MUST NOT depend on any built-in known-device
list, supported-device table, profile list, or default allow-list. The command's
discovery source is the set of USB devices visible through `libusb` at runtime.

REQ-CLI-DEV-003: `atctl devices` output MUST prefer USB descriptor values and
explicit selector fields. It MUST NOT present any built-in device label, profile
label, compatibility label, or implementation-defined product name as if it were a USB
descriptor value or product identity.

REQ-CLI-DEV-004: `atctl devices --all-usb` MUST show all matching USB devices
visible through `libusb`, including devices that are not plausible AT operation
targets. This is a troubleshooting view and MUST NOT be the default first-time
selection workflow.

REQ-CLI-DEV-005: The default operation-target filter MUST be descriptor-based,
not product-name-based. The filter uses a conservative
descriptor candidate rule: a USB device class commonly used by communication,
miscellaneous, or vendor-specific modem devices, plus at least one descriptor
shape that contains both bulk IN and bulk OUT endpoints. This rule is not a
confirmed AT probe and MUST NOT be documented as guaranteed modem support.

REQ-CLI-DEV-006: The default operation-target filter MUST be documented with
its descriptor basis and known limits. Device class filtering alone is not
sufficient for universal modem discovery because USB-IF defines `00h` in a
Device Descriptor as "use class code info from Interface Descriptors." If an
expected target is hidden by the default filter, the supported workaround is to
inspect all visible USB devices with `atctl devices --all-usb`, inspect the
candidate's descriptors with
`atctl inspect --bus <BUS> --address <ADDRESS>`, and then use explicit runtime
selectors for the CLI or bridge path. Expanding default filtering to evaluate
Interface Descriptor class codes is allowed only as a descriptor-based
improvement, not as a product-name or known-device allow-list.

Example:

```text
EG25-G 0x2c7c:0x0125 bus=1 address=5
  manufacturer=Quectel
  product=EG25-G
```

### 15.3 `atctl inspect`

REQ-CLI-INSP-001: `atctl inspect` MUST show configuration, interface, alternate
setting, endpoint address, endpoint direction, endpoint transfer type, and max
packet size when available.

REQ-CLI-INSP-002: `atctl inspect` SHOULD mark candidate AT-command endpoint
pairs and explain whether they were detected by descriptor shape only or by a
probe.

REQ-CLI-INSP-003: `atctl inspect` MUST distinguish descriptor-shape candidates,
AT-probe-selected candidates, and user-specified manual overrides when reporting
endpoint selection.

### 15.4 `atctl send <COMMAND>`

REQ-CLI-SEND-001: `atctl send <COMMAND>` MUST send one AT command and print the
response.

REQ-CLI-SEND-002: Output MUST be masked by default.

REQ-CLI-SEND-003: Raw unmasked output MUST require explicit `--no-mask`.

REQ-CLI-SEND-003A: `--no-log` MUST prevent creation of new masked command
history and session logs for that invocation. It MUST NOT disable an explicitly
requested raw diagnostic export.

REQ-CLI-SEND-004: Raw diagnostic export MUST require explicit
`--raw-log-file <PATH>`. `atctl send` MUST NOT create raw logs automatically,
MUST NOT use a default raw-log path, and MUST refuse to overwrite an existing
raw export file.

REQ-CLI-SEND-005: The command MUST return a non-zero exit code on USB errors,
timeout, and AT error unless `--ignore-at-error` is specified for AT errors.

REQ-CLI-SEND-006: `--ignore-at-error` MUST NOT hide transport failures or
timeouts.

REQ-CLI-SEND-007: JSON output MUST preserve the same masking defaults as text
output.

REQ-CLI-SEND-008: Plain read/test direct commands MAY run without confirmation
and MAY print plain output.

REQ-CLI-SEND-009: Read/test direct commands that expose sensitive identifiers or
credentials MAY run without confirmation, but output and logs MUST be masked by
default.

REQ-CLI-SEND-010: Unknown read/test direct commands MAY run without
confirmation, but MUST be treated as sensitive by default.

REQ-CLI-SEND-011: Write, change, delete, persistent, dangerous, and non-read/test
unknown direct commands MUST require explicit confirmation.

REQ-CLI-SEND-011A: Interactive confirmation for direct `send` MUST show the
normalized command, classified risk level, and classifier reason before any USB
access.

REQ-CLI-SEND-011B: Interactive confirmation for direct `send` MUST require the
user to type the exact classified risk level, such as `write`, `persistent`, or
`dangerous`.

REQ-CLI-SEND-011C: If interactive confirmation is required but standard input
is not a terminal, direct `send` MUST fail before USB access.

REQ-CLI-SEND-012: For direct `send`, `--yes` alone MUST NOT bypass confirmation
for commands that require confirmation.

REQ-CLI-SEND-013: For direct `send`, non-interactive confirmation bypass MUST
require both `--yes` and `--risk-ack <risk>`.

REQ-CLI-SEND-014: `--risk-ack <risk>` MUST match the implementation's classified
command risk. If it does not match, `atctl` MUST fail before sending anything to
the modem.

REQ-CLI-SEND-015: Direct-send implementation MUST include a maintained
risk-pattern table for known dangerous and persistent command families before
direct write/change behavior is considered complete.

REQ-CLI-SEND-016: `--raw-log-file <PATH>` MUST write the modem exchange to the
specified file while preserving normal terminal output behavior. Masked terminal
output remains masked by default. `--no-mask` affects terminal output only and
MUST NOT affect command history, session logs, or raw-export acknowledgement.

REQ-CLI-SEND-017: Raw diagnostic export acknowledgement MUST be separate from
command risk acknowledgement. Non-interactive direct send, and direct send with
`--yes`, MUST require `--raw-log-ack raw-log` before USB access when
`--raw-log-file <PATH>` is used. Interactive direct send MAY prompt the user to
type `raw-log`.

Required options:

```text
--vid <VID>
--pid <PID>
--interface <N>
--bulk-in <ENDPOINT>
--bulk-out <ENDPOINT>
--timeout <SECONDS>
--no-mask
--no-log
--export-response <PATH>
--raw-log-file <PATH>
--raw-log-ack raw-log
--json
--ignore-at-error
--yes
--risk-ack <RISK>
```

### 15.5 `atctl preset list`

REQ-CLI-PRESET-001: `atctl preset list` MUST list loaded product presets and
loaded file presets.

REQ-CLI-PRESET-002: The list MUST show preset name, command, preset set label,
declared risk, effective risk, timeout hint when present, and categories.
Categories MUST serve as discovery metadata in list output and as workflow
filters in the TUI.

REQ-CLI-PRESET-003: `atctl preset list` MUST distinguish standard workflow
product presets from file presets without requiring color. The standard set
label shown by CLI list output MUST be `Product presets`. The CLI list output
MUST also include a trailing `source-path` column. Product-provided rows MUST
show `-`; file preset rows MUST show the file path that supplied the preset.

REQ-CLI-PRESET-004: Normal startup MUST load product-provided presets. File
preset locations MUST be supplied for each invocation through `--preset-file`
or `--preset-dir`.

REQ-CLI-PRESET-005: Duplicate preset names across loaded presets MUST fail
with an actionable error. Preset loading MUST NOT silently apply last-wins
override behavior.

REQ-CLI-PRESET-006: Repository-managed example file presets MUST use and be
verified through the same multi-file loading path as other file presets. They
are part of the maintained product and verification surface.

REQ-CLI-PRESET-007: `atctl preset run <NAME>` SHOULD use the preset's
`timeout_secs` value when the command would otherwise use the default user AT
command timeout. A user-supplied `--timeout` remains the explicit override.

REQ-CLI-PRESET-008: Preset loading is an application-level feature, not a
TUI-only feature. The same loaded preset set MUST be visible in
`atctl preset list`, executable through `atctl preset run <NAME>`, and
selectable in `atctl tui`.

REQ-CLI-PRESET-009: `atctl preset list`, `atctl preset run <NAME>`, and
`atctl tui` SHOULD support explicit per-invocation file preset location
overrides for temporary or project-local workflows:

```text
--preset-file <FILE>
--preset-dir <DIR>
```

REQ-CLI-PRESET-010: When explicit file preset location flags are provided,
those files and directories MUST be loaded only for that invocation.
Application-provided product presets are still available. Explicit file preset
locations MUST NOT modify environment variables or future invocations.

REQ-CLI-PRESET-011: File preset definitions MUST enter the loaded preset set
through the per-invocation `--preset-file` or `--preset-dir` flags. This keeps
external executable definitions inside the operator's per-invocation trust
boundary.

### 15.6 `atctl preset run <NAME>`

REQ-CLI-PRESET-RUN-001: `<NAME>` MUST resolve one loaded preset by exact name
and execute that preset's single AT command once. Safe presets MAY run directly.

REQ-CLI-PRESET-RUN-002: Sensitive presets MAY run directly but output and logs
MUST be masked by default.

REQ-CLI-PRESET-RUN-003: Write, persistent, and dangerous presets MUST require
confirmation.

REQ-CLI-PRESET-RUN-006: `atctl preset run <NAME>` MUST support the same raw
diagnostic export behavior as direct `atctl send`: explicit
`--raw-log-file <PATH>`, separate `--raw-log-ack raw-log`, no default raw-log
destination, overwrite refusal, and unchanged masked terminal/session/history
logging behavior.

REQ-CLI-PRESET-RUN-006A: `atctl preset run <NAME> --no-log` MUST prevent
creation of new masked command history and session logs for that invocation.
It MUST NOT disable an explicitly requested raw diagnostic export.

REQ-CLI-PRESET-RUN-006B: `atctl preset run <NAME>` MUST support the common
Response export contract in REQ-CLI-RESPONSE-EXPORT-001 through 006.

REQ-CLI-PRESET-RUN-007: When a file preset is executed, CLI and TUI execution
surfaces MUST show the file preset source label, file path, and a concise
notice that `atctl` validates file format, duplicate names, masking, and
effective risk but does not certify the loaded definition for the current
device, SIM, network, or endpoint. CLI `preset run` MUST show this notice
before USB access even when `--yes --risk-ack <risk>` satisfies the risk
confirmation non-interactively. When an interactive risk confirmation is shown,
the notice MUST be integrated into that existing confirmation rather than
creating an additional per-command confirmation prompt.

### 15.7 `atctl sequence list` and `atctl sequence run <SEQUENCE>`

Sequences are production CLI actions for named multi-step AT operations. They
are not a replacement for direct `send` or one-shot `preset run`.

REQ-CLI-SEQ-001: `atctl sequence list` MUST list loaded product-provided
standard Sequences and loaded user or repository-managed Sequence definitions.
The list MUST show Sequence name, Sequence set label when relevant, declared
risk, effective risk, timeout hint when present, categories, required
parameters, a concise human-readable summary, and a trailing `source-path`
column. Product-provided rows MUST show `-`; file Sequence rows MUST show the
file path that supplied the Sequence.

REQ-CLI-SEQ-002: `atctl sequence run <SEQUENCE>` MUST execute a named Sequence
through the shared Sequence engine. It MUST NOT be implemented as repeated
shell calls to `atctl send` because prompt waits, payload writes, URC waits,
per-step timeouts, shared masking, and raw diagnostic export need one coherent
execution context.

REQ-CLI-SEQ-003: `atctl sequence run <SEQUENCE>` MUST accept the same USB
target selection options as direct device commands: `--vid`, `--pid`, `--bus`,
`--address`, `--interface`, `--bulk-in`, `--bulk-out`, and `--timeout`.

REQ-CLI-SEQ-004: `atctl sequence list`, `atctl sequence run <SEQUENCE>`, and
`atctl tui` SHOULD support explicit per-invocation Sequence definition location
overrides for temporary review, repository examples, and project-local
extensions:

```text
--sequence-file <FILE>
--sequence-dir <DIR>
```

REQ-CLI-SEQ-005: When explicit Sequence definition location flags are provided,
those files and directories MUST be loaded only for that invocation.
Product-provided standard Sequences are still available. Explicit Sequence
locations MUST NOT modify environment variables or future invocations.

REQ-CLI-SEQ-006: User-authored Sequence definitions MUST enter the loaded
Sequence set through `--sequence-file` or `--sequence-dir`. They are an
extension point. They MUST NOT be described as a required user step for
product-provided standard SMS Sequences.

REQ-CLI-SEQ-006A: Sequence add-on definitions MUST enter the loaded Sequence
set through the per-invocation `--sequence-file` or `--sequence-dir` flags.
This keeps external executable definitions inside the operator's
per-invocation trust boundary.

REQ-CLI-SEQ-007: Repository-managed vendor/provider Sequence examples,
including Quectel TCP/IP data-send examples and SORACOM TCP endpoint
examples, MUST be loaded through the same Sequence loader path used for user
Sequence definitions. They MUST NOT be promoted to default product-provided
standard Sequences unless this specification is revised to define that
behavior.

REQ-CLI-SEQ-008: `atctl sequence run <SEQUENCE>` MUST support raw diagnostic
export with the same sensitive-export policy as direct `send` and
`preset run`: explicit `--raw-log-file <PATH>`, separate
`--raw-log-ack raw-log`, no default raw-log destination, overwrite refusal, and
unchanged masked terminal/session/history logging behavior.

REQ-CLI-SEQ-008A: `atctl sequence run <SEQUENCE> --output json` MUST include
structured step results and success notes in addition to the selected masked or
raw transcript. Step results MUST include step id, optional label, final status,
and selected masked/raw analysis text when a Sequence step defines `evidence` or
when atctl derives interpretation from the modem response. This structured
output MUST preserve default masking. The JSON field name MUST be `analysis`;
normal Sequence JSON output MUST NOT expose a top-level or per-step `evidence`
field.

REQ-CLI-SEQ-008B: `atctl sequence run <SEQUENCE>` MUST use the same Sequence
value-resolution contract as the TUI. If a required value is missing in a
non-interactive invocation, the error MUST identify the missing parameter and
include the Sequence-provided default value, value source, or resolution hint
when available. CLI help and list output MAY remain compact, but they MUST NOT
hide the fact that a value is supplied by a default, selected from modem output,
derived during execution, provided by the Sequence, or entered by the operator
when that distinction affects how the user obtains the value.

REQ-CLI-SEQ-008C: When a file Sequence is executed, CLI and TUI execution
surfaces MUST show the Sequence source label, file path, and a concise notice
that `atctl` validates file format, duplicate names, masking, and effective
risk but does not certify the loaded definition for the current device, SIM,
network, or endpoint. CLI `sequence run` MUST show this notice before USB
access even when `--yes --risk-ack <risk>` satisfies the risk confirmation
non-interactively. When an interactive risk confirmation is shown, the notice
MUST be integrated into that existing confirmation rather than creating an
additional per-command confirmation prompt.

REQ-CLI-SEQ-008D: `atctl sequence run <SEQUENCE> --no-log` MUST prevent
creation of new masked command history and session logs for that invocation.
It MUST NOT disable an explicitly requested raw diagnostic export.

REQ-CLI-SEQ-008E: `atctl sequence run <SEQUENCE>` MUST support the common
Response export contract in REQ-CLI-RESPONSE-EXPORT-001 through 006.

REQ-CLI-SEQ-009: `atctl send` remains a direct one-shot AT command surface.
`atctl preset run <NAME>` remains a named one-shot preset surface. They MUST
NOT silently accept a multi-step Sequence name as if it were a one-shot
command. If a user supplies a Sequence name to a one-shot surface, the error
SHOULD direct the user to `atctl sequence run <SEQUENCE>`.

### 15.7A Normal Response export

Normal Response export is an explicit operator-selected copy of a bounded
`send`, `preset run`, or `sequence run` result. It is not a generated log and
does not replace stdout, history, session logs, or raw diagnostic export.

REQ-CLI-RESPONSE-EXPORT-001: `atctl send`, `atctl preset run`, and
`atctl sequence run` MUST accept `--export-response <PATH>` with the same
behavior. The complete target file path MUST be supplied for each invocation.

REQ-CLI-RESPONSE-EXPORT-002: The target parent directory MUST already exist.
The target MUST be validated before USB access, and an existing file MUST be
rejected. Export MUST use exclusive file creation and MUST NOT overwrite,
append to, or silently rename an existing file.

REQ-CLI-RESPONSE-EXPORT-003: Normal stdout MUST remain unchanged when export is
requested. A successful export MUST be reported on stderr with the exact target
path so redirected stdout remains a stable data stream.

REQ-CLI-RESPONSE-EXPORT-004: Text export MUST contain the executed command or
Sequence identity and the complete normal Response or transcript without
terminal UI chrome. When `--json` is selected, the export MUST be valid JSON
containing the same logical artifact and the selected masking state.

REQ-CLI-RESPONSE-EXPORT-005: Export MUST be masked by default and MUST follow
`--no-mask` when the operator explicitly selects unmasked foreground output.
This MUST NOT change masking of generated history or session logs and MUST NOT
create or imply raw diagnostic export. Generated CLI help for
`--export-response` MUST state that export follows `--no-mask`, so the
unmasked-file consequence is visible at the option-selection point.

REQ-CLI-RESPONSE-EXPORT-006: Export-file write errors MUST identify the target
path. Export does not change AT final-status handling, including
`--ignore-at-error`.

### 15.8 `atctl bridge --symlink <PATH>`

PTY bridge mode is an advanced production feature for terminal-tool compatible
operation. It is not a validation-only or second-class AT execution path.

REQ-CLI-BRIDGE-001: PTY bridge MUST be implemented after core CLI/TUI stability.

REQ-CLI-BRIDGE-002: PTY bridge MUST document that `screen /tmp/atctl 115200`
uses a PTY compatibility argument and not physical UART baud rate.

REQ-CLI-BRIDGE-003: Symlink overwrite, cleanup, signal handling, multiple
client behavior, and CR/LF translation MUST follow the bridge requirements in
this section.

REQ-CLI-BRIDGE-004: The PTY bridge MUST use `portable-pty`.

REQ-CLI-BRIDGE-005: Platform-specific PTY implementation MUST NOT be treated as
the default direction.

REQ-CLI-BRIDGE-006: If `portable-pty` cannot satisfy required slave path,
symlink, `screen`/`cu`, cleanup, signal handling, or terminal behavior
requirements, the limitation and any platform-specific replacement MUST be
defined in a revision to this specification before implementation.

REQ-CLI-BRIDGE-007: Use of `portable-pty` MUST NOT be documented as a promise of
Linux support for the supported release platform.

REQ-CLI-BRIDGE-008: `atctl bridge --symlink <PATH>` MUST accept the same USB
target selection options as direct device commands: `--vid`, `--pid`, `--bus`,
`--address`, `--interface`, `--bulk-in`, `--bulk-out`, and `--timeout`.

REQ-CLI-BRIDGE-009: Bridge startup MUST resolve the USB target before creating
or replacing the requested symlink. If zero or multiple devices match, startup
MUST fail before creating the symlink.

REQ-CLI-BRIDGE-010: Bridge symlink creation MUST NOT overwrite existing regular
files or directories. Existing symlinks MUST fail by default. An explicit
`--replace-symlink` option MAY replace an existing symlink, but it MUST NOT
replace non-symlink filesystem entries.

REQ-CLI-BRIDGE-011: Bridge shutdown MUST remove only the symlink created by the
current bridge process, and only when the symlink still points to the same PTY
slave path.

REQ-CLI-BRIDGE-012: Bridge signal handling MUST attempt clean shutdown and
symlink cleanup for SIGINT, SIGTERM, and SIGHUP.

REQ-CLI-BRIDGE-013: Bridge input MUST be line-oriented. `CR`, `LF`, and `CRLF`
from the PTY client MUST be treated as command terminators. Empty commands MUST
be ignored. USB command termination MUST continue to be handled by the existing
AT command terminator path.

REQ-CLI-BRIDGE-014: PTY bridge execution MUST NOT bypass the direct command
risk policy. Safe and sensitive commands MAY execute without confirmation.
Sensitive output MUST be masked by default. Write, persistent, dangerous, and
unknown commands MUST require an explicit typed risk acknowledgement from the
PTY client before sending to USB.

REQ-CLI-BRIDGE-015: The bridge supports one external PTY client. Multiple
external clients opening the same PTY symlink at the same time are not a
supported workflow. Documentation MUST state that concurrent PTY clients can
interleave input and are not guaranteed.

REQ-CLI-BRIDGE-016: PTY bridge output MUST be text-oriented and masked by
default. Raw diagnostic export MAY be enabled only with explicit
`--raw-log-file <PATH> --raw-log-ack raw-log`. Bridge raw export MUST write the
underlying AT command exchanges while the PTY client still receives the normal
masked text output.

REQ-CLI-BRIDGE-016A: PTY bridge raw diagnostic export MUST NOT create a default
raw-log path, MUST refuse to overwrite an existing file, and MUST fail before
USB access if the raw-log acknowledgement is missing or wrong.

REQ-CLI-BRIDGE-016B: PTY bridge MUST NOT expose the bounded
`--export-response` operation because one bridge connection is a continuous
terminal session rather than one current Response artifact. Documentation MUST
show how the selected PTY client records the normal masked session transcript.
For GNU Screen, this MUST include startup logging with `-L` and `-Logfile`, and
the interactive `Ctrl-A H` logging toggle. Client transcript logging MUST remain
separate from atctl raw diagnostic export.

REQ-CLI-BRIDGE-017: PTY bridge help and review documentation MUST describe the
first-time runtime discovery workflow. The workflow MUST start with
`atctl devices`, use the current AT operation-target output to choose the
target, prefer `--bus <BUS> --address <ADDRESS>` for exact runtime selection,
and avoid presenting validation-target VID/PID values as required prior
knowledge. `atctl devices --all-usb` MUST be described as a full USB
troubleshooting view, not the normal target-selection path.

REQ-CLI-BRIDGE-018: Closing the PTY client, such as quitting `screen` with
`Ctrl-A`, then `K`, then `y`, MUST be treated as normal bridge shutdown with
the normal symlink cleanup path. PTY client disconnect read/write errors such
as broken pipe, disconnected slave, or platform EIO MUST NOT be reported as USB
transport errors. USB transport errors while the PTY client remains connected
MUST still stop the bridge.

REQ-CLI-BRIDGE-019: Prompt-required AT commands, such as SMS submit commands
that wait for `>`, MUST NOT be treated as completed one-shot commands in PTY
bridge mode. After the required risk confirmation, the bridge MUST send the
command, wait for the prompt, relay the prompt text to the PTY client, accept
the next PTY line as payload, append the required control terminator such as
Ctrl-Z, then read and return the final modem response. PTY output remains
masked by default, and raw diagnostic export remains explicit.

PTY bridge runtime contract:

- `atctl bridge --symlink <PATH>` creates a `portable-pty` slave and exposes it
  through the requested symlink.
- USB selection is explicit runtime discovery. The bridge does not use a
  known-device table or hard-coded product profile.
- First-time users discover the target with `atctl devices`; `--bus` and
  `--address` are the preferred exact selectors copied from the current output.
  `--vid` and `--pid` are useful only when unique in the current output.
- Existing regular files and directories are never overwritten. Existing
  symlinks require `--replace-symlink`.
- Cleanup removes only the still-matching symlink created by the active bridge.
- Signal handling attempts cleanup for SIGINT, SIGTERM, and SIGHUP.
- PTY input is CR/LF line-oriented and empty lines are ignored.
- Confirmation-required commands prompt on the PTY and require the exact risk
  label on the next line before sending.
- Prompt-required manual multi-step commands wait for the prompt, collect the
  next PTY line as payload, append Ctrl-Z, and return the final response.
- `screen /tmp/atctl 115200` and equivalent clients use the speed argument as a
  serial-tool compatibility value only.
- A terminal client owns optional normal session-transcript recording. With GNU
  Screen, `-L -Logfile <PATH>` enables it at startup and `Ctrl-A H` toggles it
  during a session. The transcript contains the normal masked bridge output.
- Closing the `screen` client is normal bridge shutdown and runs symlink
  cleanup.

## 16. TUI Specification

The TUI is the main interactive product surface. It should make common AT
command workflows easier without hiding safety. TUI behavior MUST NOT be reduced
or omitted merely because a CLI or PTY bridge path exists.

Required panes:

```text
Devices    | Categories | Commands / Sequences
Status     | Categories | Commands / Sequences
----------------------------------
Controls   | Response   | Logs
```

Required key bindings:

```text
Up / Down     Move selection or scroll focused pane
Left / Right  Move between panes
PageUp/Down   Move or scroll the focused pane by one page
Home / End    Jump to the first or last item/line in the focused pane
Enter         Execute the focused pane's primary action: select a device in Devices, move from Categories into Commands / Sequences, run or open input for the selected command/Sequence in Commands / Sequences, use a Controls row, open Response actions, or open Log actions
/             Search commands and Sequences
Esc           Cancel the active input, dialog, or help overlay
q             Quit
?             Help
```

REQ-TUI-001: TUI command and Sequence execution MUST use the same transport,
masking, logging, and risk policy as CLI execution.

REQ-TUI-002: USB I/O MUST NOT permanently block terminal rendering or terminal
restoration. TUI command execution MUST be performed through an execution path
that lets the TUI continue periodic redraws while a command is running.

REQ-TUI-003: The TUI MUST restore the terminal on normal exit and on handled
errors where possible.

REQ-TUI-004: Sensitive output MUST be masked by default.

REQ-TUI-004A: TUI foreground output masking MUST be separate from raw
diagnostic export. The TUI output masking setting MAY control what the current
session displays and copies from the Response pane. Raw diagnostic export MUST
control raw export capture after an explicit path and `raw-log`
acknowledgement, and MUST NOT be enabled merely because foreground output
masking is disabled.

REQ-TUI-RAWLOG-001: TUI raw diagnostic export capture MUST ask the user for an
output path and exact `raw-log` acknowledgement before starting. It MUST refuse
to overwrite an existing file, MUST use user-only file permissions when
supported, and MUST show visible capture state while active.

REQ-TUI-RAWLOG-002: TUI raw diagnostic export capture MUST apply only to AT
commands executed after capture starts. Stopping capture MUST leave existing
masked Response, history, session log, and explicit Response export behavior
unchanged.

REQ-TUI-005: Dangerous commands MUST NOT be presented as ordinary diagnostic or
read-only commands. If a dangerous preset is visible in the TUI, the command row
MUST show the `[dangerous]` risk label and execution MUST require the exact typed
risk confirmation before USB access.

REQ-TUI-006: TUI confirmation dialogs MUST show the command, risk level, and
expected effect before execution.

REQ-TUI-007: TUI confirmation dialogs MUST require typing the exact displayed
risk level before sending a confirmation-required command.

REQ-TUI-008: During command execution, the TUI MUST show that an AT command is
running and MUST show the preset name, raw AT command string, risk level, and
expected effect. During Sequence execution, the TUI MUST show that a Sequence is
running, the Sequence name, current step, risk level, timeout budget, and
expected effect of the current step without duplicating the full transcript in
Status.

REQ-TUI-008A: During command execution, the TUI MUST show elapsed time,
configured timeout, remaining time, and a visible timeout-budget progress
indicator. The indicator represents elapsed time against the timeout budget; it
MUST NOT imply modem-internal completion percentage.

REQ-TUI-008B: The TUI timeout-budget indicator SHOULD use Ratatui `LineGauge`
or an equivalent compact progress bar. In the compact Status pane,
running-command progress SHOULD be rendered as a small temporary progress
block: a visual separator, a short text line for elapsed time, timeout budget,
and remaining time, and a separate progress-bar line when vertical space is
available. The progress-bar line SHOULD use a stronger filled/unfilled shape
cue, such as block and shade symbols, rather than a thin line only. When height
is constrained to one row, the TUI MAY keep the label and the compact bar on the
same row, but the bar SHOULD still preserve a non-color filled/unfilled shape
cue when width permits. The text line SHOULD use compact wording such as
`Timeout 33/180s left 147s` rather than verbose wording that can look truncated
in the compact Status width. The progress bar MUST remain distinguishable from
persistent selected-command status and MUST NOT rely on color alone.

REQ-TUI-008C: Long-running AT command handling MUST be defined for AT commands
as a class, not only for `AT+COPS=?`. `AT+COPS=?` is an example of a
long-running command, not the only command that can run long enough for timeout
and interruption behavior to matter.

REQ-TUI-008D: The TUI MUST NOT offer a normal command `Cancel` action for a
running AT command unless the implementation can truthfully guarantee one of the
following: the modem-side command execution has stopped, or the transport session
has been explicitly abandoned and resynchronized before any subsequent command
can be sent. Stopping only the local host-side read wait MUST NOT be described as
successful command cancellation.

REQ-TUI-008E: The TUI SHOULD handle long-running AT commands
with visible running state, elapsed time, timeout, remaining time, timeout-budget
progress, timeout override, and blocking of conflicting command sends. It SHOULD
NOT add a normal cancel key while modem-internal cancellation semantics remain
unverified.

REQ-TUI-008F: Running-command interruption, host-side read abort, USB reconnect,
and AT resync are out of scope for the application feature set. They MUST NOT be
implemented or documented as a required user workflow unless a later revision
to this specification defines that behavior.

REQ-TUI-009: After command completion or failure, the TUI MUST keep enough
command context visible for the user to identify which AT command produced the
displayed result.

REQ-TUI-010: Clearing the response pane MUST remove previously rendered
response content from the next TUI frame.

REQ-TUI-011: The Status area MUST be a compact status and context area. It
MUST NOT absorb large unused space that would be more useful for Response,
Logs, or command navigation. The Status area MUST be placed under Devices in
the top-band left utility column. Devices and Status MUST share the same
compact utility width as the bottom-band Controls pane. A full-width Status band
MUST NOT be used for the normal layout.

REQ-TUI-012: The Response pane MUST prioritize the actual current response body.
It MUST NOT duplicate enough command metadata to push the response body out of
the normal visible area when compact Status context already identifies the
active or selected command.

REQ-TUI-013: The Response pane MUST normalize modem line terminators and remove
terminal-affecting control sequences before rendering text in the TUI. Raw or
unmasked display means unmasked response values, not raw terminal control
bytes. A modem response MUST NOT be able to overwrite pane borders, prior
rendered text, or adjacent panes through carriage returns, ANSI escape
sequences, or other control characters.

REQ-TUI-013A: When a command or Sequence fails before a normal modem response
transcript can be produced, the TUI Response body MUST begin with a non-color
execution-result section `Result: failed`, followed by one blank line and the
existing failure text. The failure text SHOULD preserve the specific command or
Sequence failure reason. The TUI must not rely on compact Status or color alone
to communicate this failed before normal response state.

REQ-TUI-014: The compact Status area MUST show the current state, active or
selected command or Sequence name, AT command string when relevant, current
Sequence step when relevant, risk level, output masking state only when
meaningful, and completion or failure summary. It MUST avoid repeating the full
response body, Sequence transcript, or log content. Status content SHOULD use
readable key-value lines instead of packing unrelated values into a single
pipe-delimited line when vertical space is available. Status MUST NOT contain
generic keyboard shortcut boilerplate such as `Keys: ...`; keyboard hints are a
navigation aid, not command or transport state.

REQ-TUI-014L: The TUI Status Content Gate MUST be applied before changing
compact Status wording. Status is current state and execution context only.
Verification for Status wording changes MUST include render-buffer coverage for
the affected state and negative assertions for non-state explanatory text such
as `Copy:`, `Keys:`, `Summary:`, `Evidence:`, and free-form `Detail:` error
text.

REQ-TUI-014K: The compact Status area MUST NOT contain normal operation
explanations, action semantics, confirmation rationale, help text, copy/save
behavior descriptions, derived Evidence/analysis notes, response bodies,
Sequence transcripts, implementation details, or other text that explains how a
different control works instead of reporting current state. Action availability
belongs in Controls rows or the relevant pane action menu. Action results belong
in nearby action-surface feedback. Longer explanations belong in confirmation
dialogs, Help, documentation, or a detail surface defined by this specification.
Modem output and derived analysis belong in Response or a specification-defined
detail surface.

REQ-TUI-014M: Compact Status MUST NOT render arbitrary execution-error strings
or implementation-provided free-form detail fields. In completed or failed
states, Status may render a concise result row such as `Result: OK 7ms` or
`Result: failed`. Full error text, Sequence expectation failures, modem output,
and troubleshooting detail MUST be rendered in Response or another
specification-defined detail surface, not as a `Detail:` row in Status.

REQ-TUI-014O: Compact Status MUST keep selected-item context, active execution
context, completed execution context, and viewed-log context visually distinct.
Before execution, selected items MUST use `Selected Command:` or
`Selected Sequence:` because no execution result is active yet. Once an
execution is being confirmed, running, completed, failed, or cancelled,
`Status:` owns the lifecycle state and the target row MUST use the neutral
target nouns `Command:`, `Sequence:`, or `Action:` instead of duplicating the
state with labels such as `Confirming Command:`, `Executing Command:`, or
`Executed Command:`. For one-shot command targets and candidate actions with an
underlying AT line, the literal AT command string MUST be shown with
`AT command:` when it fits without displacing higher-priority Status content.
Sequence targets MUST NOT show a single top-level `AT command:` row because
their step transcript in Response owns the per-step AT command lines. Status
MUST NOT label an active or completed execution with storage/origin terms such
as `Preset:` because those terms can be mistaken for the currently selected
Commands / Sequences row after selection has moved. When the Response body is
cleared, Status MUST continue to show the last execution context with neutral
target nouns while the Response pane owns the cleared-body message and cleared
timestamp. When a saved log is being viewed, Status MUST show concise log-view
state and log identity instead of selected-item or execution-result labels.
Because Status is a compact sidebar context surface, long target labels MAY be
clipped horizontally instead of wrapped when wrapping would displace
higher-priority state, terminal-time, result, risk, masking, or progress rows.
Full command names, command text, response bodies, and log content remain
available in the Commands / Sequences, Response, Logs, exported Response, or
session-log surfaces as applicable.

REQ-TUI-014N: When an execution has completed, failed, or been cancelled and
remains the active execution context, compact Status MUST include concise
terminal-time context immediately after the lifecycle state. The timestamp
label MUST identify the event owner with the same lifecycle verb family as
`Status:`: completed executions use `Completed:`, failed executions use
`Failed:`, and cancelled executions use `Cancelled:`. The normal compact Status
layout MUST keep the event label and full timestamp on one row, for example
`Completed: YYYY-MM-DDTHH:MM:SSZ`. Status MUST NOT use `Completed at:`,
`Failed at:`, or `Cancelled at:` as the normal compact Status timestamp label,
and MUST NOT combine the terminal-time label with the result row. In all cases,
the timestamp MUST use atctl's existing UTC timestamp display format matching
session/history timestamp semantics. Status MUST NOT use shortened dates, omit
the year, omit the time zone marker, or invent a separate local-time display
format for this context. Completed, failed, or cancelled active execution
Status rows MUST be ordered by the user's post-execution confirmation task:
state, terminal time, target identity, source when present, `AT command:` when
relevant and when it fits, concise result, risk, then output masking when
shown. If the normal compact Status width cannot fit the full terminal-time
row, the TUI MUST first preserve lifecycle state, terminal time, result, risk,
and masking by widening the Status content area or omitting lower-priority rows
such as `Source:` or `AT command:`. Label/value splitting of the terminal-time
row is only an exceptional degraded rendering for unsupported very narrow
layouts; it is not the normal Status grammar. The Response, Commands /
Sequences list, and persisted logs remain the detailed command surfaces.

REQ-TUI-014J: The compact Status area MUST NOT display a Sequence `summary`
field as normal active or selected context. Sequence summaries are explanatory
purpose text, not command or transport state. They belong in the Commands /
Sequences row, the `Run Sequence` modal, search matching, or a detail/help
surface defined by this specification. Completion or failure summary in Status means concise
execution result context such as status and duration, not the Sequence purpose
sentence.

REQ-TUI-014I: Running-command timeout progress MUST be presented as temporary
execution feedback inside Status, not as persistent selected-command metadata.
Running execution Status rows MUST be ordered by the user's in-progress
monitoring task: state, target identity, source when present, `AT command:`
when relevant, risk, output masking when meaningful, then the timeout progress
block. The same Status context MUST NOT also render a separate static
`Timeout: Ns` selected-item row, because timeout budget and elapsed/left
progress are represented by the progress block while the command is running.
When the Status pane has enough height, the running-progress block MUST be
visually separated from the stable command context with a muted separator line,
then show a compact elapsed/timeout/left-duration text line and the progress bar
on a separate line. The separate progress-bar line SHOULD use a visible
filled/unfilled shape cue in addition to semantic color so the indicator remains
recognizable in no-color or low-contrast terminals. The text line MUST avoid
appearing truncated in the normal compact Status width. The normal label SHOULD
include the `Timeout` noun so the user can tell what the time budget represents,
for example `Timeout 33/180s left 147s`. If the available width cannot fit that
label, the TUI MAY shorten the label in steps, first to `33/180s left 147s`,
then to `33/180s` only when necessary. The compact label SHOULD use `left`
rather than `remaining`. If height is constrained, the TUI MAY collapse this
block, but it MUST keep progress visible while the command is running.

REQ-TUI-014G: The TUI MUST provide short keyboard hints in a dedicated footer
or command-bar area at the bottom of the screen. The footer MUST be
context-sensitive to the focused pane or modal state, MUST stay to one terminal
row in the normal layout, and MUST omit lower-priority hints rather than wrap
or overflow into adjacent panes. The footer MUST prioritize basic navigation,
activation, search, help, and quit. It MUST NOT become a dense inventory of
secondary command shortcuts. The `?` help overlay remains the concise keyboard
operation reference.

REQ-TUI-014H: The `?` help overlay MUST behave as a modal overlay. While help
is visible, the TUI MUST NOT pass `Enter`, letter keys, device selection, command
execution, raw export, response save/copy/clear, or quit actions to the
underlying panes. `Esc`, `?`, and `q` MUST close the help overlay and return to
the previous TUI state without changing the selected command, selected device,
current input, or current response. Help content MUST stay focused on concise
keyboard operation and close instructions. It MUST NOT include pane-architecture
explanations, implementation rationale, abstract flow labels such as
`Primary flow`, or descriptive inventories of what the Controls, Devices, or
Logs panes are for.

REQ-TUI-014A: The Devices pane MUST show USB device candidates currently visible
through `libusb` and narrowed only by explicit selectors. If no matching device
is visible, it MUST say so. When no matching device is visible, device-dependent
actions such as preset execution and AT command input sending MUST be disabled.
Non-device actions such as help, quit, viewing existing logs, scrolling, and
copying already displayed Response text MAY remain available.

REQ-TUI-014A1: The TUI MUST provide parity with the CLI device-discovery
workflow. The normal Devices pane MUST use the same operation-target scope as
`atctl devices`, and the TUI MUST also provide an explicitly labeled full-USB
troubleshooting view equivalent to `atctl devices --all-usb`. The full-USB view
MUST be available in-app so a user does not have to leave the TUI only to
confirm whether an expected modem is visible through `libusb`.

REQ-TUI-014A2: The TUI full-USB troubleshooting view MUST preserve the
distinction between operation targets and non-target USB devices. It MUST NOT
silently mix all visible USB devices into the normal command-target list. Hubs,
LAN adapters, cameras, microphones, billboard devices, and other non-target
items MUST be labeled as diagnostic-only or not eligible for AT sending.
Device-dependent AT sending MUST remain blocked unless the selected item has an
eligible descriptor shape and has been explicitly selected by the user.

REQ-TUI-014A3: If the normal operation-target filter hides an expected modem,
the TUI recovery path MUST let the user open the full-USB troubleshooting view,
inspect visible USB descriptor identity and selector fields, and return to the
normal selection flow when a suitable target is available. CLI commands such as
`atctl devices --all-usb` and
`atctl inspect --bus <BUS> --address <ADDRESS>` remain valid diagnostics outside
the TUI.

REQ-TUI-014B: If exactly one matching device is visible at TUI startup, the TUI
SHOULD automatically select that device, show its selected-device detail
summary, and allow command execution without an extra selection step.

REQ-TUI-014C: If multiple matching devices are visible at TUI startup, the TUI
MUST NOT silently choose one as the active execution target. It MUST start with
no active execution device and present the Devices pane as an explicit
selection list. Device-dependent actions such as preset execution and AT
command input sending MUST be blocked until the user explicitly selects a
device.

REQ-TUI-014D: The device-selection flow SHOULD use the visible Devices
pane itself: normal focus navigation reaches Devices, `Up` and `Down` move the
highlighted candidate, and `Enter` selects the highlighted device. The Devices
pane MUST also provide an `Enter`-activated row for switching between operation
targets and the full-USB troubleshooting view so this workflow does not require
a dedicated global shortcut. Once a device is selected, the Devices pane MUST
show a selected-device detail summary
including USB manufacturer when readable, USB product when readable, VID, PID,
bus, and address. Normal TUI device display MUST NOT show any built-in device
label, profile label, compatibility label, or implementation-defined product name such as
`Known`, `[known]`, or `Profile hint`. Command execution MUST constrain the
selected target by VID, PID, bus, and address so a multi-device match does not
silently select the wrong device.

REQ-TUI-014E: The user MUST be able to reselect a visible device after running
commands. For example, selecting device A, running `modem-response`, selecting
device B, and running `modem-response` again MUST be a supported workflow.
Reselecting a device changes the active execution target for subsequent
commands but MUST NOT change the already displayed response history. Device
reselection MUST be disabled while a command is actively running.

REQ-TUI-014F: The TUI MUST provide a temporary command-timeout control for the
current session. The timeout control MUST be reachable through normal focus
navigation and `Enter` activation in the Controls pane. The value MUST be
visible in Status before execution and MUST be used for the next command
executions until changed or reset.

REQ-TUI-015: Normal focus cycling MUST NOT include informational-only Status.
Normal focus cycling MUST preserve a predictable visual order and MUST keep
Categories and Commands / Sequences as the primary workflow path. When device selection is
required because multiple visible devices exist and none has been selected,
Devices MUST be the active selection surface before command-focused panes can
send AT commands or run Sequences. After a device has been selected, normal workflow focus SHOULD
prioritize `Categories -> Commands / Sequences -> Controls -> Response -> Logs -> Devices`.
Controls and Devices MUST be reachable through normal focus navigation rather
than requiring dedicated global letter shortcuts.

REQ-TUI-015D: The normal TUI layout MUST be one canonical pane topology, not a
viewport-dependent rearrangement of pane roles or Tab order. The screen MUST be
split into aligned top and bottom bands. The top band MUST contain the compact
Devices/Status utility column, Categories, and Commands. The bottom band MUST
contain Controls, Response, and Logs. The horizontal divider between top and
bottom bands MUST align across the full screen so the lower edges of Devices /
Status, Categories, and Commands / Sequences share the same row. Categories and
Commands / Sequences MUST remain adjacent in the main executable-item selection
area. Response and Logs MUST remain the lower result/review area. The default
vertical allocation SHOULD use a stable balanced split, approximately 50% top
band and 50% bottom band, so Sequence transcripts and command output have
enough review space without changing pane topology or focus order. When the
usable terminal height leaves a single extra row after the footer is reserved,
that extra row SHOULD remain in the bottom result/review band. Controls MUST
keep the same compact left utility width as Devices/Status and MUST be reachable
immediately after Commands / Sequences in focus order because many Controls
actions operate on the selected command, selected Sequence, or the current
command session.

REQ-TUI-015E: The normal TUI layout MUST allocate horizontal space according to
content complexity. Devices, Status, and Controls are compact utility panes and
MUST NOT receive a broad percentage column that leaves large unused horizontal
space in the normal layout. Their shared left utility width SHOULD be a compact
fixed terminal-column width, roughly 30 to 34 columns when the terminal is wide
enough. Categories SHOULD use a compact fixed width, roughly 22 to 26 columns
when the terminal is wide enough. Commands / Sequences, Response, and Logs
SHOULD receive the remaining horizontal space because AT command rows,
Sequence summaries, command output, Sequence transcripts, and log entries are
more likely to be long. Size changes MAY adjust widths and heights, but they
MUST NOT change the pane topology or Tab order.

REQ-TUI-015A: Every focused list pane MUST be navigable even when its item
count exceeds the visible pane height. Devices, Categories, Commands, Controls,
and Logs MUST render a viewport based on the pane's current inner height, and
the highlighted or selected item MUST remain visible after `Up`, `Down`,
`PageUp`, `PageDown`, `Home`, or `End`. `PageUp` and `PageDown` MUST move by
the focused pane's visible row capacity rather than a fixed magic number.
Response MUST keep line-based scrolling for command output and opened masked
logs. Status remains informational and MUST NOT become a scroll target unless
this specification is revised to assign it direct actions.

REQ-TUI-015B: The Controls pane MUST be a focusable operation pane, not a
shortcut reference and not a catch-all menu for every convenience action. It
MUST provide `Enter`-activated rows for execution and session controls that
belong to the current command session: AT command input, edit selected command
or Sequence inputs, set command timeout, start or stop raw diagnostic export
capture, and output masking. Response-specific actions such as copy, save, open
the Response output folder, and clear MUST belong to the Response pane action
menu when Response is showing an execution result. Opened-log actions such as
copy displayed log, reveal the opened log in Finder, and close the opened log
view MUST belong to the Response pane action menu when Response is showing a
saved log. Opening a selected log in Response MUST belong to the Logs pane
action menu. Controls MUST NOT provide
`rerun last`; repeated execution should happen from the selected
command/Sequence itself so the visible selection and executed item remain
aligned. Controls rows MUST use a stable list order and SHOULD remain visible
even when the action is currently unavailable. Controls rows MUST primarily
read as actions, not as a status table. Stable action labels MUST NOT be
replaced by result sentences such as `Copy resp sent`. Inline state MAY be
included only when it changes the action decision itself, such as `Timeout 30s`,
`Start raw export`, `Stop raw export`, or `Output masking off`. Routine
availability values such as `avail`, `no resp`, `ready`, or `sel dev` MUST NOT
be repeated on every row as a permanent state column. Activating an unavailable
row MUST leave the TUI state unchanged and provide concise nearby Controls
feedback that explains the reason. Controls MUST NOT reduce Categories,
Commands / Sequences, Response, or Logs into secondary surfaces.

REQ-TUI-015B1: Immediate action feedback MUST appear in the surface that keeps
the user oriented after the action. Controls actions MUST provide visible
feedback near the Controls action list. Action-menu rows MUST keep stable
labels and MUST NOT turn selectable rows into past-tense result labels.
One-shot Response and Logs action-menu commands MAY close the menu after
selection; when they close the menu, the compact Status area MAY show a concise
operation result such as `Exported response: <file>.`,
`Response body cleared.`, or a copy/reveal request result. Compact Status MUST
NOT become the only place that
explains a changed Response body state, and it MUST NOT become the place for
long exported-file paths. Target identity belongs in the action-menu context for
the specific selected, opened, or exported file.

REQ-TUI-015B2: The TUI Logs pane MUST represent the current saved history and
session log list for the running TUI session. It MUST read from the same
XDG state paths used by log writers and `atctl logs list`. After a TUI command
or Sequence execution finishes and normal
history/session log writing has completed, the TUI MUST refresh the Logs list
in the same session so a newly created `.session.log` can be discovered without
restarting the TUI. It MUST also refresh the Logs list before presenting the
Logs action menu, so external filesystem changes are reflected before the user
chooses a log action. If opening the
selected saved log fails because the file no longer exists, the TUI MUST keep
the read failure visible in Response, MUST identify the originally selected log
instead of substituting another row, MUST NOT enter viewed-log state, and MUST
refresh the Logs list immediately afterward so the deleted file no longer
remains selectable. The aggregate
`history.jsonl` MAY remain a single row even when it receives a new appended
history line. A log-list refresh failure MUST belong to the Logs pane, MUST
leave the current execution Status and Response intact, and MUST NOT replace
the execution result with a log-refresh result.

REQ-TUI-015C: Global letter shortcuts MUST be limited to the small primary set:
`/` for command search, `?` for help, and `q` for quit. Secondary operations
such as AT command input, edit, timeout, raw export, output masking, response
save/copy/clear, Response folder opening, opened-log close/copy/Finder reveal,
log opening, device reselection, and full-USB view switching MUST be
available through their relevant focusable panes and `Enter` activation rather
than requiring dedicated global letter shortcuts. `Enter` MUST NOT fall back to
command execution from Response, Logs, Status, or any other non-command pane.

REQ-TUI-CAT-001: The Categories pane MUST contain workflow categories only.
Preset set title, Sequence set title, vendor, provider, file path,
file-origin, or implementation labels MUST NOT be shown as Categories values
unless the preset or Sequence author explicitly writes the same value as a
category.

REQ-TUI-CAT-002: Category values MUST come from preset or Sequence `categories`
values such as `basic`, `identity`, `sim`, `network`, `pdp`, `apn`, `data`,
`signal`, `sms`, `diagnostics`, and `modem`. Preset or Sequence set metadata
  such as `Product presets`, `Product Sequences`, `Quectel commands`,
`Quectel Sequences`, `SORACOM commands`, a file path, `quectel`, or `soracom`
MUST NOT be generated as a category value by the loader or TUI.

REQ-TUI-CAT-003: The Categories pane MUST keep the same meaning regardless of
whether file presets or Sequence definition files are loaded. Adding file
presets or Sequences MUST NOT rename Categories to another concept or cause
preset set or Sequence set labels to appear in the category list.

REQ-TUI-GRAMMAR-001: TUI pane structure, list grouping, group headers, status
rows, controls rows, labels, and wording MUST be designed from a whole-surface
UI grammar, not as isolated conditional tweaks. The grammar for the affected
pane or workflow MUST identify each visible hierarchy level, what concept it
represents, when it is shown or hidden, and which user-facing labels are
allowed.

REQ-TUI-GRAMMAR-002: TUI executable-item grouping changes MUST be checked across
representative adjacent states before they are implemented or documented as
complete. Relevant states include product-only commands, mixed product/file
presets, mixed command/Sequence results, and loaded repository-managed examples.

REQ-TUI-GRAMMAR-003: Separate local requirements MUST NOT be satisfied by
producing inconsistent visual hierarchy across adjacent states. A group header
position or style MUST NOT switch between unrelated meanings such as item kind
and source set unless a stable parent hierarchy makes that distinction clear.

REQ-TUI-GRAMMAR-004: If the current specification contains local display rules
but lacks a combined grammar, or if the combined whole-surface grammar is
contradictory, the affected UI change MUST NOT be implemented until this
specification defines a consistent whole-surface grammar.

REQ-TUI-GRAMMAR-005: Changes to product-facing UI, wording, grouping, defaults,
add-ons, presets, Sequences, source labels, or execution behavior MUST remain
aligned with this specification and MUST include whole-surface, cross-surface,
and cross-state verification where those relationships are affected.

TUI executable-item display basis:

- The primary user task in the `Commands / Sequences` pane is selecting an
  executable item within the current workflow category.
- The primary list row information is item name, effective risk label, and the
  AT command string or concise Sequence summary.
- Default product-provided items are the baseline path and MUST NOT carry
  default-only source metadata in the normal list.
- Non-default file or repository-managed items MAY show their user-facing
  definition title when that source distinction helps the user understand why
  carrier-, vendor-, modem-, project-, or user-authored items are visible.
- Source metadata is a secondary distinction. It MUST NOT be represented with
  unspecified prefixes such as `Add-on:` in normal TUI labels.

REQ-TUI-CMD-SET-001: When the currently visible executable-item result set contains
one or more file presets, the Commands / Sequences pane MUST show non-default
file preset source groups using non-selectable group headers. Product command
rows MUST remain unheaded default rows in the normal list.

REQ-TUI-CMD-SET-002: File preset source group headers MUST use the user-facing
top-level TOML `title`, not raw file paths, internal identifiers, source keys,
or unspecified prefixes. Required label shape is:

```text
<file preset title from top-level TOML title>
```

REQ-TUI-CMD-SET-003: Command rows MUST keep the same shape regardless of
preset set: command name, one risk label, and AT command string. Command rows MUST
NOT use inline preset set badges such as `[built-in]`,
`[Quectel commands]`, or a file-path badge in the normal TUI command list.

REQ-TUI-CMD-SET-004: Source group headers are not commands. They MUST NOT be
focus targets, selection targets, execution targets, or count as command rows
for `Up`, `Down`, `PageUp`, `PageDown`, `Home`, or `End` command navigation.

REQ-TUI-CMD-SET-005: In mixed preset set command lists, the Commands / Sequences
pane MUST add one blank separator row before each file preset source group
header that follows another row in the same executable-item kind group. It MUST
NOT add a blank row after a source group header. This keeps each header visually
close to the commands it labels while separating it from default rows or the
previous source group.

REQ-TUI-CMD-SET-006: Source group separator rows are not commands. They
MUST NOT be focus targets, selection targets, execution targets, or count as
command rows for `Up`, `Down`, `PageUp`, `PageDown`, `Home`, or `End` command
navigation.

REQ-TUI-CMD-SET-007: When the currently visible executable-item result set contains
only product presets, the Commands / Sequences pane MUST NOT add a `Product presets`
header or preset-set separator rows only to explain the default case.

REQ-TUI-CMD-SET-008: The Status area and confirmation dialogs MAY show
`Source: <title>` for file presets using the same user-facing title as
Commands / Sequences group headers. They MUST NOT show raw file paths, internal
identifiers, or `Add-on:` prefixes in normal TUI surfaces. Product-only normal
TUI MUST NOT show `Source: Product presets` or `Preset set: Product presets`.

REQ-TUI-CMD-ORDER-001: TUI command order MUST be deterministic and MUST NOT
depend on incidental filesystem enumeration order, command-line file option
order, or loader insertion order except where this specification explicitly
preserves author-controlled TOML entry order.

REQ-TUI-CMD-ORDER-002: Command rows and file preset source groups MUST be
ordered as:

```text
1. Product command rows in curated workflow order, without a source header
2. File preset source groups, sorted by user-facing preset set title
3. Runtime-only command rows, sorted after built-in and file presets if this
   specification later defines them as visible in the command list
```

REQ-TUI-CMD-ORDER-003: Product presets MUST be displayed in the curated
workflow order defined in section 17.4, not alphabetical order and not loader
insertion order.

REQ-TUI-CMD-ORDER-004: File preset commands MUST preserve the order of
`[[presets]]` entries within each TOML file/preset set. This gives preset
authors a simple, visible way to control command order.

REQ-TUI-CMD-ORDER-005: Filtering by category or search MUST preserve the same
relative command order and preset-set group ordering for the remaining visible
commands.

REQ-TUI-SEQ-001: Sequence support MUST NOT add another permanent pane to the
normal TUI layout. The normal topology remains Devices/Status, Categories, and
Commands / Sequences in the top band, and Controls, Response, and Logs in the
bottom band. Sequence-specific inputs MUST be collected through a modal dialog
or equivalent temporary focused surface after a Sequence is selected.

REQ-TUI-SEQ-002: The executable-item pane title SHOULD be `Commands / Sequences`
once Sequence support is implemented. This title is intentionally longer than
`Commands` because the pane can contain both one-shot AT command presets and
multi-step Sequences. The title MUST NOT be changed to a generic internal term
such as `Actions` or `Items` without a corresponding specification revision.

REQ-TUI-SEQ-003: In the default view where no Sequence rows are visible, the TUI
MUST NOT add explanatory headers only to describe the absence of Sequences. When
both one-shot command rows and Sequence rows are visible in the same filtered
result, the pane MUST distinguish them without relying on color. The required
distinction is non-selectable group headers:

```text
Commands
Sequences
```

These group headers are not executable rows and MUST follow the same
non-selectable navigation rules as source group headers.

REQ-TUI-SEQ-004: One-shot command rows MUST keep the existing command-row shape:
command name, one risk label, and AT command string. Sequence rows MUST show the
Sequence name, one effective risk label, and a concise human-readable summary or
purpose. A Sequence row MUST NOT try to show every step's AT command inline.
That Sequence summary MUST NOT be duplicated into the compact Status area as
normal active or selected context.

REQ-TUI-SEQ-005: Sequence source grouping MUST follow the same user-facing
principles as preset source grouping. Product-provided standard Sequences MUST
NOT add a default-only group header solely to explain the normal baseline case.
When user or repository-managed Sequence definitions are visible, the pane MUST
show Sequence source group headers using the definition file's user-facing
`title`. It MUST NOT show raw file paths, internal source keys, inline source
badges, or invented `Add-on:` prefixes in normal TUI rows.

REQ-TUI-SEQ-006: Selecting a Sequence with operator-supplied, defaulted, or
resolvable values MUST open a `Run Sequence` modal before Sequence execution.
The modal MUST show the Sequence name, effective risk, required values,
sensitive-input masking state, current value, value source, and the confirmation
requirement for the Sequence. It MUST NOT reduce the experience to bare
`required <empty>` rows when a value has a Sequence default, a known modem check,
a selectable prior/current result, a derived value, or an external prerequisite
that can be explained. It MUST NOT run a Sequence with unresolved required
values.

REQ-TUI-SEQ-006A: The `Run Sequence` modal MUST use a compact value-resolution
grammar for Sequence values:

```text
<label>: <current value or unresolved>  <value state>
  <resolution hint, only when helpful for the active row or narrow displays>
  <resolution options or candidate rows, only for the active resolvable value>
```

The allowed value states are:

- `user`: the operator enters the value directly.
- `default`: the value is prefilled from the Sequence definition and can be
  edited before execution.
- `modem`: the value is normally confirmed from modem state, such as active PDP
  context state.
- `select`: the value is selected from listed modem or prior Response results,
  such as an SMS storage index.
- `sequence`: the value is fixed by the Sequence definition, such as a
  provider endpoint shown in review.
- `derived`: the value is derived during execution, such as SMS reply recipient
  from `AT+CMGR` sender metadata.
- `external`: the value or prerequisite must be satisfied outside the modem,
  such as a SORACOM Beam group entry-point configuration.

REQ-TUI-SEQ-006A.1: A Sequence parameter MAY declare a product-known
`candidate` source in addition to `source`, `default`, and `hint`. A candidate
source defines how the product can turn explicitly obtained modem or Sequence
output into selectable values. Product-known candidate sources are:

- `sms-message`: extracts SMS storage indexes and message context from
  `AT+CMGL` / `sms-receive-check` output.
- `pdp-context`: extracts PDP context IDs from standard `AT+CGACT?` and
  `AT+CGDCONT?` output.

The candidate field is part of the Sequence definition contract used by
built-in Sequences, repository-managed examples, and user-authored add-on
Sequences. Product-known candidate sources are limited to behavior the product
can parse without locking the standard product surface to a vendor/provider
specific command family. Vendor-specific checks such as Quectel socket state
MAY be exposed as explicitly loaded add-on commands or Sequences, but MUST NOT
be added to product-standard candidate assistance unless this specification is
revised to expand the product contract.

REQ-TUI-SEQ-006A.2: A candidate source MUST define its candidate acquisition
actions as explicit product actions. The TUI MAY render those actions inside
the `Run Sequence` modal, but selecting an action MUST execute the
corresponding command or Sequence through the normal execution path, including
risk classification, confirmation when required, timeout, output masking,
Response transcript, history/session logging, and raw diagnostic export
behavior where applicable. Opening the modal MUST NOT execute candidate actions.

REQ-TUI-SEQ-006B: TUI Sequence input MUST keep the normal TUI topology and MUST
not add a permanent Sequence-only pane. The active modal MAY show only concise
resolution hints inline, but the user must be able to determine where each
required value comes from before execution. The modal footer MUST remain
keyboard-operation focused.

REQ-TUI-SEQ-006B.1: The `Run Sequence` modal MUST be specified and implemented
as a phase-based operation surface, not as an unbounded text transcript. The
supported phases are value input, candidate selection, pre-send review,
confirmation, and error feedback. In every phase, the visible content MUST make
the current phase, next required action, and current input state clear without
requiring the user to infer hidden continuation below the visible modal.

REQ-TUI-SEQ-006B.2: If modal content exceeds the available area, the
implementation MUST either keep all phase-critical controls visible by
compressing lower-priority detail, or provide an explicit scrollable content
region with visible scroll affordance and working scroll keys. It MUST NOT show
content that appears to continue below the modal when there is no corresponding
scroll interaction. Phase-critical controls include the active value row during
input, highlighted candidate during selection, and risk confirmation instruction
plus current confirmation input during confirmation.

REQ-TUI-SEQ-006C: A value whose state is `select`, or whose parameter declares
a product-known `candidate`, MUST NOT be treated as resolved by a hint line
alone. When candidate values are available from an explicitly executed, same
TUI session result, the `Run Sequence` modal MUST show a same-modal
candidate-selection affordance. For SMS read/reply by storage index, this means
the user can select from known `AT+CMGL` / `sms-receive-check` message rows,
including enough context such as index, status, sender, timestamp, and a masked
or unmasked body preview according to the current output-masking mode. The
normal TUI path MUST NOT require the user to leave the modal, inspect another
pane, memorize or copy a value, and type it back in. Manual entry MAY remain
available for empty, stale, incomplete, or externally supplied candidate sets.
If no candidates are available, the modal MUST show selectable candidate
actions when the product knows how to obtain them. If candidates are already
available and the product knows an acquisition action for that candidate
source, the modal MUST keep that action selectable as an explicit refresh/load
action. This lets the operator update stale same-session candidates without
leaving the modal while preserving the rule that no hidden modem I/O happens on
modal open. TUI verification for a candidate-backed Sequence value MUST include
render-buffer or equivalent coverage proving that available candidates are
shown in the modal, can be selected without leaving it, and still expose the
applicable explicit refresh/load action.

REQ-TUI-SEQ-006C.1: Opening a `Run Sequence` modal MUST NOT implicitly execute
AT commands, Sequences, TCP/socket operations, PDP checks, or other modem or
network I/O to obtain candidate values. Candidate acquisition must happen only
through the normal execution flow for the command or Sequence that obtains the
data, including the applicable risk classification, confirmation, timeout,
masking, and transcript behavior. This applies to SMS candidates, PDP context
IDs, TCP read data, provider endpoint checks, and any future `source=select`
value source.

REQ-TUI-SEQ-006C.2: TUI candidate reuse MUST be scoped to the current TUI
session. Candidate sets MUST NOT be loaded automatically from history files,
session logs, raw diagnostic exports, exported Response files, previous process
runs, or another TUI session. If a future feature imports candidates from logs
or saved output, that import must be an explicit product action with its own
review surface and must not be confused with live or same-session modem state.

REQ-TUI-SEQ-006C.3: A candidate set MUST carry user-visible provenance. The
modal MUST identify the source that produced the candidates, such as the last
`sms-receive-check` result or last direct `AT+CMGL` result, the acquisition
time or equivalent current-session acquisition marker, and the total candidate
count. The display MUST make clear that showing candidates does not perform a
new modem read. When a candidate set remains available after unrelated
commands, the modal MUST still describe it as the last matching candidate
source, not as the current modem state.

REQ-TUI-SEQ-006C.4: Candidate sets MUST be keyed by candidate source, not by a
single hardcoded SMS-only cache. The same TUI session may hold separate
candidate sets for SMS message selection and PDP context selection. A candidate
set MUST be updated only from an explicitly executed command or Sequence output
that the candidate source can parse. Unrelated results MUST NOT overwrite
candidate sets for a different candidate source.

REQ-TUI-SEQ-006D: For SMS storage-index selection, candidate rows SHOULD use
the compact shape:

```text
storage=<index>  <status>  <sender>  <timestamp>  <body preview>
```

The `<index>` value is the SMS storage location returned by the modem in
`+CMGL` output and later used unchanged as the `AT+CMGR=<index>` argument. The
implementation MUST NOT infer, normalize, or compensate for whether a modem
numbers SMS storage from 0, 1, or another scheme. UI range text such as
candidate-row pagination MUST be labeled separately from modem storage indexes
so operators do not confuse visible row numbers with the value sent back to the
modem.

The sender and body preview MUST follow the current foreground output-masking
mode. When output masking is enabled, sender and body preview are masked in the
candidate list. When output masking is disabled, supported decoded body text MAY
be shown in the foreground candidate list. Candidate rows MUST remain concise
and MUST NOT duplicate the full Response transcript. Candidate rows MUST keep
the index and status visible. If the terminal width is constrained, body
preview is the first field to truncate, and the selected candidate SHOULD be
shown with enough detail to choose safely. The modal MUST keep manual typing
available so an operator can supply a value not present in the current
candidate set.

REQ-TUI-SEQ-006D.1: For PDP context ID selection, candidate rows SHOULD keep
the context ID visible first and then show enough concise context to choose
safely, such as active/inactive state, PDP type, APN, or assigned address when
the executed command returns that material. The row MUST NOT present the
candidate set as current modem state unless the provenance states that it came
from the last explicitly executed matching check in the same session.

REQ-TUI-SEQ-006D.2: Vendor-specific values that the standard product cannot
parse generically, such as Quectel TCP socket connect ID state from
`AT+QISTATE`, MUST remain normal Sequence values unless a product-wide
candidate source is intentionally specified. The TUI MAY show defaults and
hints for those values, and add-on command packs MAY provide explicit status
commands, but the normal input contract MUST keep manual editing available and
must not pretend that a vendor-specific socket state is product-standard PDP or
SMS candidate assistance.

REQ-TUI-SEQ-006E: Candidate-selection keyboard behavior MUST be TUI-efficient
and must not require mouse interaction. When a selectable candidate list is
visible for the active value, `Up` and `Down` move through candidate rows and
any visible candidate actions, `Enter` selects the highlighted candidate into
the value or runs the highlighted candidate action, and repeated `Enter`
continues the existing Sequence input flow once the current value is already
resolved. `Tab` MAY move to the next value. Character input MUST remain a manual
entry path for the active value. The modal footer MUST describe the current
candidate-selection keys without adding normal-operation explanations to
compact Status.

REQ-TUI-SEQ-006E.1: A candidate list MAY render only a window of rows when the
modal cannot fit every candidate, but all candidates in the candidate set MUST
remain selectable. The visible window MUST follow the highlighted candidate, so
the selected candidate is always visible. The modal MUST show the visible range
and total count when not all candidates fit. The range label MUST identify
itself as candidate rows, not as modem storage indexes. A display such as `... N
more` without a visible selected-row window or range is insufficient.
Verification MUST include render-buffer or equivalent coverage that available
candidate-backed values are shown in the modal before the behavior is called
complete.

REQ-TUI-SEQ-006E.2: Candidate actions are explicit value-resolution actions and
MUST remain selectable for the active candidate-backed value when the product
knows how to obtain or refresh that candidate source. When no candidates are
loaded, `Up` and `Down` SHOULD move between the current value option and the
available candidate actions. When candidates are loaded, `Up` and `Down` SHOULD
move between candidate rows and the available candidate actions. `Enter` on the
current value option advances the normal Sequence input flow. `Enter` on a
candidate row selects that value. `Enter` on a candidate action runs that
action through the normal execution path and keeps the `Run Sequence` modal
available so parsed candidates can be selected after the action completes. If
the action requires typed risk confirmation, that confirmation happens inside
the same `Run Sequence` modal before USB access and MUST use the same risk word
as the normal command/Sequence confirmation policy.

REQ-TUI-SEQ-006E.3: Candidate acquisition action execution is part of Sequence
input assistance, not execution of the selected Sequence body. While a
candidate action is confirming, running, completed, or failed, compact Status
MUST identify the context as an `Action`, not as a visible command-list Preset
or as the selected Sequence. Compact Status MUST NOT expose runtime-only preset
names, runtime source labels, free-form error detail, or the selected
Sequence's failure state for candidate action failures. If a candidate action
fails before normal response output, Response MUST begin with `Result: failed`,
then one blank line, then the action failure detail. The `Run Sequence` modal
MUST remain available when possible so the operator can retry the candidate
action, manually enter the value, select another available value, or cancel
without misreading the selected Sequence itself as already failed.

REQ-TUI-SEQ-006F: `sms-reply-check` candidate selection is a convenience for
choosing the original storage index; it is not a cached send destination.
During execution, the Sequence MUST still read the selected index with
`AT+CMGR` and derive the SMS reply recipient from the returned sender. If the
selected message cannot be read or no sender is returned, the Sequence MUST stop
before submitting a reply body.

REQ-TUI-SEQ-006G: CLI and PTY bridge surfaces MUST NOT use hidden TUI candidate
sets. Non-interactive CLI Sequence execution requires explicit parameters or
documented defaults and must fail with actionable missing-value guidance when a
required candidate-backed or `source=select` value is absent. CLI list and
missing-value output MUST expose source/default/hint/candidate metadata
compactly when present so add-on Sequence authors and automation users can see
how a value is intended to be obtained. PTY bridge input is an AT execution
surface and must not consult TUI session candidates automatically. Any
cross-surface candidate sharing requires an explicit user action defined by
this specification.

REQ-TUI-SEQ-006H: During Sequence confirmation, the risk action instruction and
current confirmation input MUST remain visible at all supported modal sizes. For
confirmation-required Sequences, the modal MUST show `Type <risk> to run` and
`Input: <current input>` as phase-critical content. For Sequences that do not
require typed confirmation, the modal MUST show the Enter-to-run instruction.
Long `Values` or `Review` sections MUST be summarized, clipped, or placed in a
scrollable detail region before they are allowed to push the confirmation
instruction or input field out of view. The footer alone is not sufficient
confirmation input visibility.

REQ-TUI-SEQ-007: The `Run Sequence` modal MUST keep keyboard focus inside the
modal until the user confirms, cancels, or completes required input. `Esc`
MUST cancel the modal and return to the previous TUI state without sending AT
commands. The modal footer or help text MUST remain concise and
operation-focused.

REQ-TUI-SEQ-008: During Sequence execution, Status MUST show compact current
execution context such as Sequence name, current step number, step label,
timeout budget, and risk. Status MUST NOT become a scrollable transcript or a
step table.

REQ-TUI-SEQ-009: Response MUST be the primary place for Sequence execution
results. It MUST show a readable step transcript with each sent AT command or
payload action, masked response text, final status, and relevant URCs or prompt
events. It MAY show a concise outcome summary at the top, but the summary MUST
not replace the underlying transcript.

REQ-TUI-SEQ-010: Controls MUST remain a compact operation pane. It MUST NOT add
a permanent Sequence-only controls section. Controls actions such as timeout,
raw diagnostic export, and output masking MUST apply to Sequence execution
where semantically valid. Response actions such as save, copy, saved
Response folder opening, and clear MUST apply to Sequence output through the
Response action menu where semantically valid. The edit-before-run row MAY
change wording only when the selected executable item is a Sequence and the
action actually opens or edits Sequence inputs; it MUST NOT use misleading
one-shot command wording for Sequence parameters.

REQ-TUI-SEQ-011: Search MUST match both one-shot command rows and Sequence rows
by name, category, summary, and where appropriate underlying AT command tokens.
Search result ordering MUST preserve deterministic preset/Sequence ordering and
must not interleave rows by incidental loader order.

REQ-TUI-016: The TUI MUST provide an `AT command` input route for sending a
one-shot command that is not saved as a preset.

REQ-TUI-017: TUI AT command input execution MUST use the same transport, masking,
logging, and risk policy as preset execution.

REQ-TUI-018: TUI AT command input MUST allow valid AT command-line
characters such as quotes, commas, semicolons, equals signs, question marks,
and command parameters. Input validation MUST protect the execution model and
terminal rendering without rejecting ordinary AT command syntax.

REQ-TUI-019: TUI AT command input MUST classify the entered command before
USB access. Confirmation-required classifications MUST open the same risk
confirmation flow used by presets.

REQ-TUI-020: SMS send and other prompt-required multi-step commands MUST NOT be
treated as ordinary one-shot AT command input. They belong in Sequence execution
or an explicitly prompt-capable PTY bridge mode.

REQ-TUI-021: The TUI MUST provide a way to copy the current Response body
without copying pane borders, pane titles, Status content, Logs content, or
other surrounding UI chrome.

REQ-TUI-022: Response copy MUST be available from the Response pane action
menu, opened by focusing Response and pressing `Enter`. It MUST NOT require a
dedicated global letter shortcut and MUST NOT be placed in the Controls pane.

REQ-TUI-023: Response copy MUST copy the executed AT command and the displayed
response body for the current command response. If the modem response already
contains the command echo, the copied text MUST NOT duplicate the command line.

REQ-TUI-024: Response copy MUST follow the current visible masking state. In
the normal state it MUST copy masked response text. If TUI session output
masking is off and the current Response has unmasked foreground material,
Response copy MAY copy the same unmasked text that is visible on screen. Saved
responses, history, session logs, and raw diagnostic export behavior are not
changed by Response copy.

REQ-TUI-025: When the Response pane is temporarily showing a saved masked log,
Response copy MUST copy only the displayed masked log body. It MUST NOT include
line-number UI, visible-range text, pane borders, or raw values.

REQ-TUI-026: The TUI MAY use OSC 52 to request a terminal clipboard write. It
MUST NOT read the clipboard. Other clipboard mechanisms require corresponding
requirements in this specification.

REQ-TUI-026A: After a Response copy action, the TUI MUST provide visible
feedback. Because OSC 52 requests a terminal clipboard write and the TUI does
not read the clipboard, success feedback MUST be worded as a copy request sent
to the terminal, not as verified clipboard contents. If the terminal clipboard
request fails, feedback MUST show a copy-request failure. The selectable copy
row label MUST remain an action label that identifies its applicable masking
state as defined by REQ-TUI-026B; it MUST NOT be replaced by transient success
or failure text.

REQ-TUI-026B: The Response pane action menu MUST expose actions for the
currently displayed Response target. When Response is showing an execution
result, it MUST expose copy current Response, export current Response, reveal
the exact last exported file, and clear current Response when each operation is
applicable. When Response is showing an initial notice such as the external
definition loading notice, that displayed notice is also a valid current
Response target for copy/export actions. When the current displayed Response
has a distinct masked and unmasked representation, the action menu MUST identify
the displayed content as masked or unmasked. It MUST use `Copy response` and
`Export response...` for masked content, and `Copy unmasked response` and
`Export unmasked response...` for displayed unmasked content. The ellipsis MUST
show that destination input follows.

`Export response...` and `Export unmasked response...` MUST open a
destination-folder chooser on every invocation. Before the chooser opens, the
menu context MUST identify the Response, UTF-8 text format, generated file name,
and applicable masked or unmasked export state. Masked export MAY write after
the operator selects a destination folder. Unmasked export MUST NOT write after
folder selection alone: it MUST return to a dedicated TUI confirmation that
shows the exact final file path, warns that the file may contain unmasked
identifiers, messages, payloads, or credentials, and requires exact typed
acknowledgement `export`. `Esc` and `q` MUST cancel without creating a file.

Selecting `Copy unmasked response` MUST open a dedicated confirmation that
states the terminal clipboard request will contain the unmasked Response and
requires exact typed acknowledgement `copy`. `Esc` and `q` MUST cancel without
sending an OSC 52 request. Masked `Copy response` MAY send the request
immediately. Export MUST create a new file exclusively and MUST NOT
overwrite, append to, or silently rename an existing file. Export success
feedback MUST name the exact created file and retain that path as the exported
file associated with the currently displayed Response. A repeated export MUST
use a newly generated name and replace only the in-memory reveal association;
it MUST NOT delete an earlier export. Replacing or clearing the displayed
Response MUST clear the association without deleting the exported file.
Clearing a Response MUST clear only the displayed Response body, not the active
execution Status context, logs list, selected command, or masking setting.
After a user clears the Response body, the Response pane MUST render an
intentional cleared state, `Response body cleared.`, plus a
`Cleared: YYYY-MM-DDTHH:MM:SSZ` row using the same UTC timestamp semantics as
Status and logs. It MUST NOT render the same `No response.` empty state used
before any Response body is available.

REQ-TUI-026C: When Response is showing an opened saved log, the Response pane
action menu MUST expose opened-log actions: copy displayed log, reveal that
exact saved log in Finder, and close log view. It MUST NOT offer `Export response...`
for an already persisted log view, and it MUST NOT use `Clear response` to close
a log view. Close log view MUST remove the opened log body from Response and
clear the viewed-log state. The viewed-log state MUST retain the exact path that
was opened. The action menu MUST show the opened log type and file name once as
shared context so the operator can verify which `history.jsonl` or
`.session.log` file the Finder action targets without putting a long path in the
compact Status area.

REQ-TUI-026D: The Response pane action menu MUST keep action labels stable and
show unavailable reasons as nearby menu feedback when the menu remains open.
If no Response body is available, including after the user has cleared the
Response body, copy/export/clear MUST not execute. The menu MUST NOT expose a
fixed saved-Responses-folder action because each export destination is selected
by the operator and may differ.

REQ-TUI-026E: When the currently displayed Response has a successfully exported
file association, its action menu MUST expose `Reveal in Finder`. The shared
menu context MUST identify the Response as a command, Sequence, candidate
action, or initial notice; show completion time when available; state the
export masking mode and format; and name the exact exported file. On macOS,
reveal MUST
ask Finder to select that exact file without opening its contents. If the file
was removed after export, reveal MUST be disabled with a concise missing-file
reason and MUST NOT fall back to the containing folder or another exported file.
Successful process launch MUST be reported as a request sent, not as verified
Finder state. Export, destination selection, and reveal failures MUST name the
failed action. Copy-path and copy-directory actions MUST NOT be added as
substitutes for this visible target context.

### 16.1 TUI Visual Accessibility and Theme Requirements

TUI styling is product behavior, not incidental implementation detail. Color,
contrast, focus, selection, and risk styling MUST be specified before
implementation when they affect user-visible behavior.

Reference basis:

- W3C WCAG 2.2 Success Criterion 1.4.1: color must not be the only visual
  means of conveying information, indicating an action, prompting a response,
  or distinguishing a visual element.
- W3C WCAG 2.2 Success Criterion 1.4.3: normal text contrast target is at
  least 4.5:1, with documented exceptions for large text and incidental text.
- W3C WCAG 2.2 Success Criterion 1.4.11: visual information required to
  identify UI components and states should meet at least 3:1 contrast against
  adjacent colors.
- W3C WAI contrast guidance treats foreground-only or background-only color
  specification as insufficient for objective contrast evaluation because the
  user's default counterpart color is unknown.
- Ratatui named colors are ANSI terminal colors. Their actual appearance can
  depend on the user's terminal palette. Ratatui RGB colors require terminal
  true-color support and can be unreliable on unsupported terminals/backends.
- `NO_COLOR` indicates that users should be able to opt out of software-added
  ANSI color by default, while explicit user configuration or command-line
  options can override it.
- Nielsen Norman Group's visibility-of-system-status heuristic supports
  keeping users informed through appropriate state feedback.
- Apple's Human Interface Guidelines and WWDC design guidance describe
  progressive disclosure as showing necessary information first and revealing
  more detail when it becomes relevant.
- W3C WCAG2ICT describes how WCAG status-message concepts can apply to
  non-web software, including text and terminal interfaces.

REQ-TUI-A11Y-001: The TUI MUST NOT rely on color as the only indicator for
focus, selection, risk level, command category, confirmation state, masking
state, or error state.

REQ-TUI-A11Y-002: The TUI MUST preserve non-color affordances such as selection
markers, text labels, risk labels, borders, dialog text, and emphasis when color
is added, changed, disabled, or unavailable.

REQ-TUI-A11Y-003: TUI colors MUST be assigned through semantic roles, not
scattered raw color choices. Required roles include at least focus, selected,
status, muted text, safe risk, sensitive risk, write risk, dangerous risk,
warning, and error.

REQ-TUI-A11Y-004: The current cyan/yellow styling is the colored baseline for
the documented dark-terminal presentation. It MUST NOT be claimed as verified
light/dark support or WCAG contrast conformance.

REQ-TUI-A11Y-005: Dark and light theme support MUST NOT be marked complete
until separate palettes are specified for both themes and their relevant
foreground/background pairs are checked against the contrast targets in this
section.

REQ-TUI-A11Y-006: If a TUI theme specifies foreground color for normal text, it
MUST also specify, constrain, or otherwise define the background used for
contrast evaluation before objective accessibility conformance is claimed.

REQ-TUI-A11Y-007: ANSI named colors MAY be used when the product intentionally
follows the user's terminal palette, but ANSI named colors alone MUST NOT be
used as evidence that light and dark themes satisfy objective contrast targets.

REQ-TUI-A11Y-008: RGB colors MAY be used for specified palettes only after the
implementation accounts for terminal support and fallback behavior. RGB color
use MUST NOT be introduced as a hidden portability decision.

REQ-TUI-A11Y-009: The TUI SHOULD provide a color opt-out path, such as honoring
`NO_COLOR` or an explicit theme/config setting, while preserving non-color
affordances.

REQ-TUI-A11Y-010: Any change to TUI palette, emphasis, or visual state
representation MUST define the concrete replacement in this specification
before implementation.

REQ-TUI-A11Y-011: TUI risk levels MUST be visually distinguishable for at
least `safe`, `sensitive`, `write`, `persistent`, `dangerous`, and `unknown`.
The distinction MUST use semantic roles and MUST NOT rely on color alone.

REQ-TUI-A11Y-012: The TUI risk palette and emphasis design MUST preserve the
specified cyan/yellow direction unless this specification defines a concrete
replacement.

REQ-TUI-A11Y-013: TUI risk styling MUST keep text labels such as `[safe]`,
`[sensitive]`, and `[write]` visible even when colors are disabled or
unavailable.

OQ-012 risk-display contract:

- Commands and Sequences MUST display exactly one risk-classification label:
  - `safe` -> `[safe]`
  - `sensitive` -> `[sensitive]`
  - `write` -> `[write]`
  - `persistent` -> `[persistent]`
  - `dangerous` -> `[dangerous]`
  - `unknown` -> `[unknown]`
- The command or Sequence row MUST NOT append masking state, confirmation
  instruction, persistence description, severity restatement, or review
  instruction to the risk label. In particular, `MASKED`, `CONFIRM`, `PERSISTS`,
  `DANGER`, and `REVIEW` are not risk labels.
- Output masking is session and Response state. It MUST be shown separately as
  `Output masking: on` or `Output masking: off` in the applicable Controls,
  Status, and Response surfaces.
- Expected effect and exact acknowledgement instructions belong in the
  confirmation surface, not in the executable-item row or risk label.
- The specified dark palette is:
  - background `#263238`
  - base text `#ECEFF1`
  - focus/status/safe `#4DD0E1`
  - selection `#FFD54F`
  - sensitive `#D6B3FF`
  - write `#FFD166`
  - persistent `#FFB86C`
  - dangerous `#FF6B6B`
  - unknown `#B0BEC5`
- The specified light palette is:
  - background `#FAFAFA`
  - base text `#263238`
  - focus/status/safe `#007C89`
  - selection/write `#7A5A00`
  - sensitive `#6B3FA0`
  - persistent `#9A4D00`
  - dangerous `#B00020`
  - unknown `#4B5563`
- Risk styling MUST apply to Commands, Status, and Confirmation areas. Response
  content masking state and Response-action warnings MUST use their own state
  and warning roles rather than reusing a command-risk label.
- Selected rows MUST NOT erase risk-specific token styling.
- TUI theme selection MUST support default dark, `--theme dark`,
  `--theme light`, and `--theme no-color`.
- `NO_COLOR` without an explicit `--theme` MUST use no-color mode.
- Verification MUST cover theme selection, semantic risk-role mapping,
  selected-row risk-token preservation, all six exact labels, negative checks
  for the removed suffix words, and render-buffer evidence that masking state
  and confirmation instructions remain separate from risk labels.

### 16.2 TUI Output Masking Requirements

REQ-TUI-MASK-001: TUI command output MUST remain masked by default.

REQ-TUI-MASK-002: TUI unmasked foreground display MUST require explicit user
action and MUST NOT be enabled by simple command selection, focus movement,
response clearing, or ordinary command execution.

REQ-TUI-MASK-003: TUI unmasked foreground display MUST show a confirmation that
explains that sensitive modem, subscriber, or credential-like values may become
visible.

REQ-TUI-MASK-004: TUI unmasked foreground display applies to the current TUI
session and foreground Response/copy behavior only.

REQ-TUI-MASK-005: TUI unmasked foreground display persists until the user turns
masking on again or exits the TUI. It MUST NOT reset because of focus,
selection, response clearing, command completion, or timeout changes.

REQ-TUI-MASK-006: Saved history and session logs MUST remain masked while
foreground display is unmasked. Explicit Response export MUST follow the
visible Response masking state. Raw diagnostic export remains a separate
explicitly acknowledged operation.

REQ-TUI-MASK-007: TUI output masking state MUST be visible without relying on
color when it can change what the user sees or copies. The TUI MUST avoid
boilerplate state lines for safe responses whose displayed text is unchanged by
masking.

TUI session output masking contract:

- TUI output masking MUST be on by default.
- `atctl tui --no-mask` MUST start the TUI with output masking off for the
  foreground TUI session only.
- The TUI Controls pane MUST provide an `Output masking` row with an inline
  state such as `on` or `off`.
- `Output masking off` MUST use warning styling in Controls while retaining the
  visible `off` text; color MUST NOT be the only indicator.
- Turning output masking off from inside the TUI MUST require a confirmation
  dialog with exact typed acknowledgement `unmask`.
- Turning output masking on from inside the TUI MAY happen immediately.
- The confirmation dialog MUST explain that unmasked sensitive modem,
  subscriber, payload, message, credential, or TCP response values may become
  visible in the TUI Response display.
- The confirmation dialog MUST explain that Response copy and explicit Response
  export follow the visible Response display. It MUST also explain that
  unmasked copy requires `copy` and unmasked export requires `export`, while
  generated history and session logs remain masked and raw diagnostic export
  requires its own path and `raw-log` acknowledgement.
- `Esc` and `q` MUST cancel the output-masking confirmation dialog.
- Output masking off MUST persist until the user turns output masking on again
  or exits the TUI. It MUST survive focus changes, category changes, command
  selection changes, response clearing, and ordinary command execution during
  that TUI session.
- TUI state MAY keep unmasked response text in memory only as needed for the
  current foreground Response display, copy, and explicit export behavior.
- Unmasked response text MUST NOT be passed to normal log writers.
- Unmasked response text MUST NOT be written to config, state, history, session
  files, or any destination other than a Response export explicitly selected by
  the operator.
- When a saved masked log is opened in the Response pane, the displayed log
  remains masked even if TUI session output masking is off.
- Status and Response context MUST show `Output masking: off` when output
  masking is off and the Response surface can display or copy unmasked
  foreground values.
- Status and Response context MAY show `Output masking: on` only when masking is
  meaningful for the current Response, such as when masked and unmasked response
  text differ.
- Status MUST NOT add persistent copy-behavior explanation such as `Copy: ...`
  for the normal foreground Response. The copy action's availability and copy
  request results belong in the Response action menu, and the output-masking
  confirmation dialog or Help may explain that Response copy follows the
  visible Response display.
- Safe responses whose displayed text is not changed by masking MUST NOT show a
  response-local output-masking state line while output masking is on.
- Verification MUST cover masked/default behavior,
  `atctl tui --no-mask`, exact typed acknowledgement, cancel behavior, visible
  output-masking state, session persistence across focus/category/selection and
  command execution, immediate masked Response copy/export, exact `copy` and
  `export` acknowledgement for displayed unmasked content, confirmation
  mismatch and cancellation, no file before unmasked export acknowledgement,
  exclusive file creation, destination cancellation, saved-log masking, and raw
  diagnostic export separation.

## 17. AT Command Taxonomy and Presets

Commands are classified as:

1. Standard/common commands
2. Vendor-specific commands
3. Provider/convenience commands
4. User-defined commands

### 17.1 Risk Model

Risk levels:

```text
safe:
  Read-only or harmless command.

sensitive:
  Reads identifiers, credentials, or operationally sensitive information.

write:
  Changes runtime configuration.

persistent:
  Changes persistent modem configuration.

dangerous:
  May reset modem, detach from network, change bands, break connectivity, or
  make the device hard to recover.

unknown:
  Cannot be confidently classified and is treated as potentially unsafe.
```

REQ-RISK-001: Every preset command MUST have a risk level.

REQ-RISK-002: APN-changing commands MUST NOT run automatically.

REQ-RISK-003: APN-changing commands MAY exist as presets or templates only when
they require explicit user selection and confirmation.

REQ-RISK-004: Direct command risk classification MUST distinguish safe,
sensitive, write, persistent, dangerous, and unknown.

REQ-RISK-005: The direct command classifier MUST treat syntactic read/test
commands with unknown sensitivity as sensitive by default.

REQ-RISK-006: The direct command classifier MUST treat commands that cannot be
confidently classified as read/test as confirmation-required unknown commands.

REQ-RISK-007: File preset add-ons MUST declare a risk level for every preset.

REQ-RISK-008: User-declared risk MUST NOT be able to downgrade the risk
classified from the AT command string.

REQ-RISK-009: For presets loaded from TOML, the implementation MUST compute an
effective risk from the declared risk and the command classifier. The effective
risk MUST preserve the stricter enforcement outcome.

REQ-RISK-010: If the classifier identifies a command as sensitive, write,
persistent, dangerous, or unknown, the effective risk MUST preserve the masking
or confirmation behavior required by that classification even when the TOML
declares a lower risk.

REQ-RISK-011: Unknown read/test commands MAY execute without confirmation only
when treated as sensitive by default. Unknown commands that cannot be
confidently classified as read/test MUST be confirmation-required.

### 17.2 Preset Purpose

Presets exist to support practical modem workflows, not only passive
inspection. The default experience MUST help users inspect a modem, confirm SIM
and network state, configure or inspect APN/PDP state, and prepare for SMS
workflow validation while preserving the risk policy.

### 17.3 Preset Sets and File Presets

Preset kinds:

```text
Product presets:
  Standard workflow presets provided by the program.

File presets:
  TOML-defined presets loaded from explicit per-invocation --preset-file /
  --preset-dir locations.
```

REQ-PRESET-SET-001: CLI preset lists MUST show a preset set label for every
preset and a trailing source-path field for source review. Product preset rows
in CLI list output MUST use `Product presets` and source path `-`.
TUI preset lists MUST distinguish standard workflow product presets
from file presets through the Commands-pane source grouping and `Source:
<title>` detail rules in section 16. TUI preset lists MUST NOT show preset set
labels in the default product-only view.

REQ-PRESET-SET-002: Preset set distinctions, when shown in the TUI, MUST be
visible without color. Preset set distinction MUST NOT rely on color-only
styling, inline badges, or invented prefixes such as `Add-on:`.

REQ-PRESET-SET-003: Product presets MUST remain vendor-neutral where
practical and MUST be organized around user workflows.

REQ-PRESET-SET-004: Vendor-specific, modem-specific, and carrier-specific
commands MUST be separated from product presets into file presets unless this
specification is revised to define them as product presets.

REQ-PRESET-SET-005: Quectel and SORACOM preset TOML files MUST be
repository-managed file preset examples and MUST be created, loaded, and
verified through the same multi-file preset loading path.

Repository-managed file preset examples:

```text
examples/presets/quectel.toml
examples/presets/soracom.toml
```

These files are part of the maintained product and verification surface for
preset loading. They MUST NOT be treated as optional documentation-only examples.
Verification MUST load these files through the same multi-file TOML
loader used for file presets, such as by passing `--preset-dir
examples/presets`. A separate example-only parser path is not acceptable
verification.

### 17.4 Standard Workflow Core Presets

Built-in preset display order for the TUI `All` category MUST follow this
curated workflow order:

```text
modem-response
modem-info
manufacturer
model
firmware-revision
imei
sim-pin-status
imsi
radio-stack
radio-stack-capabilities
current-operator
available-operators
operator-format-numeric
operator-auto-selection
circuit-registration
gprs-registration
eps-registration
enable-circuit-registration-detail
enable-gprs-registration-detail
enable-eps-registration-detail
enable-eps-registration-cause
enable-eps-registration-extended
signal-quality
extended-signal-quality
extended-signal-capabilities
pdp-contexts
pdp-auth-settings
pdp-auth-capabilities
packet-attach
active-pdp-contexts
pdp-addresses
pdp-address-capabilities
pdp-connection-details
extended-error-report
error-reporting-status
enable-verbose-errors
modem-activity-status
sms-service-support
sms-format
sms-storage
modem-functionality
set-modem-minimum-functionality
set-modem-full-functionality
restart-modem
disable-command-echo
```

This order intentionally follows connection check, modem identity, SIM, radio
access selection, operator visibility, registration, signal, PDP/APN readiness,
failure diagnostics, SMS readiness, modem functionality status/control, and
then write-risk runtime control. It MUST NOT be replaced with alphabetical
order.

Basic:

```text
modem-response: AT [safe]
disable-command-echo: ATE0 [write]
```

Modem identity:

```text
modem-info: ATI [safe]
manufacturer: AT+CGMI [safe]
model: AT+CGMM [safe]
firmware-revision: AT+CGMR [safe]
imei: AT+CGSN [sensitive]
```

SIM identity and status:

```text
sim-pin-status: AT+CPIN? [safe]
imsi: AT+CIMI [sensitive]
```

Radio access, operator selection, and network registration:

```text
radio-stack: AT+WS46? [safe]
radio-stack-capabilities: AT+WS46=? [safe]
current-operator: AT+COPS? [safe]
available-operators: AT+COPS=? [safe, timeout_secs=180]
operator-format-numeric: AT+COPS=3,2 [write]
operator-auto-selection: AT+COPS=0 [write]
circuit-registration: AT+CREG? [safe]
gprs-registration: AT+CGREG? [safe]
eps-registration: AT+CEREG? [safe]
enable-circuit-registration-detail: AT+CREG=2 [write]
enable-gprs-registration-detail: AT+CGREG=2 [write]
enable-eps-registration-detail: AT+CEREG=2 [write]
enable-eps-registration-cause: AT+CEREG=3 [write]
enable-eps-registration-extended: AT+CEREG=5 [write]
```

Signal:

```text
signal-quality: AT+CSQ [safe]
extended-signal-quality: AT+CESQ [safe]
extended-signal-capabilities: AT+CESQ=? [safe]
```

PDP / APN:

```text
pdp-contexts: AT+CGDCONT? [safe]
pdp-auth-settings: AT+CGAUTH? [sensitive]
pdp-auth-capabilities: AT+CGAUTH=? [safe]
packet-attach: AT+CGATT? [safe]
active-pdp-contexts: AT+CGACT? [safe]
pdp-addresses: AT+CGPADDR [safe]
pdp-address-capabilities: AT+CGPADDR=? [safe]
pdp-connection-details: AT+CGCONTRDP [safe]
```

`available-operators` may take longer than ordinary read commands because it
can trigger an available-operator scan. SORACOM documents this command as
typically taking 2 to 3 minutes. The product preset SHOULD declare
`timeout_secs = 180`. CLI users MAY still override this, for example
`atctl preset run available-operators --timeout 240` when a local scan exceeds
the preset hint.

`radio-stack` and `radio-stack-capabilities` let a
human operator check the standard wireless data service stack selection without
using AT command input. `operator-format-numeric`,
`operator-auto-selection`, and the registration detail presets are standard AT
commands used during troubleshooting, but they change runtime reporting or
operator selection behavior; they MUST remain write-risk presets and require
confirmation before USB access.

`extended-signal-quality` and `extended-signal-capabilities` provide standard extended
signal quality checks alongside the basic `AT+CSQ` checkpoint. `pdp-auth-settings` and
`pdp-auth-capabilities` cover standard PDP authentication inspection for
APN/CHAP/PAP troubleshooting. `pdp-auth-settings` is sensitive because `+CGAUTH:`
responses can include APN authentication usernames and passwords. `pdp-addresses`
and `pdp-address-capabilities` provide standard PDP address inspection after
PDP activation.

Diagnostics:

```text
extended-error-report: AT+CEER [safe]
error-reporting-status: AT+CMEE? [safe]
enable-verbose-errors: AT+CMEE=2 [write]
modem-activity-status: AT+CPAS [safe]
```

`AT+CEER` is a standard diagnostic command for reading the extended report for
the last unsuccessful call setup, last call release, last unsuccessful GPRS
attach, last unsuccessful PDP context activation, last GPRS detach, or last PDP
context deactivation when the modem implements the report. `AT+CMEE?` reads
the current mobile termination error reporting mode. `AT+CMEE=2` enables
verbose `+CME ERROR` reporting and therefore changes command-session behavior;
it MUST remain a write-risk preset and require confirmation before USB access.
`AT+CPAS` reads the modem activity status, including ready, unavailable,
unknown, and asleep states.

SMS readiness:

```text
sms-service-support: AT+CSMS? [safe]
sms-format: AT+CMGF? [safe]
sms-storage: AT+CPMS? [safe]
```

Modem functionality and restart:

```text
modem-functionality: AT+CFUN? [safe]
set-modem-minimum-functionality: AT+CFUN=0 [dangerous]
set-modem-full-functionality: AT+CFUN=1 [dangerous]
restart-modem: AT+CFUN=1,1 [dangerous]
```

`AT+CFUN?` is a standard read command for the modem functionality level.
`AT+CFUN=0`, `AT+CFUN=1`, and `AT+CFUN=1,1` change modem functionality or
restart the modem. They are standard UE AT commands, but they are not diagnostic
reads. They MUST remain dangerous presets, require typed confirmation, and be
documented as commands that may detach from the network, restart registration,
drop the current USB/AT session, or require the user to reconnect and reselect
the modem.

APN setting and SMS sending are valid product workflows, but commands that
change modem state or require prompt/body interaction MUST remain behind the
risk and Sequence policies. Product presets MAY include read-only readiness
checks. Carrier APN values and vendor-specific SMS helpers belong in file
presets or Sequence definition files unless explicitly promoted to
product-provided standard Sequences.

Standard SMS send/read/reply checks SHOULD be exposed as product-provided
standard Sequences. User-authored Sequence definitions are for additional,
special, project-local, or verification workflows; they MUST NOT be the
required first step for ordinary standard SMS checks. Vendor-specific
data-send checks, such as Quectel TCP/IP socket checks, belong in
repository-managed example Sequence definitions unless this specification is
revised to define a vendor-independent product behavior.

### 17.5 Repository-Managed File Preset Examples

The repository-managed file preset examples MUST demonstrate the same TOML
format used for user file presets and MUST be suitable for verification of
multi-file loading, preset set display, duplicate detection, and effective-risk
handling.

Quectel file preset requirements:

- MUST include Quectel-specific commands that were previously embedded in
  product presets, including `AT+QCCID`, `AT+QCSQ`, `AT+QNWINFO`,
  `AT+QENG="servingcell"`, `AT+QENG="neighbourcell"`, and `AT+QCFG?`.
- MUST include Quectel-specific diagnostic commands adopted for the Quectel
  preset set: `AT+QINISTAT`, `AT+QPINC?`, `AT+QSPN`, `AT+QLTS`, and
  `AT+QMBNCFG="List"`.
- MUST include Quectel-specific network scan mode diagnostics adopted for the
  Quectel preset set: `AT+QCFG="nwscanmode"` and the recovery preset
  `AT+QCFG="nwscanmode",0,1`.
- MAY include Quectel-specific modem control commands such as `AT+QPOWD` only
  as dangerous file presets. `AT+QPOWD` MUST NOT be promoted to a built-in
  preset because it is vendor-specific.
- `AT+QMBNCFG="List"` MAY be included only as a read-style diagnostic file
  preset. Other `AT+QMBNCFG` operations that select, deactivate, add, delete,
  or auto-select MBN files MUST NOT be added as ordinary safe presets.
- Quectel configuration-changing commands such as `AT+QCFG=...` MUST remain
  protected by persistent, dangerous, or unknown-risk handling unless this
  specification defines a specific safe read-only form.
- MUST declare `title = "Quectel commands"` at file level.
- MAY declare a file-level `description`.
- MUST categorize entries with relevant workflow categories such as `sim`,
  `signal`, `network`, `diagnostics`, and `modem`.
- MUST NOT use `quectel` as a workflow category. Quectel identity is the
  preset set title, not a category.
- MUST declare risk for every entry.

SORACOM file preset requirements:

- MUST include the SORACOM APN setting command
  `AT+CGDCONT=1,"IP","soracom.io"` as a confirmation-required write preset or
  template.
- MUST include SORACOM APN setting presets for the documented APNs
  `soracom.io`, `du.soracom.io`, and `m-airsim.jp`.
- MAY include SORACOM default PAP/CHAP authentication templates using
  `AT+CGAUTH=...` only when command and response masking covers authentication
  usernames and passwords.
- MUST declare `title = "SORACOM commands"` at file level.
- MAY declare a file-level `description`.
- MUST categorize entries with relevant workflow categories such as `apn` and
  `pdp`.
- MUST NOT use `soracom` as a workflow category. SORACOM identity is the
  preset set title, not a category.
- MUST declare risk for every entry.
- MUST NOT cause APN-changing commands to run automatically.

### 17.6 File Preset TOML

File presets are configuration, not source code. The TOML format MUST remain
human-editable and explicit.

TOML shape:

```toml
title = "Custom commands"
description = "Optional description for humans."

[[presets]]
name = "custom-modem-response"
command = "AT"
risk = "safe"
categories = ["custom"]
# Optional. Use only for known long-running commands.
timeout_secs = 180
```

REQ-PRESET-FILE-001: Each file preset TOML file MUST include top-level
`title`. The title is the normal user-facing preset set label in CLI output
and the non-default source title in TUI group headers and `Source: <title>`
detail. File names MUST NOT be used as the normal display name.

REQ-PRESET-FILE-002: Each `[[presets]]` entry MUST include `name`, `command`,
and `risk`.

REQ-PRESET-FILE-003: Each `[[presets]]` entry MAY include `categories`.

REQ-PRESET-FILE-004: Each `[[presets]]` entry MAY include `timeout_secs` for known
long-running commands. This value is a preset-specific execution hint, not a
global timeout setting.

REQ-PRESET-FILE-005: File preset locations MUST be supplied for each invocation
through `--preset-file` or `--preset-dir`.

REQ-PRESET-FILE-006: File preset loading MUST reject invalid TOML with a file
path and actionable parse error.

REQ-PRESET-FILE-007: File preset loading MUST reject duplicate names across
product presets and all loaded file presets.

REQ-PRESET-FILE-008: File preset loading MUST NOT create, overwrite, or
silently repair preset files.

REQ-PRESET-FILE-009: File preset entries MUST display the file-level `title`
as their preset set label in CLI listings. In the TUI, file preset identity
MUST be shown through source group headers or `Source: <title>` detail labels
when the currently visible executable-item result set contains file presets.

REQ-PRESET-FILE-010: File preset execution MUST use effective risk, not only
the TOML-declared risk.

REQ-PRESET-FILE-011: User-defined vendor commands MUST be safe to load even
when the command family is unknown. Safety comes from conservative
classification, masking, confirmation, and explicit preset set identity rather
than from rejecting ordinary AT command syntax.

Explicit file preset location overrides:

- `--preset-file <FILE>` loads one preset TOML file.
- `--preset-dir <DIR>` loads `.toml` regular files from one directory in
  deterministic lexicographic path order.
- Both options MAY be repeatable. Explicit files are loaded in command-line
  order, then explicit directories are loaded in command-line order. Files
  inside each directory are loaded in deterministic lexicographic path order.
- Loading order defines duplicate detection and file ingestion order. It MUST
  NOT by itself define TUI display order; TUI display order is defined by
  section 16 command-order requirements.
- Duplicate names across product presets and all explicit file presets MUST
  fail with the same duplicate-preset error used for file preset loading.
- Repository-managed file presets and project-local file presets SHOULD be
  reviewable through these explicit location options without copying files into
  a default auto-load directory.

### 17.7 Sequences

Sequences are named multi-step AT operations. They are separate from one-shot
presets because they may require prompt waits, payload writes, delayed URCs,
per-step timeouts, and result interpretation that cannot fit into one AT
command string.

Sequence kinds:

```text
Product-provided standard Sequences:
  Standard multi-step actions provided by the program.

Repository-managed example Sequences:
  TOML-defined example Sequences shipped in the source repository and loaded
  explicitly for vendor, carrier, or hardware-specific checks.

User-authored Sequences:
  TOML-defined Sequences loaded from explicit per-invocation --sequence-file /
  --sequence-dir locations.
```

Product-provided standard Sequence targets:

```text
sms-send-check
sms-receive-check
sms-read-message
sms-reply-check
```

`sms-send-check` SHOULD use standard 3GPP TS 27.005 SMS commands. It MUST
collect the recipient and message body as parameters, treat both as sensitive,
show the destination and message body in the pre-send review surface before USB
access, require write-risk confirmation, and record in the Response transcript
that `+CMGS` plus `OK` is submit evidence rather than destination handset
receipt evidence.

`sms-receive-check` SHOULD use standard 3GPP TS 27.005 SMS receive/list/read
commands. It MUST treat sender numbers and message bodies as sensitive. If the
chosen receive method changes message status, notification routing, storage
state, or unread/read flags, the Sequence MUST be classified as write-risk or
otherwise confirmation-required. The Sequence MUST NOT silently delete messages.
The Response transcript MUST distinguish modem message status values such as
`REC READ` and `REC UNREAD` from the product action used to read or list message
content. The product MUST parse listed message metadata and MUST decode body
content when the supported encoding path is recognized. For UCS2 hexadecimal SMS
text, the product MUST decode as UTF-16BE before masking. Normal output MUST
mask the decoded body. Unmasked foreground display MUST show the decoded body
when decoding is supported. Decoded body lines MUST appear in `Decoded SMS:`,
not as modem-originated `Modem response:` lines. Acknowledged raw diagnostic
export remains a raw modem exchange export and may contain the modem-returned
encoded body bytes.
Encoded hex MUST NOT be treated as a human-readable substitute for decoded body
content in normal or unmasked foreground transcripts.

`sms-read-message` SHOULD read a specific message by storage index using
standard 3GPP TS 27.005 commands. Because `AT+CMGR` can change an unread
message to read state, the Sequence MUST be write-risk and MUST require typed
confirmation before USB access. The pre-send review surface MUST show the target
SMS storage index before sending. The SMS storage index MUST be documented and
displayed as a value obtained from SMS storage listing output such as
`sms-receive-check` / `AT+CMGL`, not as a self-evident user-created number.
In TUI, selectable index candidates MAY be reused only from the current TUI
session's explicitly executed SMS listing result. Opening the read-message
modal MUST NOT issue a hidden `AT+CMGL` or `AT+CMGR`. If no same-session
candidate set exists, the TUI must say which product action obtains one while
keeping manual index entry available.
Sender and body values returned by the modem MUST remain masked in normal
Response, history, and session logs unless the user explicitly disables
foreground output masking for the display surface or creates acknowledged raw
diagnostic export. `sms-read-message` MUST include decoded-body analysis when
decoding is supported and MUST label undecodable bodies instead of treating
encoded hex as human-readable content.

`sms-reply-check` SHOULD reply to one received SMS storage index. It MUST collect
the SMS storage index and reply body, show both in the pre-send review surface
before USB access, read the original message with `AT+CMGR=<index>`, extract the
sender from the returned SMS metadata, and then submit the reply body to that
sender using the standard `AT+CMGS` submit path. The Sequence MUST NOT describe
manual recipient entry as SMS reply. The transcript MUST show that the reply
recipient was derived from the original message sender while keeping the sender
masked in normal output. `+CMGS` plus `OK` remains submit evidence rather than
destination handset receipt evidence. Full 3GPP reply-path compliance MUST NOT
be claimed unless the product explicitly handles the relevant
TP-Originating-Address, RP-Originating-Address, and TP-Reply-Path behavior.
The pre-send input surface MUST make clear that the operator supplies or selects
the SMS storage index and reply body, while the reply destination is derived from
the selected SMS sender during execution.
If the TUI offers candidate rows for `sms-reply-check`, the rows choose only the
storage index. Candidate sender values shown for recognition MUST NOT be reused
as the submit destination.

Repository-managed example Sequence targets:

```text
examples/sequences/quectel.toml
examples/sequences/soracom.toml
```

The Quectel example Sequence file SHOULD include at least one TCP/IP data-send
check and one ICMP ping reachability check based on the Quectel TCP/IP AT
command set. The TCP data-send Sequence SHOULD use
commands such as `AT+QIACT?` or `AT+QIACT=<cid>` for context activation checks,
`AT+QIOPEN=...` for socket opening, `AT+QISEND=...` for payload sending,
`AT+QISEND=<connectID>,0` for sent/acknowledged/unacknowledged byte counters,
`AT+QIRD=...` for reading an echo or response when available, and
`AT+QICLOSE=...` for explicit socket close. The exact command forms MUST match
the selected Quectel manual. `OK` after
`QIOPEN` MUST NOT be treated as socket-open success by itself when the command
also reports success or failure through a later `+QIOPEN` URC. `SEND OK` MUST
be reported as module-accepted-payload evidence, not remote application receipt
evidence. `+QISEND:` counters MAY be reported as TCP peer acknowledgement
evidence when the selected Quectel manual defines the fields and the transcript
includes sent, acknowledged, and unacknowledged byte counts. They MUST NOT be
reported as application-layer processing success. For fixed-length Quectel
`AT+QISEND=<connectID>,<length>` payload entry, example Sequences MUST send
exactly the declared payload bytes and MUST NOT append SMS-style Ctrl-Z unless
the selected command form explicitly requires it. A TCP acknowledgement query
step SHOULD declare `require_tcp_ack = true`; in that case the engine MUST
retry the query within the step timeout until the acknowledged byte count covers
the payload length and the unacknowledged byte count is zero, or fail the
Sequence with the last counters visible in the transcript. `+QIRD: 0` MUST be
reported as no received data. Non-empty `+QIRD:<length>` output MAY be reported
as response data evidence, but returned data remains subject to normal masking
and MUST NOT be treated as proof that a remote application committed the
payload unless the endpoint protocol defines that response as such.
The TCP Sequence input contract MUST distinguish user-entered destination and
payload values from modem-dependent PDP context ID and socket connect ID
values. Repository-managed Quectel and SORACOM TCP add-on Sequence definitions
SHOULD declare `candidate = "pdp-context"` for PDP context ID parameters when
the standard `AT+CGACT?` / `AT+CGDCONT?` parser is useful. Quectel socket
connect ID is vendor-specific TCP/IP state and MUST NOT be exposed through
product-standard candidate assistance merely for add-on convenience. It SHOULD
have an editable default plus a hint that points to an explicitly loaded
Quectel socket-state command when the operator needs to inspect current
sockets. Read length SHOULD have an editable default rather than requiring
routine manual entry.
The generic Quectel ping Sequence SHOULD use `AT+QPING=<contextID>,...` through
the selected PDP context. It MUST expose the destination host as a user-entered
value, use editable defaults for ping timeout and count, and report successful
received replies as IP reachability evidence only. It MUST NOT present ping
success as TCP socket, payload delivery, Unified Endpoint, Beam, or remote
application processing proof. A ping step SHOULD declare
`require_ping_success = true`; in that case the engine MUST fail the Sequence
when the modem returns terminal `OK` without a parsed successful `+QPING:`
reply or summary with at least one received reply. Because Quectel `AT+QPING`
accepts the command with `OK` and returns ping result lines afterwards, a ping
step that declares `require_ping_success = true` MUST use
`expect_urc = "+QPING:"` or an equivalent `+QPING:` wait marker. It MUST NOT use
only `expect = "OK"` as the step completion condition.

The SORACOM example Sequence file SHOULD include a provider-specific SORACOM
network reachability check and a provider-specific Unified Endpoint TCP example
whose endpoint, port, expected response behavior, and remote evidence rules come
from current SORACOM documentation. SORACOM identity belongs in the Sequence set
title and documentation context, not in workflow categories. A SORACOM Sequence
may use Quectel TCP/IP AT commands as the modem execution backend, but the
provider endpoint and expected evidence MUST remain separate from the generic
Quectel socket or ping example.

For SORACOM ping examples, the default documented destination is
`pong.soracom.io`. The Sequence MUST report received replies as SORACOM network
reachability evidence only. It MUST NOT describe ping success as TCP socket,
Unified Endpoint, Beam, or destination application receipt proof.

For SORACOM Unified Endpoint TCP examples, the default documented endpoint is
`unified.soracom.io:23080` or the `uni.soracom.io` alias. The Sequence MUST
show that endpoint and payload in the pre-send review surface. Because Unified
Endpoint may forward data to Beam, Funnel, Funk, Harvest Data, or other enabled
destinations, product output MUST distinguish local modem/socket evidence from
remote endpoint evidence. If no `QIRD` response data or external SORACOM
destination log is available, the Sequence MUST NOT describe the run as
end-to-end application receipt.

SORACOM Beam is not a default repository-managed basic connectivity check
because a Beam TCP/TCPs entry point depends on SIM group configuration. A Beam
example MAY be added when it is documented as Beam-specific. It MUST NOT replace
the ping and Unified Endpoint checks as the basic SORACOM sequence set.

For SORACOM TCP -> HTTP/HTTPS or Unified Endpoint forwarding, TCP byte streams
may be split or combined before reaching an HTTP destination. If preserving
message boundaries matters, the specification and example notes MUST point to
Soracom Binary Format v1, an HTTP entry point, or an application-layer framing
scheme. Raw TCP payload acceptance alone MUST NOT be documented as a complete
single-message delivery guarantee.

Sequence TOML files are configuration, not source code. The TOML format MUST
remain human-editable and explicit.

TOML shape:

```toml
title = "Quectel Sequences"
description = "Optional description for humans."

[[sequences]]
name = "quectel-tcp-send-check"
summary = "Open a Quectel TCP socket, send a payload, and read a response."
risk = "write"
categories = ["data", "network"]
timeout_secs = 180
before_running = [
  "Uses Quectel TCP/IP AT commands.",
  "Confirm the PDP context before sending; this Sequence can load standard AT+CGACT? and AT+CGDCONT? candidates in the TUI.",
  "During execution, atctl checks AT+QIACT? and sends AT+QIACT=<contextID> only when the selected Quectel PDP context is not active.",
  "Socket connect ID defaults to 0. If needed, run the Quectel socket-state add-on command and edit this value before sending."
]
success_notes = [
  "SEND OK is module accepted payload evidence, not remote application receipt evidence.",
  "End-to-end TCP success requires QIRD response data or remote endpoint evidence."
]

[[sequences.params]]
name = "context_id"
label = "PDP context ID"
required = true
sensitive = false
default = "1"
source = "modem"
candidate = "pdp-context"
hint = "Confirm the PDP context with AT+CGACT? or AT+CGDCONT? before sending."

[[sequences.params]]
name = "connect_id"
label = "Socket connect ID"
required = true
sensitive = false
default = "0"
source = "default"
hint = "Use a free Quectel socket connect ID. Run the Quectel socket-state add-on command when you need to inspect current sockets."

[[sequences.params]]
name = "host"
label = "Host"
required = true
sensitive = false
source = "user"

[[sequences.params]]
name = "port"
label = "Port"
required = true
sensitive = false
source = "user"

[[sequences.params]]
name = "payload"
label = "Payload"
required = true
sensitive = true
source = "user"

[[sequences.review]]
label = "Destination"
value = "{{host}}:{{port}}"
sensitive = false

[[sequences.review]]
label = "Payload"
value = "{{payload}}"
sensitive = true

[[sequences.steps]]
id = "activate-context"
label = "Ensure PDP context active"
ensure_pdp_context_active = "{{context_id}}"
timeout_secs = 150
evidence = "AT+QIACT? checks Quectel TCP/IP PDP context state; AT+QIACT=<contextID> is only sent when the selected context is not active."

[[sequences.steps]]
id = "open-socket"
send = "AT+QIOPEN={{context_id}},{{connect_id}},\"TCP\",\"{{host}}\",{{port}},0,1"
expect_urc = "+QIOPEN: {{connect_id}},0"
timeout_secs = 150
cleanup_on_failure = "AT+QICLOSE={{connect_id}}"

[[sequences.steps]]
id = "send-payload"
send = "AT+QISEND={{connect_id}},{{payload_len}}"
expect_prompt = ">"
payload = "{{payload}}"
terminator = "none"
expect = "SEND OK"
timeout_secs = 60
evidence = "SEND OK means the module accepted the payload for sending."

[[sequences.steps]]
id = "query-send-ack"
send = "AT+QISEND={{connect_id}},0"
expect = "+QISEND:"
require_tcp_ack = true
timeout_secs = 30
```

REQ-SEQ-FILE-001: Each Sequence TOML file MUST include top-level `title`. The
title is the normal user-facing Sequence set label in CLI output and TUI group
headers and `Source: <title>` detail when non-default Sequence sets are
visible. File names MUST NOT be used as the normal display name.

REQ-SEQ-FILE-002: Each `[[sequences]]` entry MUST include `name`, `summary`,
`risk`, and at least one `[[sequences.steps]]` entry.

REQ-SEQ-FILE-003: Each `[[sequences]]` entry MAY include `categories`,
`timeout_secs`, `params`, `review`, `success_notes`, and additional step
metadata needed by the Sequence engine.
`before_running` MAY be used for concise human-facing prerequisites or checks
that should be visible before confirmation. It MUST NOT be used as a hidden
machine dependency model; user-authored add-on files may keep one combined
Action when that matches the operator's actual workflow.
Step metadata MAY include `require_tcp_ack = true` for a step that sends
`AT+QISEND=<connectID>,0` and must not pass until the returned counters show
that the whole payload has been acknowledged by the TCP peer.
Step metadata MAY include `require_ping_success = true` for a step that sends
`AT+QPING=...` and must not pass until the returned `+QPING:` output reports at
least one successful received reply.
Step completion and semantic success are separate contracts. `expect = "OK"`
means a normal final result is enough for that step. `expect_prompt` waits for
an input prompt. `expect_urc` waits through an initial success final result and
keeps reading until the expected marker appears or a terminal error is returned.
Any step that depends on later result lines, asynchronous result lines, counters,
or prompts MUST declare the marker that completes that step; it MUST NOT rely on
the first `OK` when that `OK` only means the command was accepted.
If `require_ping_success = true` is set, the loader MUST reject definitions that
do not wait for `+QPING:` result lines. If `require_tcp_ack = true` is set, the
loader MUST reject definitions that do not read `+QISEND:` acknowledgement
counters.

REQ-SEQ-FILE-004: Sequence parameter definitions MUST identify whether a value
is required and whether it is sensitive. They MAY also define:

- `default`: a prefilled editable value used by TUI and CLI validation.
- `source`: one of `user`, `default`, `modem`, `select`, `sequence`, `derived`,
  or `external`, matching the TUI value-resolution states.
- `candidate`: optional product-known candidate source such as `sms-message` or
  `pdp-context` for values that can be selected from explicitly obtained
  same-session output.
- `hint`: a concise user-facing instruction for how the value is confirmed,
  selected, derived, or entered.

Sensitive parameters such as SMS message bodies, phone numbers, payloads, APN
credentials, usernames, passwords, and application tokens MUST be masked in
normal Response output, Response exports, history, session logs, and CLI JSON
unless the user explicitly requests unmasked foreground output or Response
export, or enables raw diagnostic export with the required acknowledgement.

REQ-SEQ-FILE-004A: Sequence value-resolution metadata is a product contract, not
documentation decoration. TUI input, TUI confirmation, CLI missing-parameter
errors, and applicable user documentation MUST use the same `default`, `source`,
`candidate`, `hint`, and `before_running` metadata. A repository-managed example Sequence MUST NOT
require a user to know an internal modem value such as PDP context ID or socket
connect ID without either a default, a modem/source hint, product-known
candidate assistance, or an explicit external prerequisite explanation.

REQ-SEQ-FILE-004B: Defaults MUST reduce routine entry work without hiding
modem-specific reality. If a default is supplied for a modem-dependent value,
the Sequence MUST still label the value source or hint so the operator can
confirm when the default does not match the current modem state. Defaults MUST
NOT be used for sensitive payloads, SMS bodies, credentials, application tokens,
or values that would create an unintended destination.

REQ-SEQ-FILE-005: Sequence steps MUST be explicit. A Sequence definition MUST
NOT hide prompt-required or URC-required behavior inside a single concatenated
command string.

REQ-SEQ-FILE-006: Sequence loading MUST reject invalid TOML with a file path and
actionable parse error. Loading MUST reject duplicate Sequence names across
product-provided standard Sequences and all loaded Sequence definition files.

REQ-SEQ-FILE-007: Sequence loading MUST NOT create, overwrite, or silently
repair Sequence definition files.

REQ-SEQ-FILE-008: User-defined Sequence definitions MUST be loaded from the
paths defined in section 19 or from explicit per-invocation Sequence location
flags. Repository-managed example Sequences MUST be loaded through those same
explicit location flags during verification.

REQ-SEQ-FILE-009: Vendor, carrier, or modem identity MUST be represented as a
Sequence set title and documentation context, not as a generated Category value.
For example, `Quectel Sequences` is an acceptable Sequence set title, but
`quectel` MUST NOT be generated as a workflow category.

REQ-SEQ-FILE-010: Standard AT PDP readiness remains covered by one-shot
standard presets such as `packet-attach`, `active-pdp-contexts`,
`pdp-addresses`, and `pdp-connection-details`. Portable external TCP data-send
verification is not provided by standard one-shot AT commands; external
data-send Sequence examples therefore belong in vendor-specific Sequence
definition files.

REQ-SEQ-FILE-011: A Sequence MAY define `[[sequences.review]]` items with
`label`, `value`, and `sensitive`. Review item values are templates rendered
from Sequence parameters and derived values such as `{{payload_len}}`. When a
Sequence defines no review items, the product SHOULD review the supplied
parameters by label before sending. Review items support the user's send
decision and MUST be shown before USB access for confirmation-required
Sequences.

REQ-SEQ-FILE-012: Active Sequence input and pre-send review surfaces MAY show
the current typed values for sensitive review items because the operator must be
able to verify the destination and content before an irreversible send. This
display MUST be limited to the active input or confirmation surface and MUST NOT
be written to normal Response output, Response exports, history, session logs,
or CLI JSON by default.

REQ-SEQ-FILE-013: A Sequence MAY define `success_notes`. These notes MUST be
included in the `Notes:` text transcript section and CLI JSON notes after
successful execution.
Notes MUST state the verified evidence level, especially when a modem result is
not end-to-end evidence.

REQ-SEQ-FILE-014: A Sequence step MAY define `evidence`. Step evidence MUST be
rendered as atctl-derived analysis when the step succeeds. In text transcripts,
it MUST appear under `Analysis:` rather than as a literal `Evidence:` line. In
structured CLI JSON step results, it MUST appear in `analysis`. Evidence text
MUST describe what that step proves and what it does not prove when the
distinction affects user judgement.

REQ-SEQ-FILE-014A: A Quectel TCP/IP Sequence step MAY define
`ensure_pdp_context_active = "<contextID template>"`. This is a product
execution contract for state-aware PDP activation, not a user prerequisite.
When executed, atctl MUST send `AT+QIACT?`, check whether the selected context
is already active, and send `AT+QIACT=<contextID>` only when that context is not
active. The transcript MUST show the command or commands actually sent, the
modem responses, and `Analysis:` explaining whether the context was reused or
activated. This prevents repeated runs from failing only because the selected
PDP context is already active.

REQ-SEQ-FILE-014B: A Sequence step MAY define `cleanup_on_failure` with a
single AT command template that is run only after that step succeeds and a later
step in the same Sequence fails. Cleanup commands MUST be visible in the normal
transcript and raw diagnostic export when raw export is enabled. Cleanup is
best-effort failure recovery for product-managed resources such as Quectel TCP
socket connect IDs; it MUST NOT hide the original failed step or replace the
original failure reason.

REQ-SEQ-FILE-015: Sequence text output MUST remain human-readable and must show
each step transcript with the origin sections defined in
REQ-SEQ-ENGINE-004A. CLI JSON output MUST additionally expose structured step
results and notes so automation can distinguish prompt acceptance, URC success,
payload acceptance, SMS decoded-body analysis, TCP acknowledgement counters,
received response data, and cleanup without parsing human transcript prose.

REQ-SEQ-FILE-016: TCP data-send result wording MUST distinguish:

- PDP/context activation evidence;
- socket-open evidence;
- module-accepted-payload evidence;
- TCP peer acknowledgement/counter evidence;
- remote response or destination-log evidence;
- cleanup evidence.

REQ-SEQ-FILE-017: TUI `Run Sequence` input and confirmation MUST show current
values inside the active modal. For SMS this includes destination and message
body for SMS send, SMS storage index and reply body for SMS reply-by-index, and
SMS storage index for SMS read. The SMS storage index value MUST be presented as
a modem/select value, not as an unexplained arbitrary integer. For TCP this
includes destination host/port, PDP context ID, socket connect ID, payload, and
read/response expectation. Destination values fixed by a SORACOM Sequence MUST
be shown as Sequence-provided review values, not as hidden user inputs. The TUI
MUST still keep normal Response, Response export, history, and session logs
masked by default.

REQ-SEQ-FILE-018: `atctl send`, `atctl preset run`, TUI one-shot AT command input,
and PTY bridge direct command handling remain one-shot or manual-command
surfaces. Product-provided SMS and repository-managed TCP examples MUST be
available through Sequences rather than by concatenating prompt-required AT
commands into a one-shot command.

## 18. Masking

REQ-MASK-001: Screen output and saved logs MUST mask sensitive values by default.

REQ-MASK-002: Values to mask include IMSI, ICCID, IMEI, MSISDN, APN usernames,
APN passwords, SMS sender/recipient numbers, SMS message bodies, Sequence
payloads marked sensitive, application tokens, and long numeric identifiers
likely to be subscriber or device identifiers.

REQ-MASK-002A: `+QCCID:` response values MUST be masked as one sensitive ICCID
value token. If a modem returns trailing `F` padding with the ICCID response,
the trailing `F` MUST also be hidden in masked output. Unmasked foreground
display and acknowledged raw diagnostic export may show the modem-returned
padding.

REQ-MASK-002B: `AT+CGAUTH=...` command strings and `+CGAUTH:` response lines
MUST mask APN authentication usernames and passwords, including short default
values and custom CHAP credentials. Masking MUST preserve enough structure for
the user to see the CID and authentication protocol while hiding credentials.

REQ-MASK-002C: SMS bodies decoded from `+CMGL` or `+CMGR` responses remain SMS
message bodies after decoding. Normal output MUST mask the decoded body and
unmasked foreground output MUST show the decoded body when decoding is
supported. Acknowledged raw diagnostic export remains a raw modem exchange and
may contain encoded modem-returned body bytes. Normal and unmasked foreground
transcripts MUST NOT expose partially masked encoded UCS2 hex as a substitute
for decoded content. QIRD response data returned by TCP example Sequences is
application response data and MUST be masked when it matches sensitive Sequence
parameters or when the Sequence analysis only needs to report response length.

REQ-MASK-003: Masking MUST apply to command responses, JSON output, session
logs, command history, and TUI panes unless the user explicitly requests
unmasked foreground display or acknowledged raw diagnostic export.

REQ-MASK-003A: The TUI and interactive CLI active Sequence input/review
surfaces may show user-entered sensitive values before sending, as specified by
REQ-SEQ-FILE-012. This exception is limited to active input/review and does not
change Response, log, raw export, save, copy, or JSON masking requirements.

REQ-MASK-004: If a command string itself contains a credential, logs MUST mask
the sensitive part of the command string.

REQ-MASK-005: `--no-mask` MUST affect foreground output, Response copy, and an
explicitly requested normal Response export. It MUST NOT cause command history
or session logs to become raw and MUST NOT imply raw diagnostic export creation.
For `atctl tui`, `--no-mask` starts the TUI session with output masking off; that
setting remains limited to foreground Response display, copy, and explicit
Response export behavior.

REQ-MASK-006: Raw log diagnostic export MUST require explicit user action, a
user-selected output destination, a sensitive-data warning, and the
acknowledgement defined by the OQ-021 decision. A later specification revision
MAY replace these requirements.

Example:

```text
Original:
+QCCID: 89811000123456789012

Masked:
+QCCID: 89811000************
```

## 19. Definition Loading and State Paths

`atctl` stores generated user state under an XDG state base. When
`XDG_STATE_HOME` is unset or empty, the base is `$HOME/.local/state`:

```text
$XDG_STATE_HOME/atctl/
  history.jsonl
  logs/
```

REQ-STATE-001: The implementation MUST honor `XDG_STATE_HOME` when it is a
non-empty absolute path and MUST append the application directory `atctl`.

REQ-STATE-002: When `XDG_STATE_HOME` is unset, empty, or relative, the
implementation MUST use `$HOME/.local/state` as the state base. A relative XDG
base MUST NOT be resolved against the process working directory.

REQ-STATE-003: If neither a valid absolute `XDG_STATE_HOME` nor a valid absolute
`HOME` is available, commands that require the state directory MUST fail with an
actionable error instead of writing under a literal `~` or a relative path.

REQ-STATE-004: State directories created by `atctl` SHOULD use user-only
permissions. The implementation MUST surface directory-creation and file-write
failures with the resolved path.

REQ-STATE-005: `atctl` MUST NOT discover or load a per-user product
configuration file. USB target, interface, and endpoint overrides are explicit
per-invocation options or TUI selections. Normal masked logging is enabled by
default and is disabled for one supported execution invocation with
`--no-log`.

File preset loading:

```text
Explicit file preset location flags for the current invocation:
  --preset-file <FILE>
  --preset-dir <DIR>
```

Sequence definition loading:

```text
Explicit Sequence definition location flags for the current invocation:
  --sequence-file <FILE>
  --sequence-dir <DIR>
```

REQ-LOAD-001: Invalid preset or Sequence TOML MUST produce actionable errors
with the file path and parse location when available.

REQ-LOAD-002: Explicit preset and Sequence directory loading MUST read only
`.toml` regular files from the provided directory.

REQ-LOAD-003: Explicit preset and Sequence directory files MUST be loaded in
deterministic lexicographic path order within each provided directory.

REQ-LOAD-004: Normal startup MUST load product-provided definitions. Add-on
preset or Sequence files MUST join the loaded set only when their explicit file
or directory flags are provided.

REQ-LOAD-005: File preset TOML entries MAY include `timeout_secs` as a positive
integer hint for command execution timeout. The value applies to that preset
only and MUST NOT change global defaults.

REQ-LOAD-006: Explicit file preset and Sequence location flags MUST apply only
to the invocation where they are provided. They MUST NOT modify environment
variables or future invocations.

REQ-LOAD-007: The implementation SHOULD NOT add atctl-specific preset or
Sequence directory environment variables. Explicit file and directory flags
remain the per-invocation loading contract.

## 20. Logging and History

Normal masked logging is enabled by default for direct send, preset execution,
Sequence execution, and TUI execution. The default paths are:

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/YYYY-MM-DDTHH-MM-SS-NNNNNNNNNZ.session.log
```

For a non-empty absolute `XDG_STATE_HOME`, the same files are stored under
`$XDG_STATE_HOME/atctl/`. The environment value applies to the current process
and its children, so log creation and later `atctl logs list` invocations use
the same state base only when they receive the same value.

REQ-LOG-001: Session logs, command history, masked response logs, and optional
raw logs MUST be separate concepts.

REQ-LOG-001A: `atctl send`, `atctl preset run`, `atctl sequence run`, and
`atctl tui` MUST write new masked command-history and session-log artifacts by
default after an AT execution produces a recordable result.

REQ-LOG-001B: `--no-log` MUST disable creation of both new masked command
history and new masked session logs for the current `send`, `preset run`,
`sequence run`, or TUI invocation. For TUI, the option applies to every AT
execution in that TUI process. It MUST NOT delete, hide, or prevent read-only
review of existing logs.

REQ-LOG-001C: `--no-log` MUST NOT disable raw diagnostic export explicitly
requested with a destination and acknowledgement. Normal masked logging and raw
diagnostic export remain independent contracts.

REQ-LOG-001D: PTY bridge does not create normal command-history or session-log
artifacts because an external PTY client owns its command session and the bridge
does not produce the same per-command execution record. Bridge raw diagnostic
export remains available only through its explicit raw-export options.

REQ-LOG-002: Raw logs MUST be disabled by default. They MUST NOT be created in
the default state/log directory without an explicit user-selected output
destination.

REQ-LOG-003: Log entries SHOULD include timestamp, device VID/PID, selected
device identity, interface, endpoints, command, risk level, masked response,
duration, and status.

REQ-LOG-004: Log files SHOULD use user-only permissions when created.

REQ-LOG-005: Normal masked history and session logs MUST remain on disk until the
operator removes them. `atctl` MUST NOT apply an automatic retention period,
rotation policy, pruning policy, or deletion policy. User documentation MUST
describe the two normal log artifacts, how to list and review them, the absence
of automatic lifecycle management, the effect of deleting each artifact type,
and the separate lifecycle of explicitly selected raw diagnostic exports.

REQ-LOG-006: The TUI Logs pane MUST provide useful in-app review of saved
masked log material, not only file names, when masked log viewing is available.
The Logs pane is a read-only diagnostic-evidence review surface,
not a log deletion, retention, rotation, raw-export, or file-management
surface.

REQ-LOG-007: TUI masked log viewing MUST read only existing history and session
log files. It MUST NOT create logs as a side effect of viewing.

REQ-LOG-008: TUI masked log viewing MUST display saved masked content only.
It MUST NOT reveal raw response values and MUST NOT bypass output masking
confirmation model.

REQ-LOG-009: The TUI Logs pane MUST make log selection visible through
keyboard navigation and non-color cues. Opening a selected log MUST update the
Response or another content pane with masked log content and status feedback.

REQ-LOG-010: Masked log content opened in the TUI MUST be scrollable by
keyboard. The user MUST be able to reach content below the visible Response pane
without leaving the TUI.

REQ-LOG-011: The TUI Response pane's primary role remains current or most
recent AT command response display. Opening a saved log MAY temporarily put the
Response pane into masked log-view mode, but this MUST NOT redefine the pane as
a general-purpose log viewer or weaken its AT response role.

REQ-LOG-012: While the Response pane is temporarily showing masked log content,
the pane itself MUST show the visible line range and total line count. The
range indicator MUST describe the visible range, not only the first visible
line. It SHOULD also indicate when the view is at the top, at the bottom, or
has more content above/below.

REQ-LOG-013: While the Response pane is temporarily showing masked log content,
the displayed log content MUST include line numbers so the user can identify
the current location from the same pane as the content. These line numbers are
for read-only orientation and MUST NOT imply an editable cursor or selectable
text position.

REQ-LOG-014: Opening a selected saved log from Logs SHOULD move focus to the
Response pane because the next likely task is reading or scrolling the opened
content. Logs selection state SHOULD remain visible so the user can return
to the selected log list.

REQ-LOG-015: Saved log listings that present session logs as recent or saved
diagnostic material MUST list session log files newest-first. The aggregate
history file MAY be pinned separately because `history.jsonl` is a single
append-only history file, not one file per session.

REQ-LOG-016: The TUI pane that lists saved history and session log files MUST
be titled `Logs`, not `History`, because it contains both command history and
session logs. Row labels MAY continue to use `history:` and `session:` to show
the log type. In-pane heading SHOULD describe the mixed content as `Saved logs`
rather than `Recent logs` when the list includes the aggregate history file.

REQ-LOG-017: The TUI MUST provide a Logs pane action menu for opening a selected
saved log in Response or revealing that selected file in Finder. Opening the
menu MUST perform only a non-destructive refresh of the saved-log list before
showing available actions. The Logs menu MUST NOT provide a generic log-folder
action because its rows include both the aggregate `history.jsonl` in the state
directory and individual session files in the `logs/` subdirectory. It MUST NOT
add destructive actions such as delete, clear, prune, or rotate. Opening a
selected log in Response MUST keep the existing masked log-view behavior.
Revealing a selected log MUST NOT require opening it in Response first. Both
actions MUST remain bound to the log identity that the user selected when
opening the menu, based on log type and exact path. If no log is available, pressing
`Enter` in Logs MUST keep the pane unchanged and provide concise no-log feedback
instead of opening an empty action menu. If a selected target no longer exists
after the pre-menu refresh, the TUI MUST update the Logs list, show that the
selected log no longer exists, and MUST NOT offer an open-log action for another
row that moved into the same list index.
That missing-log message is modal-state feedback, not selection-row feedback;
it MUST remain visible while the same Log actions menu stays open, including
after Up/Down/Home/End selection movement, and it is cleared only when the menu
is closed, an action closes the menu, or the menu is reopened and state is
recomputed.
If that target disappears after the menu is already open, opening it MUST fail
against the originally selected target, MUST show the missing-file failure in
Response, MUST refresh the Logs list, and MUST NOT fall back to `history.jsonl`
or any other remaining log.

REQ-LOG-017A: `Reveal in Finder` from Logs MUST target the exact selected path,
and the same action from a saved log visible in Response MUST target the exact
opened path, regardless of whether the log is the aggregate history file or an
individual session file. On the supported macOS runtime, both actions MUST ask
Finder to select the file without opening its contents and without modifying or
deleting it. The TUI MUST describe successful process launch as a request sent,
not as verified Finder state. If the file no longer exists before the Logs menu
opens, both selected-log actions MUST be unavailable and the list MUST refresh
without retargeting another row. If an opened log later disappears, copy
displayed log and close log view MUST remain available, while `Reveal in Finder`
MUST be disabled with an actionable missing-file explanation. Neither action
MUST fall back to a parent directory or another log file.

REQ-LOG-018: Raw diagnostic export files MUST be written as JSON Lines. The
format MUST include a schema/version event, surface/source metadata, command
metadata, transmitted bytes, received bytes, status, duration, and a warning
that the file may contain sensitive modem, subscriber, network, APN, or PDP
authentication values. When command transmission has started but no parsed AT
response is available, the file MUST include a `transport_error` event with the
command, stage, error text, duration, and any transmitted bytes known to the
application. USB target selection failures before command transmission MUST NOT
create a misleading raw exchange file.

REQ-LOG-019: Raw diagnostic export byte payloads MUST be lossless. Base64
fields are required for transmitted and received bytes in the first
implementation. Human-readable previews MAY be included, but they MUST NOT be
treated as the authoritative raw payload.

REQ-LOG-020: Raw diagnostic export write errors MUST be surfaced as command
errors. The implementation MUST validate raw export acknowledgement and
overwrite refusal before USB access for CLI and bridge execution.

REQ-TUI-A11Y-014: TUI keyboard focus order MUST preserve meaning and
operability. Informational panes that have no direct action SHOULD NOT be part
of the normal Tab focus cycle. The normal TUI focus cycle MUST prioritize
interactive panes in a visible and predictable order.

Reference basis for TUI log viewing:

- Nielsen Norman Group's visibility-of-system-status heuristic supports
  showing useful state and recent activity so users can understand what the
  system has done.
- Nielsen Norman Group's recognition-rather-than-recall heuristic supports
  making relevant prior information available in the interface instead of
  requiring the user to remember file paths or leave the TUI.
- Docker Desktop, Kubernetes `kubectl logs`, and similar operational tools
  expose logs for review or streaming without making log deletion/rotation the
  primary in-product workflow.
- Apple `NSWorkspace.activateFileViewerSelecting` opens Finder with the exact
  target file selected. VS Code and IntelliJ IDEA similarly expose file-manager
  reveal actions for a file selected in a list or currently open in an editor.
  Firefox similarly provides open and Finder-reveal actions for one completed
  saved file and changes action availability with file state. These patterns
  support independent selected-file, opened-file, and newly saved-file paths
  without forcing one task to precede another.
- OWASP logging guidance and NIST log-management guidance support keeping log
  generation, viewing, protection, retention, and disposal responsibilities
  distinct.
- Apple Human Interface Guidelines describe lists/tables as row-based data
  presentation and disclosure controls/progressive disclosure as a way to reveal
  related detail from a selected item.
- W3C WCAG2ICT status-message guidance reinforces that non-web software still
  has a user need for perceivable status updates when context changes.
- W3C WCAG focus-order guidance requires keyboard focus order to preserve
  meaning and operability.
- WAI-ARIA keyboard-interface practices and Apple keyboard-navigation guidance
  distinguish focus movement between groups from arrow-key movement within a
  focused group.

## 21. Error Handling

REQ-ERR-001: Error messages MUST explain what failed, likely cause, and next
action.

REQ-ERR-002: Error messages MUST distinguish device-not-found, multiple-device,
open failure, claim failure, endpoint detection failure, write failure, read
timeout, AT error, and parser limitation where possible.

Example:

```text
No matching USB device found.

Checked selectors:
  - VID: not specified
  - PID: not specified
  - Bus: not specified
  - Address: not specified

Try:
  atctl devices
  system_profiler SPUSBHostDataType
```

## 22. Packaging

REQ-PKG-001: `atctl` MUST be packaged as a command-line executable that
provides CLI and TUI surfaces. The normal distribution path MUST NOT treat
`atctl` as a macOS GUI application, app bundle, or Homebrew Cask unless a later
specification revision defines a GUI application.

REQ-PKG-002: The normal end-user install flow MUST be Homebrew through the
fully qualified formula name `uchimanajet7/atctl/atctl`:

```sh
brew install uchimanajet7/atctl/atctl
```

The equivalent tapped form is:

```sh
brew tap uchimanajet7/atctl
brew install atctl
```

REQ-PKG-003: The Homebrew formula MUST declare `libusb` as a runtime
dependency.

REQ-PKG-004: The preferred normal Homebrew install state SHOULD provide a
Homebrew bottle for each packaged platform target. When a matching bottle is
available, Homebrew can install the bottled binary while preserving the same
fully qualified install flow.

REQ-PKG-005: The Homebrew formula MUST keep source-build support as a fallback
for cases where no matching bottle is available, the user disables bottle use,
or tap maintainers need source-build verification.

REQ-PKG-006: If the formula builds from source, it SHOULD declare `rust` and
`pkgconf` as build dependencies.

REQ-PKG-007: If the formula installs a bottle or other prebuilt artifact, Rust
MUST NOT be documented as a runtime dependency.

REQ-PKG-008: Release artifacts for platforms outside Apple Silicon macOS MUST
NOT be promised unless the release process validates them.

REQ-PKG-009: The source repository MUST be
`https://github.com/uchimanajet7/atctl`.

REQ-PKG-010: The Homebrew tap repository MUST be
`https://github.com/uchimanajet7/homebrew-atctl`.

REQ-PKG-011: User-facing Homebrew documentation MUST use the fully qualified
formula name `uchimanajet7/atctl/atctl` as the normal one-line install command,
and may also document the equivalent tapped form using the tap name
`uchimanajet7/atctl`.

REQ-PKG-012: Packaging documentation MUST explain that Homebrew's one-argument
GitHub tap form maps `brew tap <user>/<repo>` to
`https://github.com/<user>/homebrew-<repo>`.

REQ-PKG-013: Packaging documentation MUST keep local development builds, CI
verification builds, source repository release artifacts, and Homebrew
installation behavior as separate concerns.

REQ-PKG-014: GitHub Actions release workflows and GitHub Releases assets belong
to the source repository `uchimanajet7/atctl`.

REQ-PKG-015: Source repository releases MUST use GitHub Actions to build and
publish a prebuilt Apple Silicon macOS binary and checksum to GitHub Releases.
The source release MUST be started through the manually triggered GitHub Web
workflow with an explicit release-tag input. Pushing a release tag MUST NOT
automatically start the source release workflow. This release artifact is
separate from the Homebrew bottle contract.

REQ-PKG-016: The Apple Silicon macOS GitHub Release archive asset MUST be named
`atctl-v{VERSION}-aarch64-apple-darwin.tar.gz`, where `{VERSION}` is the
semantic version without the leading `v`.

REQ-PKG-017: The checksum asset for the Apple Silicon macOS GitHub Release
archive MUST be named `atctl-v{VERSION}-aarch64-apple-darwin.tar.gz.sha256`.

REQ-PKG-018: The Apple Silicon macOS checksum asset MUST contain one
sha256sum-compatible line:
`<sha256 hex>  atctl-v{VERSION}-aarch64-apple-darwin.tar.gz`.

REQ-PKG-019: Source repository releases MUST publish one checksum file per
archive and MUST NOT publish an aggregate checksum manifest unless this
specification is revised.

REQ-PKG-020: Source repository releases MUST NOT publish provenance,
attestation, or SBOM metadata unless this specification is revised.

REQ-PKG-021: Homebrew formula, tap CI, tap metadata, and bottle metadata belong
to the tap repository `uchimanajet7/homebrew-atctl`.

REQ-PKG-022: The Homebrew formula source-build fallback MUST build from the
`uchimanajet7/atctl` source repository release archive.

REQ-PKG-023: The Homebrew tap SHOULD publish bottles for packaged platform
targets as part of the normal Homebrew distribution path.

REQ-PKG-024: Homebrew bottle publishing MUST be implemented and maintained in
the tap repository. The source repository release workflow MUST NOT publish
Homebrew bottles or run tap-repository publication as a hidden release side
effect.

REQ-PKG-025: A decision to build GitHub Release binaries MUST NOT imply that
Homebrew installs those GitHub Release binaries.

REQ-PKG-026: A decision to publish Homebrew bottles MUST NOT imply that direct
GitHub Releases download is the ordinary end-user install path.

REQ-PKG-027: GitHub Releases prebuilt `.tar.gz` artifacts MUST be documented as
release artifacts and manual artifacts, not as the normal end-user install path.

REQ-PKG-028: Direct GitHub Releases download MUST NOT be presented as the
normal end-user install path.

REQ-PKG-029: Developer ID signing and Apple notarization are not required for
the normal Homebrew install path.

REQ-PKG-030: Developer ID signing and Apple notarization are not required before
publishing GitHub Releases `.tar.gz` artifacts that are documented only as
release/manual artifacts.

REQ-PKG-031: If direct GitHub Releases download is promoted to a normal end-user
install path, Developer ID signing, Apple notarization, Gatekeeper behavior,
quarantine warnings, credential handling, and CI secret handling MUST be decided
before that promotion.

REQ-PKG-032: Packaging documentation MUST warn that unsigned or unnotarized
direct-download macOS binaries may trigger Gatekeeper or quarantine warnings
depending on how the user downloads and runs them.

REQ-PKG-033: The Cargo source package MUST be treated as Rust packaging
metadata and source-package output, not as the normal end-user install path.
The normal end-user install path remains Homebrew.

REQ-PKG-034: `Cargo.toml` MUST use an explicit `include` whitelist for the
Cargo source package so project-local process files, backups, local history,
build outputs, release-workflow drafts, and other non-package repository
materials cannot be accidentally included in `.crate` output.

REQ-PKG-035: The Cargo source package include list MUST contain only files
needed for the Rust source package, package metadata, and package verification:
`src/**`, repository-managed preset and Sequence examples under
`examples/presets/**` and `examples/sequences/**`, `README.md`, `CHANGELOG.md`,
and `LICENSE`.

REQ-PKG-036: Repository documentation under `docs/**` MUST remain source
repository documentation and MUST NOT be included in the Cargo source package
only because `README.md` references those documents. `README.md` links to
repository documentation SHOULD remain usable when rendered outside the GitHub
repository, such as on crates.io or docs.rs.

REQ-PKG-037: The Cargo source package contract MUST NOT be used as evidence
that the Homebrew source-build fallback or GitHub release-source archive
contains exactly the same file set. Homebrew source-build fallback, GitHub
release source archives, Homebrew bottles, and Cargo source packages are
separate release concerns.

REQ-PKG-038: The source repository release workflow MUST validate that the
release tag version without the leading `v` matches `Cargo.toml`
`package.version` before building or publishing release assets.

REQ-PKG-038A: The source repository release workflow MUST expose a manual
`workflow_dispatch` operation with a required `release_tag` input such as
`v0.1.0` so an operator can start the source repository release from the GitHub
Actions Web UI without manually creating a GitHub Release page or copying
release notes.

REQ-PKG-038B: When the source repository release workflow is manually
dispatched, it MUST validate the version, verify the source, build and package
the release artifact and checksum, and prepare the changelog-backed release
notes before creating a missing tag or GitHub Release. If the requested tag
already exists, the workflow MUST verify before the build and again immediately
before publication that the tag points to the selected workflow commit. It MUST
fail without moving, overwriting, or deleting a mismatched tag.

REQ-PKG-038C: The manual source repository release path MUST complete tag
preparation, release verification, archive creation, checksum creation,
changelog-backed release-note extraction, and GitHub Release creation in the
same workflow run. The final publication operation MUST create a missing tag at
the selected workflow commit or verify an existing tag, upload the archive and
checksum through a draft release, and publish the GitHub Release. It MUST NOT
rely on a second tag-push-triggered workflow run.

REQ-PKG-038D: A failure during version validation, source verification, release
build, packaging, checksum creation, or release-note preparation MUST occur
before creation of a new remote tag or GitHub Release. A failure after the
final GitHub publication operation has started MAY leave a recoverable draft
and associated tag. The workflow MUST NOT automatically move or delete remote
tags or releases as failure recovery; the operator MUST inspect that remote
state before retrying.

REQ-PKG-039: The source repository release workflow MUST extract the
matching released-version section from `CHANGELOG.md` and use that extracted
section as the GitHub Release notes.

REQ-PKG-040: The `CHANGELOG.md` released-version section used for GitHub
Release notes MUST include the released package version and a `YYYY-MM-DD`
release date in the section heading.

REQ-PKG-041: The source repository release workflow MUST fail before creating a
new remote tag or GitHub Release when the matching `CHANGELOG.md` version
section is missing or has no release-note content beyond the heading.

REQ-PKG-042: GitHub automatically generated release notes MUST NOT be the sole
or primary release notes for `atctl` source repository releases unless a later
specification revision changes the release-note source.

REQ-PKG-043: The source repository release workflow MUST NOT create or
update Homebrew Formula pull requests automatically.

REQ-PKG-044: Homebrew Formula update pull-request creation MUST be an explicit
tap-repository operation in `uchimanajet7/homebrew-atctl`, separate from source
repository release builds.

REQ-PKG-045: The tap repository SHOULD provide a manually triggered
`workflow_dispatch` workflow that takes a source repository release tag as
input, updates `Formula/atctl.rb`, and creates or updates a pull request in
`uchimanajet7/homebrew-atctl`.

REQ-PKG-046: Source repository release creation and Homebrew publication MUST
remain independently executable. A source repository GitHub Release MAY be
created without publishing Homebrew material, and Homebrew publication MUST
require a separate explicit operator action.

## 23. Verification Strategy

REQ-TEST-001: Unit tests MUST cover masking, AT parser status detection, config
parsing, preset risk classification, and CLI argument parsing where practical.

REQ-TEST-002: USB transport MUST be testable through a mock transport or fixture
without requiring physical hardware.

REQ-TEST-003: Hardware integration tests MUST be separated from ordinary unit
tests and MUST not be required unless a connected modem is explicitly available.

REQ-TEST-004: Documentation MUST explain how to run tests without hardware.

REQ-TEST-004A: The normal Rust verification gate MUST include formatting,
full-target type checking, all-feature unit/doc tests, and Clippy with warnings
treated as errors:

```sh
cargo fmt --check
cargo check --all-targets --all-features --locked
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

REQ-TEST-004B: The source repository MUST provide a GitHub Actions source-change
workflow that runs the normal Rust verification gate for every pull request
targeting `main`, every push to `main`, and an explicit manual dispatch. The
pull-request trigger MUST NOT use path filters, so the same named status check
is reported for documentation-only and source-code changes.

REQ-TEST-004C: The source-change quality job MUST run on the GitHub-hosted
`macos-26` Apple Silicon runner and MUST fail when `uname -m` is not `arm64`.
It MUST install the `libusb` and `pkgconf` build prerequisites and run the exact
REQ-TEST-004A commands with the committed `Cargo.lock`.

REQ-TEST-004D: The source-change workflow MUST use read-only repository-content
permission, MUST pin third-party Actions to immutable full commit SHAs, and MUST
disable persisted checkout credentials when later steps do not need GitHub
write access. Superseded runs for the same pull request or ref SHOULD be
cancelled.

REQ-TEST-004E: After the source-change workflow has produced its first
successful **Rust quality gate** check, the GitHub repository rules for `main`
MUST require that named check before merge. The workflow definition and the
repository rule are separate controls: the workflow reports the result, while
the repository rule enforces it. The release workflow MUST retain its own
pre-publication execution of the normal Rust verification gate.

REQ-TEST-005: TUI verification MUST include terminal restoration behavior and
basic interaction state checks before TUI completion is claimed.

REQ-TEST-006: TUI theme verification MUST include automated checks for semantic
style roles. When specified light/dark palettes exist, verification MUST include
contrast checks for the specified foreground/background pairs. Manual screenshots
or user visual review MAY supplement these checks, but MUST NOT replace them
when objective accessibility claims are made.

REQ-TEST-007: Sequence verification MUST include unit tests for Sequence TOML
loading, duplicate detection, parameter validation, risk aggregation, prompt
waits, payload writes, URC waits, per-step timeout handling, masked transcript
rendering, raw diagnostic export event coverage, CLI `sequence list/run`, and
TUI Sequence selection/input/result rendering through mock transport fixtures.

REQ-TEST-008: Repository-managed example Sequences MUST be verified through the
same loader path used for user-authored Sequence definitions, such as by passing
`--sequence-dir examples/sequences`. A separate example-only parser path is not
acceptable verification.

REQ-TEST-009: TUI grouping or wording changes MUST include cross-state
render-buffer or snapshot coverage for the affected whole-surface UI grammar.
For executable-item grouping, tests MUST compare the relevant adjacent states
instead of only asserting one local state, such as product-only commands,
mixed product/file presets, mixed command/Sequence results, and loaded
repository-managed examples.

## 24. Accepted Decisions

Accepted product and architecture decisions are summarized in
`docs/DECISIONS.md`. That document records decision rationale and consequences;
the normative product and technical requirements remain in this specification.

Any new unresolved decision that affects product behavior, safety, UI,
transport, packaging, or distribution must be raised explicitly and must not be
guessed. A later accepted decision must identify the earlier decision it
supersedes.

## 25. References

- ISO/IEC/IEEE 29148:2018:
  https://www.iso.org/standard/72089.html
- IEEE/ISO/IEC 29148-2018:
  https://standards.ieee.org/ieee/29148/6937/
- NASA, Appendix C: How to Write a Good Requirement:
  https://www.nasa.gov/reference/appendix-c-how-to-write-a-good-requirement/
- RFC 2119:
  https://www.rfc-editor.org/rfc/rfc2119
- RFC 3339:
  https://datatracker.ietf.org/doc/html/rfc3339
- 3GPP TS 27.007:
  https://www.3gpp.org/dynareport/27007.htm
- 3GPP TS 27.005:
  https://www.3gpp.org/DynaReport/27005.htm
- 3GPP TS 23.038:
  https://www.3gpp.org/DynaReport/23038.htm
- 3GPP TS 23.040:
  https://www.3gpp.org/DynaReport/23040.htm
- ITU-T V.250:
  https://www.itu.int/rec/T-REC-V.250/en
- SORACOM Onyx software setup:
  https://developers.soracom.io/en/docs/soracom-onyx-lte-usb-modem/software-setup/
- SORACOM Onyx advanced usage:
  https://developers.soracom.io/en/docs/soracom-onyx-lte-usb-modem/advanced-usage/
- SORACOM Onyx troubleshooting:
  https://developers.soracom.io/en/docs/soracom-onyx-lte-usb-modem/troubleshooting/
- SORACOM Onyx product:
  https://soracom.io/onyx-cellular-usb-modem/
- SORACOM APN settings:
  https://users.soracom.io/ja-jp/docs/air/apn-settings/
- SORACOM CHAP authentication:
  https://users.soracom.io/ja-jp/docs/air/configure-chap/
- SORACOM service endpoints:
  https://developers.soracom.io/en/docs/reference/endpoints/
- SORACOM modem testing and Ping Response Service:
  https://developers.soracom.io/en/docs/soracom-onyx-lte-usb-modem/testing/
- SORACOM Unified Endpoint:
  https://developers.soracom.io/en/docs/unified-endpoint/
- Soracom Binary Format v1:
  https://developers.soracom.io/en/docs/groups/binary-format-v1/
- RFC 9293, Transmission Control Protocol:
  https://www.rfc-editor.org/rfc/rfc9293
- Quectel EG25-G:
  https://www.quectel.com/product/lte-eg25-g/
- Quectel EG25-G hardware design:
  https://quectel.com/content/uploads/2024/04/Quectel_EG25-G_Hardware_Design_V1.5.pdf
- Quectel EC2x/EG2x/EG9x/EM05 AT commands manual landing page:
  https://www.quectel.com/download/quectel_ec2xeg2xeg9xem05_series_at_commands_manual_v2-2/
- Quectel EC2x/EG2x/EG9x/EM05 QCFG AT commands manual:
  https://quectel.com/content/uploads/2024/02/Quectel_EC2xEG2xEG9xEM05_Series_QCFG_AT_Commands_Manual_V1.0.pdf
- Quectel EC2x/EG9x/EM05 TCP/IP AT commands manual:
  https://sixfab.com/wp-content/uploads/2018/09/Quectel_EC2xEG9xEM05_TCPIP_AT_Commands_Manual_V1.0.pdf
- Ratatui popup example:
  https://ratatui.rs/examples/apps/popup/
- Cloudscape Design System Timestamps:
  https://cloudscape.design/patterns/general/timestamps/
- Atlassian Design System Date and time:
  https://atlassian.design/foundations/content/date-time
- PatternFly Timestamp design guidelines:
  https://www.patternfly.org/components/timestamp/design-guidelines/
- Nielsen Norman Group 10 Usability Heuristics:
  https://www.nngroup.com/articles/ten-usability-heuristics/
- Nielsen Norman Group Designing Empty States in Complex Applications:
  https://www.nngroup.com/articles/empty-state-interface-design/
- Nielsen Norman Group Progressive Disclosure:
  https://www.nngroup.com/articles/progressive-disclosure/
- Carbon Design System Empty States:
  https://carbondesignsystem.com/patterns/empty-states-pattern/
- Material Design 3 Lists:
  https://m3.material.io/components/lists/overview
- W3C WCAG 2.4.6 Headings and Labels:
  https://www.w3.org/WAI/WCAG22/Understanding/headings-and-labels.html
- W3C WCAG 3.2.4 Consistent Identification:
  https://www.w3.org/WAI/WCAG21/Understanding/consistent-identification.html
- Apple Human Interface Guidelines Writing:
  https://developer.apple.com/design/human-interface-guidelines/writing
- Apple NSWorkspace `activateFileViewerSelecting`:
  https://developer.apple.com/documentation/appkit/nsworkspace/activatefileviewerselecting%28_%3A%29
- Visual Studio Code file navigation and native file-manager reveal actions:
  https://code.visualstudio.com/docs/editing/userinterface
- IntelliJ IDEA file navigation and `Open in | Finder`:
  https://www.jetbrains.com/help/idea/file-navigation.html
- Firefox saved-download actions and Show in Finder:
  https://support.mozilla.org/kb/where-find-and-manage-downloaded-files-firefox
- libusb:
  https://libusb.info/
- libusb API:
  https://libusb.sourceforge.io/api-1.0/group__libusb__dev.html
- libusb USB descriptors:
  https://libusb.sourceforge.io/api-1.0/group__libusb__desc.html
- USB-IF Defined Class Codes:
  https://www.usb.org/defined-class-codes
- Microsoft USB standard descriptors:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/standard-usb-descriptors
- Zephyr USB CDC ACM:
  https://docs.zephyrproject.org/latest/services/connectivity/usb/device_next/cdc_acm.html
- Textual Footer:
  https://textual.textualize.io/widgets/footer/
- Textual input binding display:
  https://textual.textualize.io/guide/input/
- Charm Bubble Tea Bubbles help/key components:
  https://github.com/charmbracelet/bubbles
- Lazygit footer/help UX reference:
  https://www.bwplotka.dev/2025/lazygit/
- GitHub Releases:
  https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
- GitHub Actions manually running a workflow:
  https://docs.github.com/actions/managing-workflow-runs/manually-running-a-workflow
- GitHub Actions triggering a workflow from a workflow:
  https://docs.github.com/actions/using-workflows/triggering-a-workflow
- GitHub automatically generated release notes:
  https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes
- GitHub CLI `gh release create`:
  https://cli.github.com/manual/gh_release_create
- Keep a Changelog:
  https://keepachangelog.com/en/1.1.0/
- Homebrew libusb formula:
  https://formulae.brew.sh/formula/libusb
- Homebrew pkgconf formula:
  https://formulae.brew.sh/formula/pkgconf
- Homebrew taps:
  https://docs.brew.sh/Taps
- Homebrew tap creation and maintenance:
  https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap
- Homebrew Formula Cookbook:
  https://docs.brew.sh/Formula-Cookbook
- Homebrew Bottles:
  https://docs.brew.sh/Bottles
- Homebrew Cask Cookbook:
  https://docs.brew.sh/Cask-Cookbook
- Apple Developer ID and Gatekeeper:
  https://developer.apple.com/developer-id/
- Apple notarization workflow:
  https://developer.apple.com/documentation/security/customizing-the-notarization-workflow
- Apple notarizing macOS software:
  https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- rusb:
  https://docs.rs/rusb
- ratatui:
  https://docs.rs/ratatui/latest/ratatui/
- crossterm:
  https://docs.rs/crossterm/latest/crossterm/
- clap:
  https://docs.rs/clap/latest/clap/
- XDG Base Directory Specification:
  https://specifications.freedesktop.org/basedir/latest/
- OWASP Logging Cheat Sheet:
  https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html
- NIST Log Management:
  https://csrc.nist.gov/projects/log-management
- Command Line Interface Guidelines, Configuration:
  https://clig.dev/#configuration
- toml:
  https://docs.rs/toml/latest/toml/
- tracing:
  https://docs.rs/tracing/latest/tracing/
- portable-pty:
  https://docs.rs/portable-pty/latest/portable_pty/
- ctrlc:
  https://docs.rs/ctrlc/latest/ctrlc/
- 3GPP TS 27.007:
  https://www.3gpp.org/dynareport/27007.htm
- 3GPP TS 27.005:
  https://www.3gpp.org/DynaReport/27005.htm
- TOML v1.0.0:
  https://toml.io/en/v1.0.0
- XDG Base Directory Specification:
  https://specifications.freedesktop.org/basedir/
- Reference macOS/PyUSB Onyx article:
  https://zenn.dev/takao2704/articles/eg25-g-pyusb-pty-at-console
- Reference repository:
  https://github.com/takao2704/soracom-onyx-at-console
