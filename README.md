# atctl

[日本語 README](https://github.com/uchimanajet7/atctl/blob/main/README-ja.md)

`atctl` is a Rust CLI/TUI for sending and managing AT commands for USB
cellular modems from macOS on Apple Silicon.

Validated environments:

- Mac: Apple Silicon Mac
- USB modem: SORACOM Onyx LTE USB Dongle (Quectel EG25-G)
- USB ID: `0x2c7c:0x0125`

## Install

Install `atctl` with Homebrew:

```sh
brew install uchimanajet7/atctl/atctl
```

The Homebrew formula installs the runtime `libusb` dependency. See
[docs/INSTALL.md](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
for installation details and runtime prerequisites.

## First commands

Start by confirming that the modem is visible and responds to AT commands:

```sh
atctl devices
atctl inspect
atctl send AT
atctl send ATI
atctl tui
```

Use `atctl devices` to find the current USB target. If the expected modem is not
shown, run `atctl devices --all-usb` and see
[docs/TROUBLESHOOTING.md](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md).

## Main workflows

- Work interactively with `atctl tui`.
- Send a direct AT command with `atctl send <COMMAND>`.
- Run repeatable one-command checks with `atctl preset list` and
  `atctl preset run <NAME>`.
- Run multi-step SMS checks with `atctl sequence list` and
  `atctl sequence run <SEQUENCE>`.
- Load repository-managed data-send examples explicitly with
  `--sequence-dir examples/sequences`.
- Use `atctl bridge --symlink <PATH>` when a terminal-style PTY bridge is needed.
- Collect raw diagnostic evidence only with an explicit output path and
  `raw-log` acknowledgement.

## Safety

AT commands can read sensitive identifiers and change modem state. `atctl` masks
sensitive output by default, requires confirmation for state-changing actions,
and creates raw diagnostic exports only when the user chooses an output file and
acknowledges the raw export risk. See
[docs/SAFETY.md](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md).

## Documentation

User docs:

- [docs/INSTALL.md](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md): installation and runtime prerequisites
- [docs/TROUBLESHOOTING.md](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md): USB and modem troubleshooting
- [docs/PRESETS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md): presets, Sequences, TOML formats, and examples
- [docs/SAFETY.md](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md): AT command safety and data handling

Maintainer docs:

- [docs/DEVELOPMENT.md](https://github.com/uchimanajet7/atctl/blob/main/docs/DEVELOPMENT.md): local development setup and verification
- [docs/PACKAGING.md](https://github.com/uchimanajet7/atctl/blob/main/docs/PACKAGING.md): Homebrew and release packaging

Specification and status docs:

- [docs/SPEC.md](https://github.com/uchimanajet7/atctl/blob/main/docs/SPEC.md): implementation specification and source of truth
- [docs/OPEN-QUESTIONS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/OPEN-QUESTIONS.md): decisions that require approval
- [docs/IMPLEMENTATION-STATUS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/IMPLEMENTATION-STATUS.md): implementation progress and resume state

## Shared presets

Presets are an application-level workflow, not a TUI-only feature. The same
loaded preset set is used by:

- `atctl preset list`
- `atctl preset run <NAME>`
- `atctl tui`

This lets a user define a modem, carrier, or project-specific AT command once,
inspect it from the CLI, run it directly from scripts, and use it interactively
from the TUI with the same masking, risk classification, confirmation, and
timeout behavior. Product presets and file presets keep distinct origins, but
they use one loaded preset contract after validation. CLI listings include the
preset set label and a `source-path` column. Product presets use `-` for the
source path; file presets show the file path that supplied the row. `preset
run` also prints the source label, file path, and review notice before USB
access when a file preset is executed, including non-interactive
`--yes --risk-ack <risk>` runs. The TUI keeps the built-in-only view clean and
distinguishes file presets with non-selectable source group headers and
`Source: <title>` details only when that distinction is relevant. It uses the
file-level TOML `title` directly, without an `Add-on:` prefix. Categories
remain workflow categories; preset set title, vendor, and file-origin labels
are not mixed into the category list.

File presets are not discovered automatically from `~/.config/atctl`. Load
repository-managed examples or project-local presets explicitly for the current
invocation:

```sh
atctl preset list --preset-dir ./presets
atctl preset run my-command --preset-file ./presets/custom.toml
atctl tui --preset-dir ./presets
```

## Sequences

Sequences are the multi-step counterpart to one-shot presets. A preset stores
one AT command line. A Sequence can include prompt waits, payload writes, URC
waits, per-step timeouts, and a result transcript.

The same loaded Sequence set is used by:

- `atctl sequence list`
- `atctl sequence run <SEQUENCE>`
- `atctl tui`

Product-provided standard Sequences, repository-managed example Sequences, and
user-authored Sequences keep separate origins and review responsibility. They
share the same Sequence loader validation, duplicate-name rejection, execution
engine, masking, risk aggregation, transcript, raw diagnostic export, and
surface display contract after loading.

Sequence add-ons are not discovered automatically from `~/.config/atctl`.
Explicit per-invocation Sequence locations are available for repository
examples, review, and project-local extensions:

```sh
atctl sequence list --sequence-dir ./sequences
atctl sequence run custom-sequence --sequence-file ./sequences/custom.toml
atctl tui --sequence-dir ./sequences
```

CLI `sequence list` includes a `source-path` column. Product Sequences use `-`;
file Sequences show the file path that supplied the row. `sequence run` prints
the source label, file path, and review notice before USB access when a file
Sequence is executed, including non-interactive `--yes --risk-ack <risk>` runs.

Standard SMS send/read/reply checks are product-provided standard Sequences.
User-authored Sequence TOML is not a required first step for those ordinary
product workflows. SMS send reviews destination and message body before USB
access. SMS read reviews the SMS storage index and decodes supported bodies
while keeping normal output masked. SMS reply reviews SMS storage index and reply body,
derives the reply destination from the original message sender returned by
`AT+CMGR`, and then uses the standard `AT+CMGS` submit path. Response output,
logs, saved output, and JSON stay masked by default. Quectel ping/TCP checks
and SORACOM ping/Unified Endpoint TCP checks are vendor/provider-specific and
are provided as repository-managed example Sequence definitions under
`examples/sequences/`; load them explicitly with
`--sequence-dir examples/sequences`.
Those examples check Quectel PDP context state with `AT+QIACT?` during the
confirmed Sequence run and reuse an already active context instead of blindly
sending `AT+QIACT=<contextID>` again. TCP examples also show any failure cleanup such as
`AT+QICLOSE=<connectID>` in the Response transcript. Fixed-length TCP payload
steps send only the declared payload bytes, and the send-acknowledgement step
requires `AT+QISEND=<connectID>,0` counters to show the payload is fully
acknowledged before the Sequence can finish with `Result: OK`. External
application receipt still requires non-empty response data or destination-side
logs. The repository-managed ping examples use `AT+QPING`; received replies
are IP or SORACOM network reachability evidence, not TCP payload delivery or
destination application proof. Those ping steps wait for `+QPING:` result lines;
the command-accepted `OK` alone is not treated as reachability success.

See [docs/PRESETS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
for the product preset reference, file preset TOML format, loading rules, and
repository-managed example preset files.

## License

MIT. See [LICENSE](LICENSE).
