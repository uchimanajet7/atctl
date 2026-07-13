# atctl

[日本語 README](https://github.com/uchimanajet7/atctl/blob/main/README-ja.md)

`atctl` is a Rust CLI/TUI for sending and managing AT commands for USB
cellular modems from macOS on Apple Silicon.

![atctl TUI showing device details, command categories, command list, response output, and saved logs](https://github.com/uchimanajet7/atctl/raw/main/docs/assets/atctl-tui-overview.png)

Validated environment:

- Mac: Apple Silicon Mac
- USB modem: SORACOM Onyx LTE USB Dongle (Quectel EG25-G)
- USB ID: `0x2c7c:0x0125`
- Observed firmware: `EG25GGBR07A08M2G` (`ATI`, 2026-06-17)

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
- Run repeatable checks with `atctl preset list` and
  `atctl preset run <NAME>`.
- Run multi-step SMS and data-send checks with `atctl sequence list` and
  `atctl sequence run <SEQUENCE>`.
- Use `atctl bridge --symlink <PATH>` for a terminal-style PTY bridge.
- Collect raw diagnostic evidence only with an explicit output path and
  `raw-log` acknowledgement.

## Logs

`atctl send`, `atctl preset run`, `atctl sequence run`, and TUI executions
write masked command history and session logs by default:

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/<timestamp>.session.log
```

These paths follow the
[XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/).
Set `XDG_STATE_HOME` to a non-empty absolute path to use a different state
directory for one invocation. `atctl` appends its own `atctl` directory:

```sh
env XDG_STATE_HOME="$HOME/Documents" atctl send AT
env XDG_STATE_HOME="$HOME/Documents" atctl logs list
```

These commands use `$HOME/Documents/atctl/`. To skip new masked history and
session logs for one command or TUI session, use `--no-log`:

```sh
atctl send AT --no-log
atctl preset run modem-info --no-log
atctl sequence run sms-receive-check --no-log
atctl tui --no-log
```

`--no-log` does not hide existing logs and does not disable a raw diagnostic
export explicitly requested with `--raw-log-file` or the TUI raw-export action.
`atctl` retains normal masked logs in the XDG state directory until the operator
removes them; it does not apply a retention period or automatic rotation. TUI
log actions can reveal the selected log directly. To keep a separate copy of a
current Response, choose `Export response...` in the TUI and select a destination
folder, or pass `--export-response <PATH>` to `send`, `preset run`, or
`sequence run`. Response export follows the selected foreground masking mode,
uses a new file, and never replaces normal stdout or the generated masked logs.
When the TUI Response is unmasked, copy requires `copy` acknowledgement and
export requires `export` acknowledgement after destination selection.
See
[Review and Manage Logs](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md#review-and-manage-logs)
for CLI and TUI review, Response export, Finder access, and deletion effects.

## Safety

Command and Sequence rows use one risk label: `[safe]`, `[sensitive]`,
`[write]`, `[persistent]`, `[dangerous]`, or `[unknown]`. Output masking state
and required confirmation are shown separately.

AT commands can read sensitive identifiers and change modem state. `atctl`
masks sensitive output by default, requires confirmation for state-changing
actions, and creates raw diagnostic exports only when the user chooses an
output file and acknowledges the risk. Read the
[safety guide](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
before running unfamiliar or state-changing commands.

## Presets and Sequences

A preset runs one AT command. A Sequence runs a multi-step workflow such as SMS
send, read, or reply. Product-provided presets and standard Sequences are ready
to use from the CLI and TUI.

```sh
atctl preset list
atctl preset run modem-info
atctl sequence list
atctl sequence run sms-receive-check
```

Repository-managed examples and project-local definitions are loaded by
providing their file or directory for the current command or TUI session:

```sh
atctl preset list --preset-dir examples/presets
atctl sequence list --sequence-dir examples/sequences
atctl tui --preset-dir examples/presets --sequence-dir examples/sequences
```

External definitions retain their source identity and use the same risk,
confirmation, masking, logging, and raw-export protections as product-provided
definitions. Review their source and destination values before execution. See
the [presets and Sequences reference](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
for inventories, TOML formats, loading options, Sequence parameters, and
evidence interpretation.

## User Documentation

- [Installation](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
- [Presets and Sequences reference](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
- [Safety guide](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
- [Troubleshooting](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md)

## Questions and reports

For usage questions, bug reports, and feature requests, search the existing
[GitHub Issues](https://github.com/uchimanajet7/atctl/issues) and open a new issue
if needed.

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
