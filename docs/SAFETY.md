# Safety Guide

AT commands can read sensitive identifiers and change modem state. This guide
explains what operators must review before USB access and what `atctl` must
confirm, mask, log, or block by default.

## Risk Levels

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
```

## Before USB Access

- Safe presets may execute directly.
- Sensitive presets may execute directly but output and logs must be masked by
  default.
- Write presets require confirmation.
- Persistent presets require stronger confirmation.
- Dangerous presets are never automatic. If visible in CLI, TUI, or PTY
  workflows, they require exact typed risk confirmation before USB access.
- Direct `atctl send` commands use the same risk model.
- Sequence execution uses the same risk model. Effective Sequence risk must
  account for declared risk, step command classification, prompt/body behavior,
  payload sensitivity, parameter sensitivity, and known side effects.
- File preset add-ons must declare risk, but declared risk is not the only safety
  input. Execution must use effective risk derived from declared risk and
  command classification.
- Sequence add-ons must declare risk, but declared risk is not the only
  safety input. Execution must use effective risk derived from declared risk,
  step classification, and parameter sensitivity.
- User-declared risk must never downgrade classifier output.
- The safety model applies across production AT execution surfaces. TUI, CLI
  `send`, CLI `preset run`, CLI `sequence run`, and PTY bridge may present
  different prompts or controls, but they must not weaken or silently omit
  required risk classification, confirmation, masking, or raw diagnostic export
  behavior.

## APN Commands

APN-changing commands may exist as presets or templates, but must never run
automatically.

Example:

```text
AT+CGDCONT=1,"IP","soracom.io"
```

This command requires explicit user selection and confirmation.

APN workflows are part of the product goal because users need to verify that a
modem, SIM, and data configuration can actually be used. APN commands must be
made safe through explicit file preset identity, risk classification, masking,
and confirmation, not by removing APN workflows from presets.

PDP authentication commands require the same treatment. `AT+CGAUTH?` is a
read-style diagnostic, but its response can include APN authentication
usernames and passwords, so it is sensitive and must be masked by default.
`AT+CGAUTH=...` changes PDP authentication settings and must remain write-risk.
If the command string itself includes a username or password, saved command
history and session logs must mask those credential fields.

## Connectivity Diagnostic Runtime Changes

Some standard AT commands are useful during manual troubleshooting but still
change modem runtime behavior:

```text
AT+COPS=3,2
AT+COPS=0
AT+CREG=2
AT+CGREG=2
AT+CEREG=2
AT+CEREG=3
AT+CEREG=5
```

These commands must be classified as write-risk and require confirmation before
USB access. They may change operator selection mode or registration reporting
mode. They are acceptable as explicit presets because they help a human inspect
operator identity, cell location, and registration rejection details, but they
must not be treated as automatic diagnostics.

## SMS Safety

SMS readiness checks may exist as ordinary one-shot presets when they are
read/test commands, for example:

```text
AT+CSMS?
AT+CMGF?
AT+CPMS?
```

SMS sending and other commands that require a prompt, message body, Ctrl-Z, or
similar multi-step interaction must not be treated as ordinary one-shot
commands. They must run through Sequence handling or through the PTY bridge's
prompt-capable manual path.

Standard SMS workflows are product-provided actions. Operators do not need to
author workflow definitions before using standard SMS checks. User-authored
workflow definitions remain an extension point for additional, special,
project-local, or verification workflows. Standard multi-step
operations must still keep the required risk confirmation and masking behavior.

Product-provided actions, repository-managed examples, and user-authored
extensions keep separate origins and review responsibility. After loading and
validation, their applicable execution behavior uses the same safety contract:
risk classification cannot be downgraded by a file, confirmation requirements
are preserved, normal output and logs stay masked by default, and raw diagnostic
export remains an explicit acknowledged action.

Repository-managed examples and user-authored extensions are external
definitions. They are not loaded from a default config directory during normal
startup. The operator must explicitly pass `--preset-file`, `--preset-dir`,
`--sequence-file`, or `--sequence-dir` for the current invocation. Loading an
external definition does not send AT commands by itself, but running a loaded
item may read sensitive values, change modem state, send SMS, or transmit
network payloads. `atctl` validates format, duplicate names, masking, and
effective risk; it does not certify that an external definition is appropriate
for the current device, SIM, network, or endpoint. CLI list output includes
source path review fields for external definitions. CLI run surfaces print the
source label, file path, and review notice before USB access when an external
definition is executed, including non-interactive `--yes --risk-ack <risk>`
runs.

SMS send Sequences must treat the recipient and message body as sensitive. SMS
reply Sequences must treat the extracted sender and reply body as sensitive.
The TUI and CLI confirmation path must show active input/review values before
USB access so the operator can confirm what will be sent. This active input and
pre-send review display is the intentional exception to normal masking;
Response output, saved output, session logs, history, and JSON output remain
masked by default. `+CMGS` followed by `OK` is evidence that the SMS submit
operation was accepted by the modem/network path; it must not be documented as
proof that the destination handset displayed the message.

SMS receive Sequences must treat sender numbers and message bodies as
sensitive. If a receive/list/read operation changes message status, notification
routing, storage selection, or unread/read flags, the Sequence must be
confirmation-required and must not silently delete messages. `REC READ` and
`REC UNREAD` values from commands such as `AT+CMGL` are modem message status
values, not product actions. A Sequence that reads a specific SMS with
`AT+CMGR=<index>` must be write-risk because the modem may mark an unread
message as read. Supported SMS bodies must be decoded before masking; normal
output must hide decoded body values by default. Unmasked foreground display
may show decoded SMS body values when the user explicitly requests it. Raw
diagnostic export remains a separate acknowledged raw modem exchange and may
contain modem-returned encoded body bytes.
The SMS storage index used by read and reply Sequences must be presented as a
value obtained from modem SMS listing output, such as `+CMGL`, rather than as an
unexplained integer. The value must be used exactly as returned by the modem;
atctl must not assume or adjust whether a modem starts SMS storage numbering
from 0 or 1. TUI candidate rows should label this as `storage=<index>` so it is
not confused with candidate-row pagination.

SMS reply-by-index is not a fresh recipient-supplied send. It reads the
original SMS storage index, derives the reply destination from the returned
sender, and then performs a standard `AT+CMGS` submit to that sender. Full
3GPP reply-path behavior must not be claimed unless the product explicitly
implements the required reply-path fields and routing behavior.

## Data-Send Sequences

Standard AT commands can inspect attach, PDP context, PDP address, and
connection details, but portable external TCP socket sending is not provided by
the standard one-shot AT command set used by the product presets. External
data-send checks therefore require vendor-specific Sequence definitions unless
a future approved specification defines another product surface.

Vendor-specific socket Sequences, such as Quectel TCP/IP checks, must be
explicit about what was verified:

- socket open evidence such as `+QIOPEN: <id>,0`;
- module accepted payload evidence such as `SEND OK`;
- TCP peer acknowledgement evidence such as `AT+QISEND=<id>,0` counters;
- end-to-end application evidence such as non-empty `QIRD` response data or
  remote endpoint logs.

The Sequence must not treat `OK` after a vendor socket-open command as complete
network success when the vendor command reports final socket status through a
later URC. `SEND OK` must not be described as remote application receipt.
For fixed-length TCP payload entry, repository-managed TCP examples must send
only the declared payload bytes and must not append SMS-style Ctrl-Z. They also
must not treat `+QISEND:` counters with remaining unacknowledged payload bytes
as a successful send condition. Repository-managed ping examples use
`AT+QPING`, must wait for `+QPING:` result lines, and must not treat terminal
`OK` alone as successful reachability when no successful `+QPING:` reply or
summary is parsed.
TCP Sequence inputs must distinguish user-entered payload/destination values
from modem-dependent values such as PDP context ID and socket connect ID. When
defaults are provided for modem-dependent values, add-on Sequence definitions
should use product-known candidate assistance only for standard values the
product can parse generically, such as PDP contexts from `AT+CGACT?` and
`AT+CGDCONT?`. Vendor-specific checks such as Quectel socket state must remain
explicit add-on commands or Sequences. Selecting a candidate action must use
the normal command or Sequence execution path; it must not bypass risk
classification, confirmation, masking, timeout, Response transcript, or
logging behavior. Candidate actions remain explicit refresh/load actions even
after same-session candidates are visible; opening a modal must not perform
hidden modem I/O to make candidates look current.
During confirmed Quectel TCP/IP Sequence execution, PDP context activation must
be state-aware: check `AT+QIACT?`, reuse the selected context when it is already
active, and send `AT+QIACT=<contextID>` only when activation is needed. If a
Sequence opens a product-managed socket and later fails, configured cleanup such
as `AT+QICLOSE=<connectID>` must be visible in the Response transcript and must
not hide the original failed step reason.
`AT+QISEND=<id>,0` counters must be presented as sent/acknowledged/
unacknowledged byte evidence, not application processing proof. `+QIRD: 0`
means no buffered receive data and must not be presented as response evidence.

SORACOM-specific ping and TCP examples must stay separate from generic Quectel
examples. The SORACOM ping example uses `pong.soracom.io` as SORACOM network
reachability evidence only. Unified Endpoint TCP uses
`unified.soracom.io:23080` or its documented `uni.soracom.io` alias. Beam TCP
is configuration-dependent and must not be treated as a default basic
connectivity check. For SORACOM TCP forwarding, one TCP write must not be
treated as one cloud message; use Soracom Binary Format v1, an HTTP entry
point, or application-layer framing when message boundaries matter.

## Long-Running Read/Test Commands

Read/test commands may be safe even when they take longer than ordinary reads.
`AT+COPS=?` is a standard available-operator test command used during carrier
diagnosis and may take longer because it can scan available carrier / RAT
options.

Long runtime is not by itself a write risk. Known long-running safe presets may
carry a preset timeout hint, and users can still provide an explicit longer
timeout when needed:

```sh
atctl preset run available-operators --timeout 240
```

## Diagnostic Error Reporting Commands

Diagnostic read commands such as `AT+CEER`, `AT+CMEE?`, and `AT+CPAS` may be
ordinary safe presets when they only read the latest failure report, error
reporting mode, or modem activity status.

Commands that change error reporting behavior are not read-only diagnostics.
`AT+CMEE=2` is useful because it enables verbose `+CME ERROR` reporting, but it
changes modem command-session behavior and must remain a write-risk command
that requires confirmation.

## Modem Functionality and Power Commands

Commands such as `AT+CFUN=0`, `AT+CFUN=1`, `AT+CFUN=1,1`, and vendor-specific
power-down commands are not ordinary diagnostic commands. They may detach from
the network, restart registration, disrupt PDP state, restart the modem, power
down the module, or drop the current USB/AT session.

Built-in modem functionality presets use the `modem` category and classify
state-changing `AT+CFUN=...` commands as dangerous. `AT+QPOWD` is
Quectel-specific and belongs in the Quectel file preset example, not in
vendor-neutral product presets.

The user must deliberately select these commands and type the exact risk level
before sending. The implementation must not describe these commands as generic
troubleshooting checks or imply that they are safe diagnostics for users who do
not understand their side effects.

See `docs/PRESETS.md` for the product preset list, file preset format, and
standard/vendor command reference links.

## User Preset Files

File presets and repository-managed file preset examples must follow the same
safety model:

- File presets are loaded only from explicit per-invocation `--preset-file` or
  `--preset-dir` locations.
- Every preset declares risk.
- The command classifier still runs.
- The effective risk preserves the stricter enforcement outcome.
- The preset set title and source path are shown in CLI listings. In the TUI,
  the same title is shown as a non-selectable source group header or
  `Source: <title>` detail when file presets are visible.
- Duplicate preset names fail instead of silently overriding.
- Vendor-specific commands may be loaded, but unknown or unsafe command shapes
  must remain protected by masking and confirmation.
- Vendor-specific read-style diagnostics may be made safe or sensitive only
  when the exact command form is known. Adjacent configuration-changing command
  forms must not inherit that lower risk.

## Sequence Add-on Files

User-authored Sequence files and repository-managed example Sequence files must
follow the same safety model:

- Sequence files are loaded only from explicit per-invocation `--sequence-file`
  or `--sequence-dir` locations.
- Every Sequence declares risk.
- Every step command is classified.
- The effective Sequence risk preserves the stricter enforcement outcome from
  declared risk, step classification, payload sensitivity, parameter
  sensitivity, and known side effects.
- Duplicate Sequence names fail instead of silently overriding.
- The Sequence set title and source path are shown in CLI listings.
- Sensitive Sequence parameters such as phone numbers, SMS bodies, payloads,
  APN credentials, usernames, passwords, and tokens are masked by default.
- Required values such as SMS storage index, PDP context ID, socket connect ID,
  and read length include defaults, candidate names, or resolution hints when
  the product can identify where the value comes from.
- Candidate-backed values use in-modal TUI candidates when known explicit
  same-session results are available. SMS candidate sender and body preview
  follow the same foreground output-masking mode as other TUI Response material.
  SMS candidate rows label the modem-returned storage value as `storage=<index>`
  and do not normalize storage numbering.
- Opening a TUI `Run Sequence` modal must not silently execute `AT+CMGL`,
  `AT+CMGR`, PDP checks, socket checks, or endpoint checks to populate
  candidates. Candidate rows may be reused only from the current TUI session's
  explicitly executed command or Sequence result, and the modal must identify
  that source. When the product knows an acquisition action for the candidate
  source, the action remains selectable as an explicit refresh/load operation
  even after candidates are visible. If the candidate action itself is
  confirmation-required, the same modal must request the risk word before USB
  access. If that action fails, the failure is an action failure with full
  detail in Response, not a completed failure of the selected Sequence body.
- Confirmation-required Sequences must keep the risk instruction and current
  `Input:` line visible in the `Run Sequence` modal. Long values or review
  details must be summarized, clipped, or made explicitly scrollable before
  they hide the action needed to run or cancel.
- Vendor-specific Sequences may be loaded, but unknown command shapes and
  state-changing command forms remain protected by masking and confirmation.

## Direct Send Policy

Direct arbitrary commands such as:

```sh
atctl send 'AT+CGDCONT=1,"IP","soracom.io"'
```

must be classified before execution.

Read/test commands:

- Plain read/test commands may run without confirmation and may print plain
  output.
- Read/test commands that expose sensitive identifiers or credentials may run
  without confirmation, but output and logs must be masked by default.
- Unknown read/test commands may run without confirmation, but must be treated
  as sensitive by default.

Change commands:

- Write, change, delete, persistent, dangerous, and non-read/test unknown
  commands require explicit confirmation.
- In an interactive terminal, direct `send` confirmation shows the normalized
  command, classified risk, and classifier reason.
- The user must type the exact classified risk level, such as `write`,
  `persistent`, or `dangerous`, before the command is sent.
- If confirmation is required but standard input is not a terminal, the command
  fails before USB access.
- Direct-send implementation must include a maintained risk-pattern table for
  known dangerous, persistent, write-risk, and vendor-specific diagnostic
  command forms.

Automation:

```sh
# Rejected because --yes alone does not state what risk was accepted.
atctl send 'AT+CFUN=0' --yes

# Accepted only if the classifier resolves the command as dangerous.
atctl send 'AT+CFUN=0' --yes --risk-ack dangerous
```

## PTY Bridge Policy

`atctl bridge --symlink <PATH>` is a compatibility interface for terminal tools
such as `screen` and `cu`. It must not become a safety bypass.

- Safe and sensitive commands may run from the PTY without confirmation.
- Sensitive output remains masked by default.
- Write, persistent, dangerous, and unknown commands require an exact typed
  risk acknowledgement from the PTY client before sending.
- Raw logging, session-wide raw mode, and session-level
  abort/reconnect/resync are separate designs and must not be silently added to
  the PTY bridge.
- Multiple external clients opening the same PTY symlink at the same time are
  not a supported workflow because input can interleave.

`--yes` disables the interactive prompt only when paired with
`--risk-ack <risk>`. The risk acknowledgement must match the classified command
risk, otherwise `atctl` must fail before sending the command to the modem.

## Default Masking

The following must be masked by default:

- IMSI
- ICCID
- IMEI
- MSISDN if present
- APN username/password if present
- PDP authentication username/password if present
- Long numeric identifiers likely to be subscriber or device identifiers

Masking applies to:

- CLI text output
- JSON output
- TUI response panes
- Session logs
- Command history

Raw display output requires `--no-mask`. Raw diagnostic export requires an
explicit user-selected destination and acknowledgement, such as
`--raw-log-file <PATH> --raw-log-ack raw-log`, and must not be enabled by
default.

TUI output is also masked by default. `atctl tui --no-mask` starts the
foreground TUI session with output masking off, and the TUI Controls pane can
disable output masking after exact typed `unmask` acknowledgement. Response copy
follows the visible Response display. Saved responses, history, and session
logs remain masked. TUI raw diagnostic export is a separate capture action that
requires entering a path and the exact `raw-log` acknowledgement before capture
starts.

## Logging

Logs must default to masked output.

If a command string contains a credential, the sensitive part of the command
string must also be masked in logs.
