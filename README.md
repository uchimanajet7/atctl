# atctl

[日本語 README](README-ja.md)

`atctl` is a Rust CLI/TUI for sending and managing AT commands for USB
cellular modems from macOS on Apple Silicon.

It uses `libusb` for direct USB access, including cases where macOS does not
expose a usable `/dev/cu.*` serial device.

![atctl TUI showing device details, command categories, command list, response output, and saved logs](docs/assets/atctl-tui-overview.png)

## Validated Scope

- Mac: Apple Silicon Mac
- USB modem: SORACOM Onyx LTE USB Dongle
- Internal modem: Quectel EG25-G
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

This is the validated environment, not a claim of broad modem or operating
system coverage. See the [installation guide](docs/INSTALL.md) for the complete
validation boundary and runtime prerequisites.

## Install

Install `atctl` with Homebrew:

```sh
brew install uchimanajet7/atctl/atctl
```

The Homebrew formula installs the `libusb` runtime dependency. See the
[installation guide](docs/INSTALL.md)
for prerequisites and installation checks.

## First Five Commands

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
[troubleshooting guide](docs/TROUBLESHOOTING.md).

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

## Safety and Logs

AT commands can read sensitive identifiers and change modem state. `atctl`
masks sensitive output by default and requires confirmation for state-changing
or otherwise material actions. Review unfamiliar commands, destination values,
and externally supplied preset or Sequence definitions before execution.

Normal `send`, preset, Sequence, and TUI executions write masked command history
and session logs by default:

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/<timestamp>.session.log
```

`--no-log` prevents new normal logs only for that invocation; it does not delete
existing logs or disable a separately requested raw diagnostic export. Raw
diagnostic evidence may contain unmasked sensitive data and is created only
when the operator chooses an output path and enters the `raw-log`
acknowledgement.

Read the [safety guide](docs/SAFETY.md) before unfamiliar or state-changing
operations. See [Review and Manage Logs](docs/TROUBLESHOOTING.md#review-and-manage-logs)
for log locations, review, export, retention, and deletion, and the
[presets and Sequences reference](docs/PRESETS.md) for definition loading,
input assistance, execution review, and workflow details.

## User Documentation

- [Installation](docs/INSTALL.md) — install, update, remove, and first-run checks
- [Presets and Sequences reference](docs/PRESETS.md) — usage, inventory, TOML,
  and repository examples
- [Safety guide](docs/SAFETY.md) — operator precautions and sensitive-data care
- [Troubleshooting](docs/TROUBLESHOOTING.md) — diagnosis, recovery, logs, and
  Response export
- [Release history](CHANGELOG.md) — notable changes by released version

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
[contribution guide](CONTRIBUTING.md)
before opening a pull request.

## Maintainer Information

- [Development guide](docs/DEVELOPMENT.md)
- [Packaging and release guide](docs/PACKAGING.md)
- [Product and technical specification](docs/SPEC.md)
- [Accepted product and architecture decisions](docs/DECISIONS.md)

## License

MIT. See [LICENSE](LICENSE).
