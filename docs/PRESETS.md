# Presets and Sequences Reference

This document explains how `atctl` presents, loads, reviews, and runs one-shot
presets and multi-step Sequences.

Presets are not TUI-only. The same loaded preset set is used by:

```sh
atctl preset list
atctl preset run <NAME>
atctl tui
```

Presets represent one AT command line. Sequences represent multi-step AT
operations such as SMS send/read/reply checks and Quectel or SORACOM TCP/IP
socket checks. Both are selectable from the TUI executable-item surface.

Product-provided definitions ship with `atctl`. File definitions are loaded from
files or directories provided for the current invocation. After loading, both
origins use the same validation, risk handling, masking, logging, raw diagnostic
export, and execution behavior.

## Preset Sources

`atctl` has two preset sources:

- Product presets: product-provided standard workflow presets shipped by the
  program.
- File presets: TOML-defined presets loaded from files or directories provided
  for the current invocation.

Product presets are vendor-neutral where practical. Vendor-specific,
modem-specific, carrier-specific, and project-specific commands are provided as
file presets.
After loading, both preset kinds use the same command execution behavior and
duplicate-name rules. The source distinction remains available for CLI labels,
TUI source grouping, review responsibility, and troubleshooting context.

In the TUI, product presets are the default command rows. They are not shown
under a `Product presets` header in the normal command list. When file presets
are visible, their top-level TOML `title` is shown as a non-selectable source
group header, such as `SORACOM commands` or `Quectel commands`; the TUI does
not add an `Add-on:` prefix.

## Product Presets

Product presets are displayed in a curated workflow order, not alphabetical
order. The order follows connection check, modem identity, SIM, radio access
selection, operator visibility, registration, signal, PDP/APN readiness,
failure diagnostics, SMS readiness, modem functionality status/control, and
runtime control.

| Preset | Command | Categories | Risk | Purpose | Reference |
|---|---|---|---|---|---|
| `modem-response` | `AT` | `basic` | `safe` | Check that the modem responds to a minimal AT command. | ITU-T V.250 |
| `modem-info` | `ATI` | `identity` | `safe` | Read modem identification text. | ITU-T V.250 / vendor behavior |
| `manufacturer` | `AT+CGMI` | `identity` | `safe` | Read manufacturer. | 3GPP TS 27.007 |
| `model` | `AT+CGMM` | `identity` | `safe` | Read model. | 3GPP TS 27.007 |
| `firmware-revision` | `AT+CGMR` | `identity` | `safe` | Read firmware revision. | 3GPP TS 27.007 |
| `imei` | `AT+CGSN` | `identity` | `sensitive` | Read device identifier. | 3GPP TS 27.007 |
| `sim-pin-status` | `AT+CPIN?` | `sim` | `safe` | Check SIM readiness. | 3GPP TS 27.007 |
| `imsi` | `AT+CIMI` | `sim` | `sensitive` | Read subscriber identifier. | 3GPP TS 27.007 |
| `radio-stack` | `AT+WS46?` | `network` | `safe` | Read selected wireless data service stack. | 3GPP TS 27.007 |
| `radio-stack-capabilities` | `AT+WS46=?` | `network` | `safe` | Read supported wireless data service stack values. | 3GPP TS 27.007 |
| `current-operator` | `AT+COPS?` | `network` | `safe` | Read selected operator. | 3GPP TS 27.007 |
| `available-operators` | `AT+COPS=?` | `network` | `safe` | Scan visible operators and RATs. | 3GPP TS 27.007 |
| `operator-format-numeric` | `AT+COPS=3,2` | `network` | `write` | Set operator readout format to numeric PLMN for subsequent operator checks. | 3GPP TS 27.007 |
| `operator-auto-selection` | `AT+COPS=0` | `network` | `write` | Return operator selection to automatic mode. | 3GPP TS 27.007 |
| `circuit-registration` | `AT+CREG?` | `network` | `safe` | Check circuit registration. | 3GPP TS 27.007 |
| `gprs-registration` | `AT+CGREG?` | `network` | `safe` | Check GPRS registration. | 3GPP TS 27.007 |
| `eps-registration` | `AT+CEREG?` | `network` | `safe` | Check EPS/LTE registration. | 3GPP TS 27.007 |
| `enable-circuit-registration-detail` | `AT+CREG=2` | `network` | `write` | Enable detailed circuit registration reporting. | 3GPP TS 27.007 |
| `enable-gprs-registration-detail` | `AT+CGREG=2` | `network` | `write` | Enable detailed GPRS registration reporting. | 3GPP TS 27.007 |
| `enable-eps-registration-detail` | `AT+CEREG=2` | `network` | `write` | Enable detailed EPS registration reporting. | 3GPP TS 27.007 |
| `enable-eps-registration-cause` | `AT+CEREG=3` | `network` | `write` | Enable EPS registration detail including reject cause when provided. | 3GPP TS 27.007 |
| `enable-eps-registration-extended` | `AT+CEREG=5` | `network` | `write` | Enable extended EPS registration detail when supported. | 3GPP TS 27.007 |
| `signal-quality` | `AT+CSQ` | `signal` | `safe` | Read basic signal quality. | 3GPP TS 27.007 |
| `extended-signal-quality` | `AT+CESQ` | `signal` | `safe` | Read extended signal quality. | 3GPP TS 27.007 |
| `extended-signal-capabilities` | `AT+CESQ=?` | `signal` | `safe` | Read extended signal quality support. | 3GPP TS 27.007 |
| `pdp-contexts` | `AT+CGDCONT?` | `pdp`, `apn` | `safe` | Inspect PDP/APN context definitions. | 3GPP TS 27.007 |
| `pdp-auth-settings` | `AT+CGAUTH?` | `pdp`, `apn` | `sensitive` | Inspect PDP authentication settings. | 3GPP TS 27.007 |
| `pdp-auth-capabilities` | `AT+CGAUTH=?` | `pdp`, `apn` | `safe` | Read PDP authentication command support. | 3GPP TS 27.007 |
| `packet-attach` | `AT+CGATT?` | `network`, `pdp` | `safe` | Check packet service attach state. | 3GPP TS 27.007 |
| `active-pdp-contexts` | `AT+CGACT?` | `pdp` | `safe` | Check active PDP contexts. | 3GPP TS 27.007 |
| `pdp-addresses` | `AT+CGPADDR` | `pdp` | `safe` | Read PDP context IP address assignments. | 3GPP TS 27.007 |
| `pdp-address-capabilities` | `AT+CGPADDR=?` | `pdp` | `safe` | Read PDP address command support. | 3GPP TS 27.007 |
| `pdp-connection-details` | `AT+CGCONTRDP` | `pdp` | `safe` | Read PDP connection details. | 3GPP TS 27.007 |
| `extended-error-report` | `AT+CEER` | `diagnostics` | `safe` | Read the latest extended modem/network failure report when supported by the modem. | 3GPP TS 27.007 |
| `error-reporting-status` | `AT+CMEE?` | `diagnostics` | `safe` | Read whether extended mobile termination errors are enabled. | 3GPP TS 27.007 |
| `enable-verbose-errors` | `AT+CMEE=2` | `diagnostics` | `write` | Enable verbose `+CME ERROR` reporting for easier troubleshooting. | 3GPP TS 27.007 |
| `modem-activity-status` | `AT+CPAS` | `modem`, `diagnostics` | `safe` | Read whether the modem is ready, unavailable, unknown, or asleep. | 3GPP TS 27.007 |
| `sms-service-support` | `AT+CSMS?` | `sms` | `safe` | Check SMS service support. | 3GPP TS 27.005 |
| `sms-format` | `AT+CMGF?` | `sms` | `safe` | Check SMS format. | 3GPP TS 27.005 |
| `sms-storage` | `AT+CPMS?` | `sms` | `safe` | Check SMS storage selection. | 3GPP TS 27.005 |
| `modem-functionality` | `AT+CFUN?` | `modem` | `safe` | Read modem functionality level. | 3GPP TS 27.007 |
| `set-modem-minimum-functionality` | `AT+CFUN=0` | `modem` | `dangerous` | Change modem functionality to minimum mode. | 3GPP TS 27.007 |
| `set-modem-full-functionality` | `AT+CFUN=1` | `modem` | `dangerous` | Return modem functionality to full mode. | 3GPP TS 27.007 |
| `restart-modem` | `AT+CFUN=1,1` | `modem` | `dangerous` | Request modem restart. | 3GPP TS 27.007 |
| `disable-command-echo` | `ATE0` | `basic` | `write` | Disable command echo for the current AT session. | ITU-T V.250 |

`available-operators` declares `timeout_secs = 180` because `AT+COPS=?` can
take longer than ordinary reads.

The operator format, operator auto-selection, and registration detail presets
are useful during troubleshooting but change modem runtime behavior. They are
write-risk presets and require confirmation before USB access.

`pdp-auth-settings` can expose APN authentication usernames and passwords through
`+CGAUTH:` responses. It is sensitive, and saved output must remain masked by
default.

`extended-error-report` is useful after failed attach, registration, or PDP
activation attempts because `AT+CEER` returns the modem's latest extended
failure report when the modem implements it. `enable-verbose-errors` changes
the error reporting mode and is therefore a write-risk preset even though it is
used for diagnostics.

The `set-modem-minimum-functionality`, `set-modem-full-functionality`, and
`restart-modem` presets are modem state-changing operations, not diagnostic
reads. They can detach from the network, restart network registration, disrupt
PDP state, drop the current USB/AT session, or require the user to reconnect
and reselect the modem. They are visible as dangerous commands and require
exact typed risk confirmation before sending.

## File Preset TOML

File presets use human-editable TOML.

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

Required top-level fields:

- `title`: user-facing preset set label shown in CLI listings and used as the
  TUI source group header and `Source: <title>` detail when file presets are
  visible.

Optional top-level fields:

- `description`: human-readable description.

Required preset fields:

- `name`: unique preset name across all loaded presets.
- `command`: one AT command line.
- `risk`: declared risk, one of `safe`, `sensitive`, `write`, `persistent`,
  `dangerous`, or `unknown`.

Optional preset fields:

- `categories`: workflow categories such as `basic`, `identity`, `sim`,
  `network`, `pdp`, `apn`, `signal`, `sms`, `diagnostics`, or `modem`.
- `timeout_secs`: preset-specific timeout hint for known long-running commands.

The command classifier still runs for file presets. A TOML file cannot downgrade
the effective risk. For example, `AT+CFUN=1,1` remains dangerous even if a file
incorrectly declares a lower risk.

## Loading File Presets

File presets are external executable definitions. To use them for review, local
projects, or repository examples, provide their locations for the current
invocation:

```sh
atctl preset list --preset-dir ./presets
atctl preset list --preset-file ./presets/custom.toml
atctl preset run custom-modem-response --preset-file ./presets/custom.toml
atctl tui --preset-dir ./presets
```

The product presets remain loaded. Loading an external file does not send AT
commands by itself, but running loaded presets may read sensitive values,
change modem state, or affect the network. Review the file source, command
text, destination values, declared risk, and effective risk before running a
loaded preset. `atctl` validates the TOML shape, duplicate names, masking, and
effective risk, but it does not certify that an external definition is
appropriate for the current device, SIM, network, or endpoint.

`atctl preset list` shows both the preset set label and a trailing
`source-path` column. Product presets show `-`; file presets show the file path
that supplied the row. `atctl preset run` prints the source label, file path,
and review notice before USB access when a file preset is executed, including
non-interactive `--yes --risk-ack <risk>` runs.

Both explicit options are repeatable. Explicit files are loaded in command-line
order. Explicit directories are loaded in command-line order, and `.toml` files
inside each directory are loaded in deterministic lexicographic path order.
Duplicate preset names fail instead of silently overriding.

## Repository Examples

Repository-managed example file presets live under:

```text
examples/presets/
```

They are loaded with the same TOML loader used for user file presets:

```sh
atctl preset list --preset-dir examples/presets
atctl tui --preset-dir examples/presets
```

The Quectel example file contains Quectel-specific commands such as `AT+QCCID`,
`AT+QCSQ`, `AT+QNWINFO`, `AT+QENG`, `AT+QCFG?`, `AT+QINISTAT`, `AT+QPINC?`,
`AT+QSPN`, `AT+QLTS`, `AT+QMBNCFG="List"`, `AT+QCFG="nwscanmode"`,
`AT+QCFG="nwscanmode",0,1`, and `AT+QPOWD`.

The Quectel diagnostic presets are file presets because they are
vendor-specific. `AT+QMBNCFG="List"` is included only as a read-style MBN list
diagnostic. Other MBN operations such as selecting, deactivating, adding,
deleting, or auto-selecting MBN files are not ordinary safe presets.

`AT+QPOWD` is not a product preset because it is Quectel-specific. It is a
dangerous file preset in the Quectel example set. It may power down the module
and drop the current USB/AT session. After sending it, do not assume the current
TUI or PTY session remains usable.

The SORACOM example file contains SORACOM-specific APN setup. Carrier APN values
and default SORACOM authentication templates belong in file presets, not in
vendor-neutral product presets.

## Sequence Workflows

Sequences are named multi-step AT operations. They are used when a workflow
needs prompt waits, payload writes, delayed URCs, per-step timeouts, or a step
transcript.

The same loaded Sequence set is used by:

```sh
atctl sequence list
atctl sequence run <SEQUENCE>
atctl tui
```

Product-provided standard Sequences are ordinary product actions. Users do not
need to author TOML before using standard SMS send/read/reply checks. User
Sequence TOML is an extension point for additional, project-local, or
special-purpose operations.
Repository-managed example Sequences and user-authored Sequences are loaded
through the same Sequence definition path, but their origin remains visible and
does not turn them into product-provided standard Sequences.

In the TUI, product-provided standard Sequences are default Sequence rows. They
are not shown under a `Product Sequences` header in the normal list. When
user-authored or repository-managed Sequence definitions are visible, their
top-level TOML `title` is shown as a non-selectable source group header, such
as `Quectel Sequences`; the TUI does not add an `Add-on:` prefix.

Sequence add-ons are external executable definitions. To use repository-managed
examples or project-local definitions, provide their locations for the current
invocation:

```sh
atctl sequence list --sequence-file ./sequences/custom.toml
atctl sequence list --sequence-dir ./sequences
atctl sequence run custom-sequence --sequence-file ./sequences/custom.toml
atctl tui --sequence-dir ./sequences
```

When any `--sequence-file` or `--sequence-dir` flag is used, explicit Sequence
locations are added for that invocation. Product-provided standard Sequences
remain loaded. Loading a Sequence file does not send AT commands by itself, but
running loaded Sequences may change modem state, send SMS, or transmit network
payloads. Review the file source, steps, parameters, destination values,
declared risk, and effective risk before running a loaded Sequence.

`atctl sequence list` shows both the Sequence set label and a trailing
`source-path` column. Product Sequences show `-`; file Sequences show the file
path that supplied the row. `atctl sequence run` prints the source label, file
path, and review notice before USB access when a file Sequence is executed,
including non-interactive `--yes --risk-ack <risk>` runs.

## Sequence TOML Files

Sequence files use human-editable TOML.

```toml
title = "Custom Sequences"
description = "Optional description for humans."

[[sequences]]
name = "custom-sequence"
summary = "Short user-facing summary."
risk = "write"
categories = ["data"]
timeout_secs = 180
success_notes = [
  "This note appears in the final transcript and CLI JSON notes."
]

[[sequences.params]]
name = "payload"
label = "Payload"
required = true
sensitive = true
source = "user"
hint = "Enter the payload to send."

[[sequences.review]]
label = "Payload"
value = "{{payload}}"
sensitive = true

[[sequences.steps]]
id = "send-command"
send = "AT+EXAMPLE={{payload}}"
expect = "OK"
timeout_secs = 30
evidence = "OK means this step matched its expected modem response."
```

Required top-level fields:

- `title`: user-facing Sequence set label shown in CLI listings and used as the
  TUI source group header and `Source: <title>` detail when non-default
  Sequence sets are visible.

Optional top-level fields:

- `description`: human-readable description.

Required Sequence fields:

- `name`: unique Sequence name across all loaded Sequences.
- `summary`: concise purpose shown in listings, TUI rows, and execution review.
- `risk`: declared risk, one of `safe`, `sensitive`, `write`, `persistent`,
  `dangerous`, or `unknown`.
- at least one step.

Optional Sequence fields:

- `categories`: workflow categories such as `sms`, `data`, `network`, `pdp`,
  `apn`, or `diagnostics`.
- `timeout_secs`: Sequence-specific total timeout hint.
- `params`: required or optional values. Sensitive inputs must be marked.
  Values may also define `default`, `source`, `candidate`, and `hint` so TUI
  and CLI users can see where the value comes from before execution.
- `review`: active review items rendered from Sequence parameters. When no
  review items are defined, `atctl` reviews the supplied parameters by label
  before confirmation.
- `success_notes`: notes appended to the transcript and CLI JSON when a
  Sequence succeeds.
- `before_running`: concise prerequisite or confirmation notes shown before
  execution. Use this for human-facing add-on context, not as a hidden machine
  dependency model.

Parameter value-resolution fields:

- `default`: editable value prefilled before execution, such as `read_length =
  "1500"` or a Quectel socket connect ID of `"0"`.
- `source`: one of `user`, `default`, `modem`, `select`, `sequence`, `derived`,
  or `external`.
- `candidate`: optional product-known candidate source used when a parameter
  can be selected from explicitly obtained modem or Sequence output. Initial
  candidate names are `sms-message` and `pdp-context`.
- `hint`: concise instruction for how the operator confirms, selects, derives,
  or enters the value.

The same parameter metadata appears in TUI input and CLI missing-value errors,
so values such as PDP context ID, socket connect ID, and SMS storage index can
include a default, source, candidate list, or entry hint.

### Selecting Candidate Values in the TUI

For `source = "select"` or a parameter with `candidate`, the `Run Sequence`
dialog shows available values and their source. If values have not been loaded,
select the action shown in the dialog to obtain them. Opening the dialog alone
does not read the modem or contact a network endpoint.

Candidate actions use the normal risk confirmation, timeout, masking, Response,
and logging behavior. A failed candidate action is reported separately from the
selected Sequence. Manual entry remains available when the candidate list is
empty or does not contain the required value.

Repository-managed and project-local Sequences can use a named `candidate`
when `atctl` supports that candidate type. The currently supported names are
`sms-message` and `pdp-context`.

For SMS read/reply by storage index, candidates come from known `AT+CMGL` or
`sms-receive-check` rows obtained by an explicit same-session execution and
should include enough context to identify the message, such as index, status,
sender, timestamp, and a body preview respecting the current masking mode.
The candidate value is the SMS storage index returned by the modem. atctl does
not convert it between 0-based and 1-based numbering; the selected value is
sent back unchanged in `AT+CMGR=<index>`. TUI candidate rows label this value as
`storage=<index>` so it is not confused with candidate-row pagination.
`sms-reply-check` still derives the actual reply recipient from the `AT+CMGR`
sender returned during execution; the selected index only chooses which stored
message to read.

Sequence steps must be explicit. Prompt waits, payload writes, Ctrl-Z or ESC
terminators, URC waits, and response capture are part of the Sequence model and
must not be hidden in one concatenated AT command string.

Optional step fields:

- `label`: user-facing step label.
- `expect`: expected material in the normal response for a step whose final
  result code is enough to complete the step. `expect = "OK"` must be used only
  when `OK` itself is the completion condition.
- `expect_prompt`: prompt material that must appear before a payload write.
- `expect_urc`: result marker that can arrive after an initial success final
  result. Use this for commands that accept work with `OK` and then report the
  real result in later lines such as `+QIOPEN:` or `+QPING:`.
- `payload`: payload template to write after a prompt or as a payload step.
- `terminator`: `none`, `ctrl-z`, or `esc`.
- `timeout_secs`: step-specific timeout.
- `evidence`: note describing what the successful step proves. Normal text
  output renders this material under `Analysis:`, not as a literal `Evidence:`
  transcript line.

Semantic success flags must match the wait marker. A step with
`require_ping_success = true` must wait for `+QPING:` using `expect_urc`; using
only `expect = "OK"` is rejected because `AT+QPING` can return `OK` before the
ping result lines. A step with `require_tcp_ack = true` must read `+QISEND:`
counters before it can evaluate acknowledged and unacknowledged bytes.

Normal Sequence text transcripts render origin sections such as `Command:`,
`Modem response:`, `Decoded SMS:`, `Analysis:`, `Notes:`, and `Result:` as
separate blocks with one blank line between blocks.

Pre-send review is intentionally separate from normal Response and log output.
The review surface may show current typed values for sensitive items so the
operator can verify the destination and content before sending. Normal Response
output, Response exports, history, session logs, and JSON remain masked by
default unless the operator explicitly selects unmasked foreground output and
Response export.

## Logging

Preset and Sequence executions write masked history and session logs by
default. Use `--no-log` when the current execution must not create either
normal log artifact:

```sh
atctl preset run modem-info --no-log
atctl sequence run sms-receive-check --no-log
atctl tui --no-log
```

`--no-log` applies only to the current command or TUI session. It does not hide
existing logs and does not disable a raw diagnostic export explicitly requested
with an output path and acknowledgement. Normal log paths follow
`XDG_STATE_HOME`; see the [README logging section](../README.md#logs) for the
default paths and an override example.

## Standard Sequences

Product-provided standard Sequence targets:

- `sms-send-check`: send an SMS using standard 3GPP TS 27.005 commands. It
  reviews recipient and message body before USB access, uses write-risk
  confirmation, and records under `Notes:` / `Analysis:` that `+CMGS` plus
  `OK` is submit evidence rather than destination handset receipt proof.
- `sms-receive-check`: check received SMS material using standard 3GPP TS
  27.005 receive/list/read commands. Sender numbers and message bodies are
  sensitive. If the chosen receive method changes message status, notification
  routing, storage, or unread/read flags, the Sequence must be confirmation
  required and must not silently delete messages. Supported message bodies are
  decoded before masking so the raw encoded body is not treated as readable
  content.
- `sms-read-message`: read a specific SMS storage index. Because `AT+CMGR` can
  change an unread message to read state, this Sequence is write-risk and
  requires confirmation before USB access. Unmasked foreground output shows
  supported message bodies as decoded text, while normal output stays masked.
  Raw diagnostic export remains a separate acknowledged raw modem exchange and
  may contain the modem-returned encoded body bytes. The SMS storage index comes
  from SMS storage listing output such as `sms-receive-check` / `AT+CMGL`; it is
  not an arbitrary user-created number.
- `sms-reply-check`: read a specific SMS storage index, extract the sender from
  `AT+CMGR`, and submit the reply body to that sender using the standard SMS
  submit path. It reviews SMS storage index and reply body before USB access and
  records submit evidence under `Notes:` / `Analysis:`, not handset receipt
  proof. It must not be described as manual recipient-entry send. The reply
  destination is derived from the selected message sender during execution.

## Repository Example Sequences

Repository-managed example Sequences live under:

```text
examples/sequences/
```

They are loaded with the same Sequence loader used for explicit Sequence
add-on definitions:

```sh
atctl sequence list --sequence-dir examples/sequences
atctl tui --sequence-dir examples/sequences
```

The Quectel example demonstrates Quectel-specific TCP/IP data-send checks. It
uses commands such as `AT+QIACT?`, `AT+QIACT=<cid>`, `AT+QIOPEN=...`,
`AT+QISEND=...`, `AT+QIRD=...`, and `AT+QICLOSE=...`.
For repeated runs, the TCP Sequence checks `AT+QIACT?` first and reuses an
already active Quectel PDP context instead of blindly sending
`AT+QIACT=<cid>` again. If the socket opens and a later step fails, the example
Sequence runs the configured `AT+QICLOSE=<connectID>` cleanup command and keeps
that cleanup visible in the transcript.

Quectel data-send and ping Sequences are vendor-specific examples rather than
default vendor-neutral standard Sequences. The ping example uses `AT+QPING`
through the selected PDP context and reports received replies as IP reachability
evidence only. Ping success is not TCP socket, payload delivery, or application
processing proof. Ping steps
declare `expect_urc = "+QPING:"` and `require_ping_success = true`, so `atctl`
waits past the command-accepted `OK` for `+QPING:` result lines.
Terminal `OK` without a parsed successful `+QPING:` reply or summary is not
enough for `Result: OK`.
`OK` after `QIOPEN` is not enough by itself when the command reports socket
success or failure through a later `+QIOPEN` URC. `SEND OK` means the module
accepted the payload for sending; it is not remote application receipt.
`AT+QISEND=<connectID>,0` counters are TCP/socket evidence, not application
processing proof. End-to-end data exchange needs response evidence such as
non-empty `QIRD` output or remote endpoint evidence. The transcript should
report sent, acknowledged, and unacknowledged byte counts when counters are
returned, and report `+QIRD: 0` as no buffered response data. For fixed-length
`AT+QISEND=<connectID>,<length>` payload entry, example Sequences send the
declared payload bytes without SMS-style Ctrl-Z. Their acknowledgement query
steps declare `require_tcp_ack = true`, so `atctl` retries
`AT+QISEND=<connectID>,0` within the step timeout and fails the Sequence if the
payload remains unacknowledged instead of reporting `Result: OK`.

Quectel and SORACOM TCP examples provide editable defaults or candidate help for
routine modem values. PDP context ID uses `candidate = "pdp-context"` with
standard `AT+CGACT?` / `AT+CGDCONT?` results. Socket connect ID uses an editable
default and a hint to the Quectel socket-state command. Read length also has an
editable default.

The SORACOM example file demonstrates provider-specific network reachability
and TCP entry point checks using Quectel TCP/IP AT commands as the modem
backend. It is separate from the generic Quectel example because endpoint,
port, SORACOM service behavior, and remote evidence rules are SORACOM-specific.

Current SORACOM example targets:

- SORACOM Ping Response Service: `pong.soracom.io`, used to check SORACOM
  network reachability. This verifies ping replies only and is not TCP,
  Unified Endpoint, Beam, or destination application receipt proof.
- Unified Endpoint TCP: `unified.soracom.io:23080` with `uni.soracom.io` also
  documented by SORACOM as an alias. This verifies modem/socket evidence and
  requires SORACOM destination logs or response data for end-to-end proof.

For SORACOM TCP -> HTTP/HTTPS or Unified Endpoint forwarding, TCP stream data
may be split or combined. If message boundaries matter, use Soracom Binary
Format v1, an HTTP entry point, or application-layer framing instead of
assuming one TCP write equals one cloud message.

## References

- 3GPP TS 27.007, AT command set for User Equipment:
  https://www.3gpp.org/dynareport/27007.htm
- 3GPP TS 27.005, SMS and CBS AT command interface:
  https://www.3gpp.org/DynaReport/27005.htm
- 3GPP TS 23.038, SMS alphabets and language-specific information:
  https://www.3gpp.org/DynaReport/23038.htm
- 3GPP TS 23.040, technical realization of SMS:
  https://www.3gpp.org/DynaReport/23040.htm
- ITU-T V.250, serial asynchronous automatic dialling and control:
  https://www.itu.int/rec/T-REC-V.250/en
- Quectel EG25-G hardware design:
  https://quectel.com/content/uploads/2024/04/Quectel_EG25-G_Hardware_Design_V1.5.pdf
- Quectel EC2x/EG2x/EG9x/EM05 AT commands manual landing page:
  https://www.quectel.com/download/quectel_ec2xeg2xeg9xem05_series_at_commands_manual_v2-2/
- Quectel EC2x/EG2x/EG9x/EM05 QCFG AT commands manual:
  https://quectel.com/content/uploads/2024/02/Quectel_EC2xEG2xEG9xEM05_Series_QCFG_AT_Commands_Manual_V1.0.pdf
- Quectel EC2x/EG9x/EM05 TCP/IP AT commands manual:
  https://sixfab.com/wp-content/uploads/2018/09/Quectel_EC2xEG9xEM05_TCPIP_AT_Commands_Manual_V1.0.pdf
- SORACOM service endpoints:
  https://developers.soracom.io/en/docs/reference/endpoints/
- SORACOM modem testing and Ping Response Service:
  https://developers.soracom.io/en/docs/soracom-onyx-lte-usb-modem/testing/
- SORACOM Unified Endpoint:
  https://developers.soracom.io/en/docs/unified-endpoint/
- Soracom Binary Format v1:
  https://developers.soracom.io/en/docs/groups/binary-format-v1/
- SORACOM APN settings:
  https://users.soracom.io/ja-jp/docs/air/apn-settings/
- SORACOM CHAP authentication:
  https://users.soracom.io/ja-jp/docs/air/configure-chap/
