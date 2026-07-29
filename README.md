# atctl

[日本語 README](https://github.com/uchimanajet7/atctl/blob/main/README-ja.md)

`atctl` is a Rust CLI/TUI for sending and managing AT commands for USB
cellular modems from macOS on Apple Silicon.

![atctl TUI showing device details, command categories, command list, response output, and saved logs](https://github.com/uchimanajet7/atctl/raw/main/docs/assets/atctl-tui-overview.png)

The validated environment is an Apple Silicon Mac with a SORACOM Onyx LTE USB
Dongle (Quectel EG25-G, USB ID `0x2c7c:0x0125`). See the
[installation guide](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
for the complete validation boundary and runtime prerequisites.

Real-hardware workflows confirmed in this environment:

- USB device discovery and descriptor inspection with `devices` and `inspect`,
  plus direct `AT`/`ATI` execution
- Preset execution with masked history and session logging
- TUI candidate refresh and related Sequence input behavior
- PTY bridge operation through `screen`
- SMS send, receive/list, read, and reply, plus Quectel/SORACOM ping and TCP
  Sequence workflows

## Install

Install `atctl` with Homebrew:

```sh
brew install uchimanajet7/atctl/atctl
```

The Homebrew formula installs the `libusb` runtime dependency. See the
[installation guide](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
for prerequisites and installation checks.

## First Commands

Confirm that the modem is visible and responds to AT commands:

```sh
atctl devices
atctl inspect
atctl send AT
atctl send ATI
atctl tui
```

Use `atctl devices` to find the current USB target. If the expected modem is not
shown, run `atctl devices --all-usb` and follow the
[troubleshooting guide](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md).

## Main Workflows

- Work interactively with `atctl tui`.
- Send one AT command with `atctl send <COMMAND>`.
- Find repeatable checks with `atctl preset list`, inspect one completely with
  `atctl preset show <NAME>`, and run it with `atctl preset run <NAME>`.
- Find multi-step SMS and data-send checks with `atctl sequence list`, inspect
  one completely with `atctl sequence show <SEQUENCE>`, and run it with
  `atctl sequence run <SEQUENCE>`.
- Use `atctl bridge --symlink <PATH>` for a terminal-style PTY bridge.
- Collect raw diagnostic evidence only with an explicit output path and
  `raw-log` acknowledgement.

## Logs

Normal `send`, preset, Sequence, and TUI executions write masked command history
and session logs by default under the XDG state directory:

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/<timestamp>.session.log
```

See
[Review and Manage Logs](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md#review-and-manage-logs)
for XDG path overrides, `--no-log`, CLI and TUI review, Response export, Finder
access, retention, and deletion. See the
[safety guide](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md#logging)
for the masking, sensitive-metadata, and raw-diagnostic boundaries.

## Safety

Human-readable CLI lists and TUI rows show `name` and the AT command or purpose
preview. Only items that require an explicit confirmation before execution
append `[confirm]`; the tag describes the next interaction, not risk severity.
In mixed results, human CLI lists, TUI rows, and TUI candidate-loading actions
align the tag in a fixed trailing lane so it can be scanned independently of
variable command or purpose text. TUI adds a dedicated confirmation-flow color
while retaining the complete text and bold emphasis in no-color mode. TUI
selection never changes row height. Use CLI `show` or press TUI `d` for the
complete read-only execution review; no modem access occurs when these details
open. TUI Details and human CLI `show` group complete information as Overview,
Definition, Before running, Inputs, and Execution, omitting empty sections.
Sequence value input identifies the Sequence and keeps only the active value,
relevant assistance, candidates, errors, and exact next action visible. Final
review shows resolved values, applicable before-running notes, and execution
consequences. If that review is long, `PageUp`/`PageDown` scrolls its Review
region while the run instruction and current input remain visible. A
multi-value Sequence keeps `Value <current> of <total>` at the modal's top
right while values are being resolved; single-value and final-review phases
omit it.
For candidate-backed Sequence values, TUI and interactive `sequence run` offer
manual entry plus existing executable providers: `sms-receive-check` for SMS
storage indexes and `active-pdp-contexts` / `pdp-contexts` for PDP context IDs.
They never substitute copied one-command actions or run a provider merely
because input is missing. In TUI, pressing `Enter` on a loaded candidate accepts
it and immediately continues to the next value or Sequence review; it does not
run the selected Sequence. Candidate-provider completion returns to value
selection without choosing a value automatically. Non-interactive and JSON runs
report the provider and the required `--param` retry instead.
When confirmation is required, the review explains what will happen and shows
sensitive-data or recovery information only when relevant. Unknown operations
separate what is known, possible consequences, and what atctl could not
establish. Known material operations use the confirmation word `run`; unknown
operations use `unknown` so uncertainty cannot be mistaken for a known effect.

AT commands can read sensitive identifiers and change modem state. `atctl`
masks sensitive output by default, requires confirmation for state-changing or
otherwise material actions, and creates raw diagnostic exports only when the
operator chooses an output file and acknowledges the warning. Read the
[safety guide](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
before running unfamiliar or state-changing commands.

## Presets and Sequences

A preset runs one AT command. A Sequence runs a multi-step workflow such as SMS
send, read, or reply. Product-provided presets and standard Sequences are ready
to use from the CLI and TUI.

```sh
atctl preset list
atctl preset show modem-info
atctl preset run modem-info
atctl sequence list
atctl sequence show sms-receive-check
atctl sequence run sms-receive-check
```

Human-readable lists keep only the name, AT command or purpose preview, and
`[confirm]` when confirmation is required. Mixed lists align the tag in a
fixed trailing lane; lists with no confirmation-required item reclaim that
space. `show` provides complete
read-only details without USB access. Add `--json` to `list` or `show` for
complete structured data.

Repository-managed examples and project-local definitions are loaded by
providing their file or directory for the current command or TUI session:

```sh
atctl preset list --preset-dir examples/presets
atctl sequence list --sequence-dir examples/sequences
atctl tui --preset-dir examples/presets --sequence-dir examples/sequences
```

Review external definition sources and destination values before execution.
See the
[presets and Sequences reference](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
for inventories, TOML formats, loading options, source identity, Sequence
parameters, execution behavior, and evidence interpretation. The
[safety guide](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
defines the common review, confirmation, masking, logging, and raw-export
protections.

## User Documentation

- [Installation](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
- [Presets and Sequences reference](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
- [Safety guide](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
- [Troubleshooting](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md)

## Questions and reports

For usage questions, bug reports, and feature requests, search the existing
[GitHub Issues](https://github.com/uchimanajet7/atctl/issues) and open a new issue
if needed.

For suspected security vulnerabilities, use
[GitHub private vulnerability reporting](https://github.com/uchimanajet7/atctl/security/advisories/new)
instead of opening a public issue.

For a bug report, include the `atctl` version when available, the affected
command or TUI action, reproduction steps, expected result, actual result, and
only reviewed masked output when needed. Do not include raw diagnostic exports,
unmasked output, credentials, subscriber identifiers, phone numbers, or message
contents in a public issue.

## Contributing

Code and documentation contributions are welcome. Read the
[contribution guide](https://github.com/uchimanajet7/atctl/blob/main/CONTRIBUTING.md)
before opening a pull request.

## Maintainer Information

- [Development guide](https://github.com/uchimanajet7/atctl/blob/main/docs/DEVELOPMENT.md)
- [Packaging and release guide](https://github.com/uchimanajet7/atctl/blob/main/docs/PACKAGING.md)
- [Product and technical specification](https://github.com/uchimanajet7/atctl/blob/main/docs/SPEC.md)
- [Accepted product and architecture decisions](https://github.com/uchimanajet7/atctl/blob/main/docs/DECISIONS.md)

## License

MIT. See [LICENSE](LICENSE).
