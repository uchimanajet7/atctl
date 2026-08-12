# Troubleshooting

Use this guide to diagnose an `atctl` or modem symptom, check the result of each
step, recover where possible, and review or dispose of diagnostic artifacts.
Start with the section that matches the observed symptom.

For preset and Sequence inventories, definition formats, and loading, see
[Presets and Sequences Reference](PRESETS.md). For operator precautions around
review, confirmation, masking, logging, and raw diagnostics, see the
[Safety Guide](SAFETY.md).

## USB Modem Is Not Visible in the USB Tree

Run:

```sh
system_profiler SPUSBHostDataType | grep -Ei -A 12 'EG25|Quectel|2c7c|0125'
```

Expected identifiers for the currently documented SORACOM Onyx environment:

```text
Vendor: Quectel
Product: EG25-G
VID: 0x2c7c
PID: 0x0125
```

If the modem is not visible:

- Reconnect the modem.
- Try another USB port.
- Avoid unpowered USB hubs.
- Check whether the modem needs more power under radio load.

## `/dev/cu.*` Does Not Appear

This is the reason `atctl` exists. macOS may show the modem as a USB device
without creating a usable `/dev/cu.*` serial device.

Run:

```sh
ls /dev/cu.*
```

If no Onyx or EG25-G callout device appears, try `atctl devices`. `atctl` can
use libusb when the USB interfaces and endpoints are available to claim.

## List Devices Through libusb

```sh
atctl devices
```

By default, the output shows only plausible `atctl` operation targets based on
current USB descriptors. It does not depend on a built-in known-device list.
The current filter is descriptor-based: it keeps devices whose USB class is a
communication, miscellaneous, or vendor-specific candidate and whose descriptors
include at least one bulk IN / bulk OUT endpoint pair. This reduces unrelated
USB noise such as hubs, LAN adapters, webcams, microphones, and billboard
devices. It is not an AT probe and does not guarantee that a device supports AT
commands.

To inspect every USB device visible through `libusb`, including devices that
are not useful AT targets, run:

```sh
atctl devices --all-usb
```

Use `--all-usb` for troubleshooting physical USB visibility and descriptor
noise, not as the normal first-time target selection workflow.

If an expected modem does not appear in the default output:

1. Run `atctl devices --all-usb` and find the candidate by current USB
   descriptor values such as manufacturer, product, VID, PID, bus, and address.
2. Run `atctl inspect --bus <BUS> --address <ADDRESS>` for that candidate.
3. Check the device-level class shown by `devices --all-usb` or `inspect`. A
   class triple of `00/00/00` is a known limit of the default operation-target
   filter: USB-IF defines device class `00h` to use class information from
   Interface Descriptors. It does not by itself identify a modem or an AT
   interface.
4. Treat interface classes and bulk IN / bulk OUT pairs as descriptor evidence,
   not as proof that sending AT commands is safe. Different interfaces on one
   composite USB device can serve unrelated functions.
5. Review the descriptor output before sharing it. Remove or mask serial
   numbers and any other device-specific or subscriber-identifying values.
6. Search the existing [GitHub Issues](https://github.com/uchimanajet7/atctl/issues)
   and open a new issue if needed. Include the `atctl` version, operating system,
   the reviewed relevant output from `devices --all-usb` and `inspect`, and a
   link to vendor interface documentation when available. State whether the AT
   interface and endpoints have been independently identified.

Do not use `--bus <BUS> --address <ADDRESS>` alone as the ordinary recovery
path for a diagnostic-only item. The USB transport may otherwise try candidate
bulk endpoint pairs with `AT`. For `send`, `preset run`, `sequence run`, or PTY
bridge execution, proceed only when vendor documentation or an independently
validated device configuration identifies the AT interface and both bulk
endpoints. Then provide all of those values explicitly, for example:

```sh
atctl send AT \
  --bus <BUS> \
  --address <ADDRESS> \
  --interface <N> \
  --bulk-in <ENDPOINT> \
  --bulk-out <ENDPOINT>
```

This condition applies equally to every AT execution surface. The TUI keeps a
diagnostic-only item unavailable for AT sending.

The TUI Devices pane initially shows the same operation targets as
`atctl devices`. Select `Show all USB devices` and press `Enter` to inspect the
full USB view. Select `Show operation targets` to return. Devices that are not
operation targets remain unavailable for AT sending.

Reference basis:

- USB-IF Defined Class Codes:
  https://www.usb.org/defined-class-codes
- Microsoft USB standard descriptors:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/standard-usb-descriptors
- libusb USB descriptors:
  https://libusb.sourceforge.io/api-1.0/group__libusb__desc.html
- Zephyr USB CDC ACM:
  https://docs.zephyrproject.org/latest/services/connectivity/usb/device_next/cdc_acm.html

## Inspect USB Descriptors

```sh
atctl inspect
```

The output includes configurations, interfaces, alternate settings, and bulk
endpoints. Endpoint values observed in one environment are diagnostic evidence,
not universal defaults. For a device already shown as an operation target, if
endpoint auto-detection fails, inspect the current descriptors and retry with
explicit values:

```sh
atctl send AT --interface <N> --bulk-in <ENDPOINT> --bulk-out <ENDPOINT>
```

For a diagnostic-only device, descriptor shape alone is not enough. Use
explicit interface and endpoint values only after vendor documentation or an
independently validated device configuration identifies them as the AT path.

## Interface Claim Failure

Possible causes:

- Another `atctl` process is using the interface.
- `screen`, `minicom`, `cu`, or another tool is using a related interface.
- macOS or a kernel driver owns the interface.
- The device was unplugged.

Diagnostic command:

```sh
ps aux | grep -Ei 'atctl|screen|minicom|cu' | grep -v grep
```

Then stop the conflicting process, unplug and reconnect the modem, and retry.

## PTY Bridge

The bridge exposes an `atctl`-managed PTY through a symlink for terminal tools:

```sh
atctl devices
atctl bridge --symlink /tmp/atctl --bus <BUS> --address <ADDRESS>
screen /tmp/atctl 115200
```

Choose `BUS` and `ADDRESS` from the current `atctl devices` output. If the
expected target is not shown there, inspect `atctl devices --all-usb` to confirm
USB visibility. VID/PID values are runtime selectors, not required prior
knowledge. Use `--vid` and `--pid` only when that pair is unique in the current
target output.

For example, if `atctl devices` prints:

```text
EG25-G 0x2c7c:0x0125 bus=1 address=4
```

then run:

```sh
BUS=1
ADDRESS=4

atctl inspect --bus $BUS --address $ADDRESS
atctl bridge --symlink /tmp/atctl --bus $BUS --address $ADDRESS
screen /tmp/atctl 115200
```

`115200` is required by serial terminal tools as a compatibility value. It is
not the physical speed of the USB modem path. To quit `screen`, press
`Ctrl-A`, then `K`, then `y`.

The bridge is a continuous terminal session, so it does not use the bounded
`--export-response` option. To record the normal masked bridge transcript with
GNU Screen, choose the transcript file when starting the client:

```sh
screen -L -Logfile "$HOME/Documents/atctl-bridge-session.log" \
  /tmp/atctl 115200
```

Inside an existing Screen session, `Ctrl-A`, then `H` starts or stops session
logging. Screen appends when the selected log already exists; choose a new path
when a separate transcript is required. This terminal transcript is distinct
from atctl raw diagnostic export. Use `--raw-log-file <PATH>` with
`--raw-log-ack raw-log` only when underlying unmasked diagnostic exchanges are
required and the sensitive-output warning is accepted.

The bridge resolves the USB target before creating the symlink. If no matching
device is visible, or if multiple devices match the provided filters, bridge
startup fails before creating `/tmp/atctl`.

If the symlink path already exists:

- Existing regular files and directories are never overwritten.
- Existing symlinks are rejected by default.
- Use `--replace-symlink` only when the existing path is a stale symlink you
  want `atctl` to replace.

Examples:

```sh
BUS=1
ADDRESS=4

atctl bridge --symlink /tmp/atctl --bus $BUS --address $ADDRESS
atctl bridge --symlink /tmp/atctl --replace-symlink --bus $BUS --address $ADDRESS
atctl bridge --symlink /tmp/atctl --vid 0x2c7c --pid 0x0125
```

The VID/PID example is valid only when the current `atctl devices` output shows
that `0x2c7c:0x0125` identifies exactly one visible operation target.

Only one external PTY client is supported. Opening the same symlink from
multiple clients can interleave input and is not guaranteed.

If a bridge command times out or the USB transport fails, restart the bridge
before sending more commands. Running-command interruption, host-side read abort,
USB reconnect, and AT resync are not normal bridge features.

See [PTY Bridge Policy](SAFETY.md#pty-bridge-policy) for the confirmation,
masking, raw-export, and multi-client safety boundaries.

## Timeout

Timeout may mean:

- Wrong interface or endpoints were selected.
- The modem did not respond to the command.
- The command needs a longer timeout.
- The device disconnected.

Try:

```sh
atctl inspect
atctl send AT --timeout 30
```

User AT command execution defaults to a 30-second timeout. Long-running
presets and Sequences can use an item-specific timeout hint, and an explicit
CLI timeout takes precedence when supplied.

The available-operator scan can take longer than ordinary reads:

```sh
atctl preset run available-operators
```

This preset sends `AT+COPS=?`, which is useful when checking which carrier /
RAT options the modem can see. SORACOM documents this scan as typically taking
2 to 3 minutes, so the product preset uses a 240-second timeout hint. If the
local scan still times out, run it with a longer explicit timeout:

```sh
atctl preset run available-operators --timeout 600
```

In the TUI, select the Controls pane `Timeout` row and press `Enter` before
running the command to set one timeout for Commands and Sequences until the TUI
closes. Leave `Seconds` blank and press `Enter` to return to each Command or
Sequence timeout.

A timeout is a host-side wait limit, not modem cancellation. The modem may
still be processing the operation after atctl reports a timeout. Do not
automatically retry a material operation; wait for the modem to settle or
re-establish a synchronized session first. A modem-reported AT `ERROR` is a
different result and should be investigated using the returned error text.

## Cellular Connectivity Checkpoints

For SORACOM Air cellular data-send/receive troubleshooting, the standard core
presets cover the AT command checkpoints listed in SORACOM's advanced
diagnostic reference:

```sh
atctl preset run modem-info
atctl preset run imsi
atctl preset run radio-stack
atctl preset run radio-stack-capabilities
atctl preset run current-operator
atctl preset run available-operators
atctl preset run operator-format-numeric
atctl preset run signal-quality
atctl preset run extended-signal-quality
atctl preset run circuit-registration
atctl preset run gprs-registration
atctl preset run eps-registration
atctl preset run enable-circuit-registration-detail
atctl preset run enable-gprs-registration-detail
atctl preset run enable-eps-registration-detail
atctl preset run enable-eps-registration-cause
atctl preset run enable-eps-registration-extended
atctl preset run pdp-contexts
atctl preset run pdp-auth-settings
atctl preset run pdp-auth-capabilities
atctl preset run packet-attach
atctl preset run active-pdp-contexts
atctl preset run pdp-addresses
atctl preset run pdp-connection-details
atctl preset run extended-error-report
atctl preset run error-reporting-status
atctl preset run modem-activity-status
```

Source: https://users.soracom.io/ja-jp/guides/diagnostic/advanced/

The commands that set operator format, operator auto-selection, or detailed
registration reporting have runtime-state effects and require confirmation.
Run them only when you deliberately want to change the modem's runtime
reporting or selection mode for troubleshooting. To return operator selection
to automatic mode, use:

```sh
atctl preset run operator-auto-selection
```

`pdp-auth-settings` reads standard PDP authentication settings and can expose APN
credentials. Output and saved logs are masked by default.

For the complete product preset reference, file preset TOML format, loading
rules, and repository-managed example preset files, see
[docs/PRESETS.md](PRESETS.md). To use external preset or Sequence files, pass
their paths in the current command or TUI session.

The standard PDP presets can show attach state, active PDP contexts, IP address
assignment, and connection details. They do not prove that an application
payload reached an external server. Use a vendor-specific data-send Sequence or
another endpoint-aware tool for that check.

## SMS and Data-Send Sequence Checks

Use Sequences for multi-step checks that cannot be represented as one command
and one final response.

This section provides the operator procedure and the result checks used during
troubleshooting. See [Standard Sequences](PRESETS.md#standard-sequences) and
[Repository Example Sequences](PRESETS.md#repository-example-sequences) for the
complete definition and evidence reference, and
[SMS Safety](SAFETY.md#sms-safety) and
[Data-Send Sequences](SAFETY.md#data-send-sequences) for the required safety
boundaries.

Standard SMS checks:

```sh
atctl sequence list
atctl sequence run sms-send-check --param recipient=<PHONE_NUMBER> --param message=<MESSAGE>
atctl sequence run sms-receive-check
atctl sequence run sms-read-message --param index=<INDEX>
atctl sequence run sms-reply-check --param index=<INDEX> --param message=<MESSAGE>
```

In the TUI, select the `sms` category and choose a Sequence from
`Commands / Sequences`. Press `d` to inspect the complete definition without
accessing the modem, then press `Enter` to resolve values and review the run.

For SMS read or reply, use the displayed candidate action for
`sms-receive-check` when a fresh list is needed, then select the required
`storage=<index>` value. Opening `Run Sequence` does not read messages. A
candidate provider uses its own normal review and confirmation, and accepting a
candidate still does not run the selected read or reply Sequence. See
[Selecting Candidate Values](PRESETS.md#selecting-candidate-values) for the
complete input procedure.
Interactive `atctl sequence run sms-read-message` and
`atctl sequence run sms-reply-check` offer the same provider and manual-entry
paths when `index` is omitted. In a non-interactive or JSON invocation, run
`sms-receive-check` explicitly, obtain the storage index, and rerun with
`--param index=<INDEX>`; atctl does not execute the provider silently.

Review the destination, message body, or storage index shown before USB access,
then enter the requested `run` or `unknown` confirmation. During execution, Status shows the
current step and Response shows command material, modem response, decoded SMS,
analysis, notes, and the final result in separate sections. If a candidate
action fails, read the failure detail in Response and retry the candidate action
before running the selected Sequence.

For SMS send, `+CMGS` followed by `OK` is submit evidence from the modem/network
path. It is not proof that the destination handset displayed the message. For
SMS receive, sender numbers and message bodies are sensitive. Receive/list/read
operations that change message status or unread/read flags require confirmation
and do not silently delete messages. `REC READ` and `REC UNREAD` in modem
output are message status values. Reading a specific SMS with `AT+CMGR` can
change unread state to read state, so `sms-read-message` and reply-by-index
require confirmation. Listing with `AT+CMGL` can also change received-unread
messages to received-read. Supported SMS bodies such as UCS2 hex are decoded
before masking.
Normal Response shows decoded-body lines under `Decoded SMS:` with the body
hidden and atctl-derived interpretation under `Analysis:`. `--no-mask` or TUI
output masking off shows supported bodies as decoded text in the foreground
display. Raw diagnostic export is a separate acknowledged raw modem exchange
and may contain modem-returned encoded body bytes.

For SMS reply, `sms-reply-check` is not the same as a fresh
recipient-supplied send. It reads the original SMS storage index, extracts the
sender returned by `AT+CMGR`, and submits the reply body to that sender. The
submit result is still `+CMGS` evidence, not destination handset receipt proof.
Use `sms-receive-check` or another SMS list command first when you need to
confirm the SMS storage index; the value is the storage index shown by `+CMGL`.

Repository-managed Quectel and SORACOM data-send checks:

```sh
atctl sequence list --sequence-dir examples/sequences
atctl sequence run quectel-ping-check --sequence-dir examples/sequences --param host=<HOST>
atctl sequence run quectel-tcp-send-check --sequence-dir examples/sequences --param host=<HOST> --param port=<PORT> --param payload=<PAYLOAD>
atctl sequence run soracom-ping-check --sequence-dir examples/sequences
atctl sequence run soracom-unified-endpoint-tcp-send-check --sequence-dir examples/sequences --param payload=<PAYLOAD>
atctl tui --sequence-dir examples/sequences
```

The Quectel data-send and ping examples are vendor-specific. The SORACOM
examples are provider-specific reachability and endpoint checks implemented
with Quectel TCP/IP AT commands as the modem backend. They are loaded
explicitly and remain repository-managed examples rather than product-provided
standard Sequences.

The TCP examples prefill routine modem plumbing values where an editable
default is useful: PDP context ID defaults to `1`, socket connect ID defaults to
`0`, and read length defaults to `1500`. These repository-managed add-on
Sequences use product-known candidate assistance for standard PDP context
checks from `AT+CGACT?` and `AT+CGDCONT?`. Socket connect ID is vendor-specific
Quectel TCP/IP state, so it stays an editable value with a hint to run the
explicit Quectel socket-state add-on command when current socket state must be
checked. Selecting a PDP candidate action runs the referenced existing
`active-pdp-contexts` or `pdp-contexts` Preset through its normal execution path
and then presents parsed candidates in the same modal. Candidate actions use
the same exception-only `[confirm]` tag and conditional trailing-lane alignment
as TUI executable-item rows when their own review requires confirmation.
Opening the modal itself does not perform a hidden PDP, socket, TCP, or network
check.

The ping examples use Quectel `AT+QPING` through the selected PDP context.
Received replies show IP reachability to the requested host or to SORACOM
`pong.soracom.io`. Ping success does not prove TCP socket opening, Unified
Endpoint forwarding, Beam configuration, payload delivery, or destination
application processing. For the repository-managed ping examples, terminal
`OK` is only command-accepted evidence. The Sequence waits for `+QPING:` result
lines, and terminal `OK` without a parsed successful `+QPING:` reply or summary
is not enough for `Result: OK`.

During TCP Sequence execution, `atctl` checks Quectel TCP/IP PDP context state
with `AT+QIACT?`. If the selected context is already active, it reuses that
context and does not send `AT+QIACT=<contextID>` again. If the context is not
active, it sends `AT+QIACT=<contextID>` during the confirmed Sequence run. This
keeps repeated runs from failing simply because the modem kept the PDP context
active after the previous successful run.

If a TCP socket has been opened and a later Sequence step fails, the example
Sequences run the configured `AT+QICLOSE=<connectID>` cleanup command as
best-effort failure recovery. The cleanup command and modem response are shown
in Response; the original failed step remains the failure reason.

For Quectel socket checks, `+QIOPEN: <id>,0` indicates socket open success,
`SEND OK` indicates the module accepted the payload for sending,
an `AT+QISEND=<id>,0` step with `require_tcp_ack = true` reports TCP/socket
acknowledgement state, and a non-empty QIRD response or remote endpoint evidence
is needed for end-to-end application proof. `OK` alone after `QIOPEN` is not
full network success. The same three-field `+QISEND:` response has UDP
sent/unsent semantics, so output without the explicit TCP step contract is not
automatically labelled as TCP analysis. For the repository-managed TCP examples,
`+QISEND:` counters with remaining unacknowledged payload bytes are not a
successful send condition; the Sequence retries the acknowledgement query
within the step timeout and fails with the last counters visible if the payload
is still not acknowledged. For a normal QIRD read, `+QIRD: 0` means no buffered
receive data. `AT+QIRD=<id>,0` instead returns total received, already read, and
unread counters without consuming payload; a UDP SERVICE read may return
length, remote IP, and remote port. atctl uses the originating command to avoid
confusing these forms.

For the SORACOM reachability endpoints, Beam boundary, and TCP message-framing
reference, see
[Repository Example Sequences](PRESETS.md#repository-example-sequences).

## Failure Reports and Verbose Errors

When attach, registration, operator selection, or PDP activation fails with only
`ERROR`, first inspect the modem's failure context:

```sh
atctl preset run extended-error-report
atctl preset run error-reporting-status
atctl preset run modem-activity-status
```

`extended-error-report` sends `AT+CEER` and reads the latest extended failure
report when the modem implements it. `error-reporting-status` sends `AT+CMEE?`
and shows whether extended mobile termination error reporting is enabled.
`modem-activity-status` sends `AT+CPAS` and can show whether the modem is ready,
unavailable, unknown, or asleep.

To enable verbose `+CME ERROR` reporting for subsequent commands, use the
confirmation-required preset explicitly:

```sh
atctl preset run enable-verbose-errors
```

This sends `AT+CMEE=2`. It is useful for troubleshooting, but it changes error
reporting behavior and requires confirmation before USB access.

## Modem Functionality Changes

`atctl` includes built-in modem functionality presets for standard `AT+CFUN`
commands:

```sh
atctl preset run modem-functionality
atctl preset run set-modem-minimum-functionality
atctl preset run set-modem-full-functionality
atctl preset run restart-modem
```

`modem-functionality` sends `AT+CFUN?` and reads the current functionality
level. The other three commands require `run` confirmation because their
reviews show runtime-state and service/connectivity effects. `restart-modem`
also shows reset/recovery and restart-required reversibility. They may detach
from the network, restart registration, disrupt PDP state, drop the current
USB/AT session, or require reconnecting and reselecting the modem. They are not
ordinary diagnostic checks.

Quectel-specific power-down is provided only by the Quectel example file preset:

```sh
atctl preset run power-down-quectel --preset-dir examples/presets
```

`power-down-quectel` sends `AT+QPOWD`. Its review shows runtime-state change,
service interruption, restart-required reversibility, and required
confirmation because it may power down the module. It is not a product preset
because it is Quectel-specific.

For Quectel-specific SIM, network, time, and MBN diagnostics, load the Quectel
example preset set explicitly:

```sh
atctl preset list --preset-dir examples/presets
atctl preset run sim-init-status-quectel --preset-dir examples/presets
atctl preset run pin-retries-quectel --preset-dir examples/presets
atctl preset run network-name-quectel --preset-dir examples/presets
atctl preset run network-time-quectel --preset-dir examples/presets
atctl preset run mbn-list-quectel --preset-dir examples/presets
```

`mbn-list-quectel` sends only `AT+QMBNCFG="List"`. Other Quectel MBN operations
that select, deactivate, add, delete, or auto-select MBN files are not ordinary
known no-confirmation observations.

## Review and Manage Logs

Normal AT execution writes masked command history and session logs by default:

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/<timestamp>.session.log
```

`history.jsonl` is one append-only aggregate history file. It records timestamp,
the `cli` or `tui` surface, structured command/preset/Sequence identity, the
masked execution review, status, duration, and device selection without the
response body. Each `.session.log` is a separate masked record for one execution
and includes its masked response. Definition origin, title, and path remain
separate fields instead of being combined into an internal source string.

List the paths of both artifact types:

```sh
atctl logs list
```

If `XDG_STATE_HOME` was set for the original execution, use the same value when
listing its logs. `atctl` appends its own `atctl` directory to the XDG base:

```sh
env XDG_STATE_HOME="$HOME/Documents" atctl send AT
env XDG_STATE_HOME="$HOME/Documents" atctl logs list
```

This example stores and lists logs under `$HOME/Documents/atctl/`.

To review or locate one log from the TUI:

1. Focus `Logs`, select `history: history.jsonl` or a `session:` row, and press
   `Enter`.
2. Choose `Open log in Response` to review the masked contents, or choose
   `Reveal in Finder` to locate the selected file directly.
3. After opening a log in Response, press `Enter` there to copy the displayed
   masked log, reveal the same exact file, or close log view.
4. Finder opens the containing folder and selects the exact selected or opened
   file. It does not open the file contents, move the file, or delete it.

Complete the current `atctl` execution before moving log files to the Trash.
Deleting `history.jsonl` removes the complete aggregate command history; the
next logged execution creates a new history file. Deleting one `.session.log`
removes only that detailed execution record and does not remove its existing
entry from `history.jsonl`. Return to Logs and press `Enter` to refresh the list
after external file changes.

`atctl` does not apply an automatic retention period, rotation policy, pruning,
or deletion. Retain logs according to the troubleshooting case and applicable
organizational policy, then remove files and copies that are no longer needed.
A command or TUI session started with `--no-log` does not create new history or
session-log entries; existing logs remain listable.

### Export a Response

Response export creates an operator-selected copy of the current normal
Response. It is separate from generated history, session logs, and raw
diagnostic export.

In the TUI, focus Response, press `Enter`, and choose `Export response...`.
Before the folder chooser opens, the action menu identifies the Response,
generated UTF-8 text file name, and the applicable masking state. Select a
destination folder. The chooser is shown for every export. A masked export
creates the file immediately after folder selection.

If Response is displaying values that differ from its masked form, the actions
are `Copy unmasked response` and `Export unmasked response...`. Copy requires
typing `copy`. Export returns from the folder chooser to a confirmation showing
the exact final path and requires typing `export` before creating the file.
`Esc` cancels without copying or creating a file. Because these are typed
acknowledgement fields, `q` is entered as ordinary text.

After a successful export, open Response actions again and choose `Reveal in
Finder` to select the exact exported file. Running another command, opening
another Response, or clearing Response removes that association without
deleting the exported file.

For CLI execution, provide the complete output file path:

```sh
atctl send ATI --export-response "$HOME/Documents/ati-response.txt"
atctl preset run modem-info \
  --export-response "$HOME/Documents/modem-info-response.txt"
atctl sequence run sms-receive-check \
  --export-response "$HOME/Documents/sms-receive-check-response.txt"
```

The CLI validates the destination before USB access, refuses an existing file,
and continues to print the normal Response to stdout. With `--json`, the export
file contains JSON; otherwise it contains UTF-8 text. Export is masked by
default. `--no-mask` makes both foreground output and the explicitly requested
Response export unmasked; generated history and session logs remain masked.

Each export creates a new file. `atctl` does not rotate or delete exported
files. Previously created files under `$XDG_STATE_HOME/atctl/responses/` remain
where they are and are not listed by `atctl logs list` or the TUI Logs pane.

Raw diagnostic exports are separate files at the path explicitly selected by
the operator. They are not returned by `atctl logs list` or shown in the TUI
Logs pane and must be retained and removed separately.

## Sensitive Output Masking

By default, `atctl` masks IMSI, ICCID, IMEI, MSISDN, APN credentials, and similar
identifiers.

See [Default Masking](SAFETY.md#default-masking) for operator guidance about
classification, short values, saved output, and unknown vendor output. The
commands below deliberately request unmasked display or raw diagnostic evidence.

Unmasked IMSI display requires explicit action and should be used only when the
diagnostic case specifically requires confirming the IMSI:

```sh
atctl send AT+CIMI --no-mask
```

The TUI keeps sensitive output masked by default. `atctl tui --no-mask` starts
the foreground TUI session with output masking off, and the TUI Controls pane
can disable output masking after exact typed `unmask` acknowledgement. Response
copy and export follow the visible Response display; unmasked copy requires
`copy`, and unmasked export requires `export` after destination selection.
Saved logs remain masked.

Raw diagnostic export is for final evidence collection when masked output is
not enough. It requires an explicit destination and acknowledgement, and it
does not change normal terminal masking. Choose the USB target from current
`atctl devices` output first, then include that runtime selection in the
command:

```sh
atctl devices
atctl send AT+CSQ --bus <BUS> --address <ADDRESS> --raw-log-file ./case-signal.rawlog --raw-log-ack raw-log
```

The same warning is shown before raw export on CLI and PTY, in the TUI
acknowledgement modal, and in the raw JSONL header:

```text
This raw diagnostic export records transmitted and received modem bytes without masking. It may contain sensitive modem and subscriber identifiers; network data; APN or PDP authentication data; phone numbers and SMS messages; arbitrary transmitted or received payloads; credentials and tokens; and vendor-specific identifiers.
```

Use a case-specific path, keep the file protected, and share it only with the
party that needs the raw diagnostic evidence. Raw JSONL exchange events identify
the `cli`, `tui`, or `pty` surface separately from the structured command,
preset, or Sequence operation.
