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

- Command and Sequence rows show one risk label only: `[safe]`, `[sensitive]`,
  `[write]`, `[persistent]`, `[dangerous]`, or `[unknown]`. Masking state,
  expected effects, and confirmation instructions are shown separately.
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

Before running an APN-changing command, verify the target PDP context, APN,
authentication method, and credentials. `atctl` shows the definition source,
classifies the command, masks credentials, and requests confirmation before the
command reaches USB.

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
project-local, or diagnostic workflows. Standard multi-step
operations must still keep the required risk confirmation and masking behavior.

Product-provided actions, repository-managed examples, and user-authored
extensions keep separate origins and review responsibility. After loading and
validation, they use the same safety protections:
risk classification cannot be downgraded by a file, confirmation requirements
are preserved, normal output and logs stay masked by default, and raw diagnostic
export remains an explicit acknowledged action.

Repository-managed examples and user-authored extensions are external
definitions. Provide their location with `--preset-file`, `--preset-dir`,
`--sequence-file`, or `--sequence-dir` for the current invocation. Loading a
definition does not send AT commands. Before running it, review the displayed
source, steps, destination values, declared risk, and effective risk. `atctl`
validates format and duplicate names and applies masking and risk enforcement,
but the operator remains responsible for confirming that the definition is
appropriate for the current device, SIM, network, and endpoint.

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
connection details. External TCP socket checks use vendor-specific Sequence
definitions because the product's standard one-shot commands do not provide a
portable TCP send operation.

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

Select these commands only when the listed side effects are acceptable, and type
the exact risk level before sending. They are state-changing operations, not
generic read-only diagnostics.

See `docs/PRESETS.md` for the product preset list, file preset format, and
standard/vendor command reference links.

## User Preset Files

File presets and repository-managed file preset examples must follow the same
safety model:

- File preset locations are provided for each invocation through
  `--preset-file` or `--preset-dir`.
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

Review user-authored and repository-managed Sequence files before running them:

- Provide each Sequence file or directory through `--sequence-file` or
  `--sequence-dir` for the current invocation.
- Every Sequence declares risk.
- Every step command is classified.
- The effective Sequence risk preserves the stricter enforcement outcome from
  declared risk, step classification, payload sensitivity, parameter
  sensitivity, and known side effects.
- Duplicate Sequence names fail instead of silently overriding.
- The Sequence set title and source path are shown in CLI listings.
- Sensitive Sequence parameters such as phone numbers, SMS bodies, payloads,
  APN credentials, usernames, passwords, and tokens are masked by default.
- For required values such as SMS storage index or PDP context ID, use the
  displayed default, source hint, or candidate action instead of guessing.
- Candidate actions are explicit operations. Opening `Run Sequence` does not
  silently read SMS, PDP, socket, or endpoint state. Candidate actions retain
  the normal confirmation, masking, timeout, Response, and logging protections.
- SMS candidate sender and body previews follow the current foreground masking
  mode. The modem-returned index is shown as `storage=<index>` and is not
  renumbered.
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
- `atctl` classifies known dangerous, persistent, write-risk, and
  vendor-specific diagnostic command forms before execution.

For non-interactive execution, pair `--yes` with the exact classified risk:

```sh
atctl send 'AT+CFUN=0' --yes

atctl send 'AT+CFUN=0' --yes --risk-ack dangerous
```

The first command fails before USB access because `--yes` alone does not state
which risk was accepted. The second proceeds only when the classified risk is
`dangerous`.

## PTY Bridge Policy

`atctl bridge --symlink <PATH>` is a compatibility interface for terminal tools
such as `screen` and `cu`. It must not become a safety bypass.

- Safe and sensitive commands may run from the PTY without confirmation.
- Sensitive output remains masked by default.
- Write, persistent, dangerous, and unknown commands require an exact typed
  risk acknowledgement from the PTY client before sending.
- Raw diagnostic export requires its own explicit destination and
  acknowledgement. Stopping or restarting the bridge is required after a USB
  transport failure or desynchronization.
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
and explicit Response export follow the visible Response display. When the
displayed Response has a distinct unmasked form, copy requires exact `copy`
acknowledgement and export requires destination selection followed by exact
`export` acknowledgement before a file is created. Generated history and
session logs remain masked. TUI raw diagnostic export is a separate capture action that
requires entering a path and the exact `raw-log` acknowledgement before capture
starts.

## Logging

Logs must default to masked output.

If a command string contains a credential, the sensitive part of the command
string must also be masked in logs.

Normal command history and session logs are written under the XDG state
directory by default:

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/<timestamp>.session.log
```

To use another state directory for one invocation, set `XDG_STATE_HOME` to a
non-empty absolute path. `atctl` appends its own `atctl` directory:

```sh
env XDG_STATE_HOME="$HOME/Documents" atctl send AT
env XDG_STATE_HOME="$HOME/Documents" atctl logs list
```

Use `--no-log` to prevent creation of new masked history and session logs for
one `send`, `preset run`, `sequence run`, or TUI invocation. Existing saved
logs remain available for review. `--no-log` does not disable a raw diagnostic
export that the operator explicitly starts with a selected destination and the
required acknowledgement.

Masked logs can still contain operationally sensitive metadata, including AT
commands, timestamps, device selection, risk, duration, and result status.
Protect the normal log directory and any copies even though response values and
sensitive command fields are masked.

`atctl` retains normal history and session logs until the operator removes them.
It does not apply an automatic retention period, rotation policy, pruning, or
deletion. Retain logs for the period required by the troubleshooting case and
applicable organizational, legal, contractual, or security policy, then remove
logs and copies that are no longer required. Finish the current `atctl`
execution before moving a saved log to the Trash.

Use `atctl logs list` or the TUI Logs pane to review normal masked logs. In the
TUI, `Reveal in Finder` on a selected log identifies that exact file without
requiring Response review and without opening, moving, or deleting the file.
The same action remains available after the log is opened in Response.

Response export is an explicit copy operation, not a generated log. In the TUI,
`Export response...` shows the Response identity, UTF-8 text format, generated
file name, and masking state before opening a destination-folder chooser. The
chooser is shown on every export. A successful export retains the exact file for
`Reveal in Finder`; repeating the export creates another file and never
overwrites an existing file.

When the current Response is displaying values that differ from its masked
form, the action labels are `Copy unmasked response` and
`Export unmasked response...`. Copy requires exact acknowledgement `copy`.
Export shows the exact final path after folder selection and requires exact
acknowledgement `export`; cancellation or a mismatched acknowledgement creates
no file. This confirmation is separate from command risk classification and
does not add `MASKED`, `CONFIRM`, `PERSISTS`, `DANGER`, or `REVIEW` suffixes to
command rows.

`atctl send`, `atctl preset run`, and `atctl sequence run` accept
`--export-response <PATH>`. The target is validated before USB access, an
existing file is rejected, and normal stdout is unchanged. CLI export is masked
by default and follows `--no-mask` when the operator explicitly selects
unmasked foreground output. TUI export follows the Response masking mode that
is visible when export is selected. Unmasked exports can contain identifiers,
message bodies, payloads, or credentials and require the same protection and
disposal care as raw diagnostic material.

Generated history and session logs remain masked regardless of Response export.
Raw diagnostic export remains a separate, explicitly acknowledged operation.
Previously created files under `$XDG_STATE_HOME/atctl/responses/` are not
deleted or migrated automatically.

Raw diagnostic exports remain outside these normal masked lifecycles at the
destination the operator explicitly selected and require stricter protection
and separate disposal.
