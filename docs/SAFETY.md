# Safety Guide

AT commands can read sensitive identifiers and change modem state. This guide
explains what operators must review before USB access and what `atctl` must
confirm, mask, log, or block by default.

For preset and Sequence inventories, definition formats, loading, and execution
semantics, see [Presets and Sequences Reference](PRESETS.md). For diagnostic,
recovery, log-review, Response-export, and disposal procedures, see
[Troubleshooting](TROUBLESHOOTING.md).

## Execution Review

`atctl` does not reduce an operation to one risk label. Before execution it
derives these independent fields from the final command or Sequence:

- **Assessment**: `known` when the product has a rule for every operation
  boundary, otherwise `unknown`.
- **Effects**: observation, runtime-state change, persistent-state change,
  external communication, delete/reset/recovery, and service interruption.
- **Data handling**: sensitive input, sensitive output, and sensitive external
  disclosure.
- **Reversibility**: not applicable, immediately reversible, restart required,
  persistent until changed, irreversible, or not established.
- **Consequences**: a plain-language description of what may happen.
- **Confirmation**: required or not required, with reasons in structured
  output.

`unknown` does not mean that atctl detected every possible harmful effect. It
means complete semantics are not established. The confirmation therefore shows
the known fields, the fields that remain unestablished, and the possible
consequences before the operator chooses whether to proceed.

## Before USB Access

- Human CLI lists and TUI rows show the name and AT command or purpose preview.
  A row appends the complete ASCII tag `[confirm]` only when execution requires
  explicit confirmation. Mixed result sets reserve a fixed trailing lane for
  every row, align each displayed tag there, and leave it blank on
  confirmation-free rows; a result set with no confirmation-required item
  reclaims the space. The preview is shortened before the name and the tag is
  never split. TUI candidate-acquisition actions use the same derived tag and
  alignment rule. The TUI uses color only as reinforcement and retains the full
  bold text in no-color mode. The tag describes the next interaction, not risk
  severity. CLI `show` and TUI
  `d Details` expose the complete review without contacting the modem.
  Compact Status does not repeat selection-time review metadata.
- Known observations without a material effect may execute directly. Sensitive
  output and normal logs remain masked by default.
- Known material operations require `run` confirmation before USB access.
- Unknown operations require `unknown` confirmation, begin with
  `Effects are not fully known.`, and separate known information, possible
  consequences, and what is not established.
- Direct `atctl send`, presets, and Sequences use the same product-derived
  execution review. Preset and Sequence files do not declare or override it.
- `atctl preset show <NAME>` and `atctl sequence show <SEQUENCE>` expose the
  complete definition and execution review without USB access. Human-readable
  lists stay compact; `list --json` and `show --json` expose complete structured
  discovery data.
- A Sequence review covers step commands, conditional internal commands,
  payload writes, failure cleanup, sensitive parameters, and external
  transmission before its first USB write. After operator values and defaults
  are bound, the confirmation uses a newly derived review of those resolved
  operations rather than the unresolved list-row review. Values produced only
  by an earlier modem response remain identified as runtime-derived and each
  actual command is checked again at the write boundary.
- The safety model applies across production AT execution surfaces. TUI, CLI
  `send`, CLI `preset run`, CLI `sequence run`, and PTY bridge may present
  different prompts or controls, but they must not weaken or silently omit
  required review, confirmation, masking, or raw diagnostic export
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
derives the execution review, masks credentials, and requests confirmation before the
command reaches USB.

PDP authentication commands require the same treatment. `AT+CGAUTH?` is a
read-style diagnostic, but its response can include APN authentication
usernames and passwords, so it is sensitive and must be masked by default.
`AT+CGAUTH=...` changes stored PDP authentication settings and requires
confirmation.
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

These commands have a known runtime-state effect and require confirmation before
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
One-shot CLI `send`, `preset run`, and TUI AT-command input reject these
operations before USB access. The PTY bridge supports the reviewed SMS
command/payload/Ctrl-Z path; prompt framing it does not support, such as
fixed-length Quectel `AT+QISEND`, is also rejected before USB access and must use
a Sequence.

Standard SMS workflows are product-provided actions. Operators do not need to
author workflow definitions before using standard SMS checks. User-authored
workflow definitions remain an extension point for additional, special,
project-local, or diagnostic workflows. All origins keep separate review
responsibility while using the same product-derived review, confirmation,
masking, logging, and raw-export protections. See
[Sequence Workflows](PRESETS.md#sequence-workflows) for loading and origin
details and [Sequence Add-on Files](#sequence-add-on-files) for the external
definition safety checklist.

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
`AT+CMGR=<index>` requires confirmation because the modem may mark an unread
message as read. `AT+CMGL` also changes listed received-unread messages to
received-read according to 3GPP TS 27.005 and therefore uses the same
state-change confirmation. Supported SMS bodies must be decoded before masking; normal
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
- TCP peer acknowledgement evidence from an `AT+QISEND=<id>,0` step that
  explicitly declares `require_tcp_ack = true`;
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
explicit add-on commands or Sequences. Candidate acquisition must reference the
existing Preset or Sequence that owns the operation; it must not copy an AT
command into a second candidate-only definition. Selecting a provider must use
that item's normal execution path and must not bypass execution review,
confirmation, masking, timeout, Response transcript, logging, or raw diagnostic
export behavior. Candidate actions remain explicit refresh/load actions even
after same-session candidates are visible; opening a modal or encountering a
missing non-interactive CLI value must not perform hidden modem I/O to make
candidates look current.
During confirmed Quectel TCP/IP Sequence execution, PDP context activation must
be state-aware: check `AT+QIACT?`, reuse the selected context when it is already
active, and send `AT+QIACT=<contextID>` only when activation is needed. If a
Sequence opens a product-managed socket and later fails, configured cleanup such
as `AT+QICLOSE=<connectID>` must be visible in the Response transcript and must
not hide the original failed step reason.
Only an `AT+QISEND=<id>,0` step with `require_tcp_ack = true` may present its
three fields as TCP total/acknowledged/unacknowledged byte evidence. The same
response shape has UDP total/sent/unsent semantics, so an undeclared context
must remain modem response output rather than guessed TCP analysis. These
counters are not application processing proof. For a normal QIRD read,
`+QIRD: 0` means no buffered receive data and must not be presented as response
evidence. For `AT+QIRD=<connectID>,0`, three numeric fields are receive
counters and do not mean that payload data was returned.
Direct `AT+QIRD=<connectID>,<read_length>` data retrieval requires confirmation
because it advances the module's read/unread receive-buffer state. The
`AT+QIRD=<connectID>,0` size query and `AT+QIRD=?` capability query do not
retrieve buffered payload data and remain known observations.
`AT+QISEND=<connectID>,0` is likewise a known counter observation. A positive
send length starts a prompt/payload send and belongs in a Sequence so the
payload and its framing are reviewed before the first USB write.

Provider-specific Sequences must remain distinct from generic vendor examples
and must not overstate reachability, forwarding, message-boundary, or remote
application evidence. See
[Repository Example Sequences](PRESETS.md#repository-example-sequences) for the
endpoint-specific reference and
[SMS and Data-Send Sequence Checks](TROUBLESHOOTING.md#sms-and-data-send-sequence-checks)
for the operator result checks.

## Long-Running Read/Test Commands

Known observations may take longer than ordinary reads without gaining a
state-changing effect.
`AT+COPS=?` is a standard available-operator test command used during carrier
diagnosis and may take longer because it can scan available carrier / RAT
options.

Long runtime is not by itself an operation effect. Known long-running presets
may carry a timeout hint, and an explicit timeout value does not change the
operation assessment.

See [Timeout](TROUBLESHOOTING.md#timeout) for the task-oriented recovery path
and [Product Presets](PRESETS.md#product-presets) for the preset reference.

## Diagnostic Error Reporting Commands

Diagnostic read commands such as `AT+CEER`, `AT+CMEE?`, and `AT+CPAS` may be
ordinary no-confirmation presets when they only read the latest failure report, error
reporting mode, or modem activity status.

Commands that change error reporting behavior are not read-only diagnostics.
`AT+CMEE=2` is useful because it enables verbose `+CME ERROR` reporting, but it
changes modem command-session behavior and requires confirmation.

See [Failure Reports and Verbose Errors](TROUBLESHOOTING.md#failure-reports-and-verbose-errors)
for the diagnostic procedure.

## Modem Functionality and Power Commands

Commands such as `AT+CFUN=0`, `AT+CFUN=1`, `AT+CFUN=1,1`, and vendor-specific
power-down commands are not ordinary diagnostic commands. They may detach from
the network, restart registration, disrupt PDP state, restart the modem, power
down the module, or drop the current USB/AT session.

Built-in modem functionality presets use the `modem` category and review
state-changing `AT+CFUN=...` commands as runtime/service-interruption effects;
`AT+CFUN=1,1` also has a reset effect. `AT+QPOWD` is
Quectel-specific and belongs in the Quectel file preset example, not in
vendor-neutral product presets.

Select these commands only when the listed side effects are acceptable, and type
`run` before sending. They are state-changing operations, not
generic read-only diagnostics.

See [Product Presets](PRESETS.md#product-presets) for the complete preset
reference and [Modem Functionality Changes](TROUBLESHOOTING.md#modem-functionality-changes)
for the recovery procedure.

## User Preset Files

File presets and repository-managed file preset examples must follow the same
safety model:

- File preset locations are provided for each invocation through
  `--preset-file` or `--preset-dir`.
- A preset contains command, category, and optional timeout metadata; it does
  not declare risk.
- The product derives the execution review from the command.
- The preset set title and source path are shown once for each file group in
  human-readable CLI listings and remain structured in JSON. In the TUI,
  the same title is shown as a non-selectable source group header or
  `Source: <title>` detail when file presets are visible.
- Duplicate preset names fail instead of silently overriding.
- Vendor-specific commands may be loaded, but unknown operations and known
  material operations remain protected by masking and confirmation.
- Vendor-specific observation forms are treated as known only when the exact
  command form has a product rule. Adjacent configuration-changing forms do not
  inherit that assessment.

## Sequence Add-on Files

Review user-authored and repository-managed Sequence files before running them:

- Provide each Sequence file or directory through `--sequence-file` or
  `--sequence-dir` for the current invocation.
- A Sequence does not declare risk. The product reviews every step, conditional
  internal command, payload, and failure cleanup operation.
- The combined review preserves every effect and data-handling dimension,
  parameter sensitivity, reversibility, and unknown boundary.
- Duplicate Sequence names fail instead of silently overriding.
- The Sequence set title and source path are shown once for each file group in
  human-readable CLI listings and remain structured in JSON.
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

must be reviewed before execution.

Read/test commands:

- Known observations may run without confirmation.
- Known observations that expose sensitive identifiers or credentials still
  run with normal masking enabled by default.
- Unknown read/test syntax remains `unknown`; syntax alone does not prove that
  the operation has no state-changing effect.

Change commands:

- Known commands with material effects and all unknown commands require explicit
  confirmation.
- In an interactive terminal, direct `send` confirmation shows the normalized
  command and structured execution review.
- The user types `run` for a known material operation or `unknown` for an
  unknown operation before the command is sent.
- If confirmation is required but standard input is not a terminal, the command
  fails before USB access.
- `atctl` reviews product-known standard and vendor-specific command forms before
  execution. Commands not covered by those rules remain unknown.

For non-interactive execution, use `--yes` for a known material operation:

```sh
atctl send 'AT+CFUN=0' --yes
```

For an unknown operation, both flags are required:

```sh
atctl send 'AT+VENDOR=1' --yes --allow-unknown
```

`--allow-unknown` without `--yes` fails, and `--allow-unknown` is rejected for a
known operation. This makes unknown authorization explicit without claiming a
specific effect that atctl cannot establish.

## PTY Bridge Policy

`atctl bridge --symlink <PATH>` is a compatibility interface for terminal tools
such as `screen` and `cu`. It must not become a safety bypass.

- Known observations may run from the PTY without confirmation; sensitive output
  remains masked by default.
- Known material operations require `run`; unknown operations require
  `unknown` before sending.
- For supported prompt/body operations, the PTY bridge collects and reviews the
  complete command and payload before sending either one.
- Raw diagnostic export requires its own explicit destination and
  acknowledgement. Stopping or restarting the bridge is required after a USB
  transport failure or desynchronization.
- Multiple external clients opening the same PTY symlink at the same time are
  not a supported workflow because input can interleave.

The PTY bridge is interactive and does not use `send`'s `--yes` or
`--allow-unknown` flags.

## Default Masking

The following must be masked by default:

- IMSI
- ICCID
- IMEI
- MSISDN if present
- APN username/password if present
- PDP authentication username/password if present
- SIM PIN/PUK and other known credential fields if present
- SMS sender and recipient numbers, including short codes
- SMS message bodies and Sequence values declared sensitive
- Application tokens and sensitive payloads
- Long numeric identifiers likely to be subscriber or device identifiers

Known fields are classified by their AT-command meaning, not by digit count.
For example, a short sender such as `901001` is masked as `******`, while an SMS
storage index or `+CMGS` message reference remains visible. Quotes and protocol
punctuation remain visible so the response structure is still usable.

Unknown vendor output has a deliberate boundary: short tokens are not guessed
to be phone numbers without a known field meaning. The conservative long-number
fallback still applies. This avoids both disclosing product-known short phone
values and hiding short signal values, indexes, context IDs, ports, counts, or
result codes needed for troubleshooting.

Masking applies to:

- CLI text output
- JSON output
- TUI response panes
- Session logs
- Command history

The execution path produces one masked form for these normal consumers. Saved
history and session logs retain that masked form; opening them later does not
reclassify the text without its original execution context.

Raw display output requires `--no-mask`. Raw diagnostic export requires an
explicit user-selected destination and acknowledgement, such as
`--raw-log-file <PATH> --raw-log-ack raw-log`, and must not be enabled by
default.

CLI direct-send, preset, and Sequence raw preflight, PTY bridge raw preflight,
the TUI raw-capture acknowledgement modal, and every raw JSONL header use this
canonical warning:

```text
This raw diagnostic export records transmitted and received modem bytes without masking. It may contain sensitive modem and subscriber identifiers; network data; APN or PDP authentication data; phone numbers and SMS messages; arbitrary transmitted or received payloads; credentials and tokens; and vendor-specific identifiers.
```

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

Normal history and session logs must use masked output. If a command contains a
credential or another known sensitive field, that field must also be masked in
the saved command material. Generated history and session logs remain masked
even when the foreground Response and an explicitly requested Response export
are unmasked.

Masked logs can still contain operationally sensitive metadata, including AT
commands, timestamps, device selection, structured execution review, duration,
and result status. Protect the normal log directory and copies. Retain them only
for the troubleshooting case and applicable organizational, legal,
contractual, or security policy, then remove material that is no longer needed.
`atctl` does not apply automatic retention, rotation, pruning, or deletion.

`--no-log` prevents new normal history and session logs only for the selected
invocation. It does not hide existing logs or disable a raw diagnostic export
that the operator separately starts with a destination and acknowledgement.
Response export is likewise an explicit copy operation, not a generated log.
Unmasked Response exports require the same protection and disposal care as
other unmasked diagnostic material.

Raw diagnostic exports remain outside the normal masked log lifecycle at the
operator-selected destination. They require explicit acknowledgement, stricter
protection, and separate disposal.

See [Review and Manage Logs](TROUBLESHOOTING.md#review-and-manage-logs) for XDG
paths, CLI and TUI review, Finder access, retention and deletion effects, and
[Export a Response](TROUBLESHOOTING.md#export-a-response) for the complete copy
and export procedures.
