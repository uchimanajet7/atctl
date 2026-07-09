# atctl

[English README](https://github.com/uchimanajet7/atctl/blob/main/README.md)

`atctl` は、macOS Apple Silicon から USB セルラーモデムへ AT コマンドを
送信し、管理するための Rust 製 CLI/TUI です。

![atctl TUI の device details、command categories、command list、response output、saved logs の画面](https://github.com/uchimanajet7/atctl/raw/main/docs/assets/atctl-tui-overview.png)

検証済み環境:

- Mac: Apple Silicon Mac
- USB モデム: SORACOM Onyx LTE USB Dongle（Quectel EG25-G）
- USB ID: `0x2c7c:0x0125`

## インストール

Homebrew で `atctl` をインストールします。

```sh
brew install uchimanajet7/atctl/atctl
```

Homebrew formula は runtime dependency の `libusb` もインストールします。
インストール手順と runtime prerequisites の詳細は
[docs/INSTALL.md](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
を参照してください。

## 最初の操作

まず、モデムが表示され、AT コマンドに応答することを確認します。

```sh
atctl devices
atctl inspect
atctl send AT
atctl send ATI
atctl tui
```

現在の USB target は `atctl devices` で確認します。想定したモデムが表示
されない場合は `atctl devices --all-usb` を実行し、
[docs/TROUBLESHOOTING.md](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md)
を参照してください。

## 主なワークフロー

- `atctl tui` で interactive に作業する。
- `atctl send <COMMAND>` で AT コマンドを直接送信する。
- `atctl preset list` と `atctl preset run <NAME>` で repeatable な
  one-command check を実行する。
- `atctl sequence list` と `atctl sequence run <SEQUENCE>` で multi-step
  SMS check を実行する。
- repository-managed data-send example は `--sequence-dir examples/sequences`
  で明示的に読み込む。
- terminal-style PTY bridge が必要な場合は `atctl bridge --symlink <PATH>`
  を使う。
- raw diagnostic evidence は、output path と `raw-log` acknowledgement を
  明示した場合だけ収集する。

## 安全

AT コマンドは機微な識別子を読み取ることがあり、modem state を変更すること
もあります。`atctl` は機微な出力を default で mask し、state-changing
action には confirmation を要求し、raw diagnostic export は user が output
file を選び、raw export risk を acknowledgement した場合だけ作成します。
詳しくは
[docs/SAFETY.md](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
を参照してください。

## ドキュメント

利用者向け:

- [docs/INSTALL.md](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md): インストールと runtime prerequisites
- [docs/TROUBLESHOOTING.md](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md): USB / modem troubleshooting
- [docs/PRESETS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md): presets、Sequences、TOML formats、examples
- [docs/SAFETY.md](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md): AT command safety と data handling

メンテナ向け:

- [docs/DEVELOPMENT.md](https://github.com/uchimanajet7/atctl/blob/main/docs/DEVELOPMENT.md): local development setup と verification
- [docs/PACKAGING.md](https://github.com/uchimanajet7/atctl/blob/main/docs/PACKAGING.md): Homebrew と release packaging

仕様・進捗:

- [docs/SPEC.md](https://github.com/uchimanajet7/atctl/blob/main/docs/SPEC.md): implementation specification と source of truth
- [docs/OPEN-QUESTIONS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/OPEN-QUESTIONS.md): approval が必要な decisions
- [docs/IMPLEMENTATION-STATUS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/IMPLEMENTATION-STATUS.md): implementation progress と resume state

## Shared presets

Presets は TUI 専用ではなく、application-level workflow です。同じ loaded
preset set が次の surface で使われます。

- `atctl preset list`
- `atctl preset run <NAME>`
- `atctl tui`

これにより、modem、carrier、project-specific AT command を一度定義し、CLI
で確認し、script から直接実行し、TUI から interactive に利用できます。同じ
masking、risk classification、confirmation、timeout behavior が適用されます。
Product presets と file presets は origin を分けますが、validation 後は同じ
loaded preset contract を使います。CLI listing には preset set label と
`source-path` column が含まれます。Product preset は source path に `-` を
使い、file preset はその row を提供した file path を表示します。`preset run`
も、file preset 実行時には USB access の前に source label、file path、review
notice を表示します。これは non-interactive な
`--yes --risk-ack <risk>` 実行でも同じです。TUI は built-in-only view を
簡潔に保ち、file preset は non-selectable source group header と
`Source: <title>` details で、必要な場合だけ区別します。file-level TOML
`title` をそのまま使い、`Add-on:` prefix は付けません。Categories は workflow
categories のままです。preset set title、vendor、file-origin label は category
list に混ぜません。

File preset は `~/.config/atctl` から自動 discovery されません。
repository-managed example や project-local preset は、現在の invocation で
明示的に読み込みます。

```sh
atctl preset list --preset-dir ./presets
atctl preset run my-command --preset-file ./presets/custom.toml
atctl tui --preset-dir ./presets
```

## Sequences

Sequences は one-shot preset に対応する multi-step AT operation です。preset は
1 つの AT command line を保存します。Sequence は prompt wait、payload write、
URC wait、per-step timeout、result transcript を扱えます。

同じ loaded Sequence set が次の surface で使われます。

- `atctl sequence list`
- `atctl sequence run <SEQUENCE>`
- `atctl tui`

Product-provided standard Sequences、repository-managed example Sequences、
user-authored Sequences は origin と review responsibility を分けます。読み込み
後は同じ Sequence loader validation、duplicate-name rejection、execution
engine、masking、risk aggregation、transcript、raw diagnostic export、surface
display contract を共有します。

Sequence add-on は `~/.config/atctl` から自動 discovery されません。
repository example、review、project-local extension には explicit
per-invocation Sequence location を使います。

```sh
atctl sequence list --sequence-dir ./sequences
atctl sequence run custom-sequence --sequence-file ./sequences/custom.toml
atctl tui --sequence-dir ./sequences
```

CLI `sequence list` には `source-path` column が含まれます。Product Sequence は
`-` を使い、file Sequence はその row を提供した file path を表示します。
`sequence run` は、file Sequence 実行時には USB access の前に source label、
file path、review notice を表示します。これは non-interactive な
`--yes --risk-ack <risk>` 実行でも同じです。

標準 SMS send/read/reply check は product-provided standard Sequence です。
通常の product workflow で user-authored Sequence TOML を先に作成する必要は
ありません。SMS send は USB access 前に destination と message body を review
します。SMS read は SMS storage index を review し、supported body を decode
しながら normal output を masked のまま保ちます。SMS reply は SMS storage
index と reply body を review し、`AT+CMGR` が返した元メッセージの sender から
reply destination を導出して、standard `AT+CMGS` submit path を使います。
Response output、logs、saved output、JSON は default で masked のままです。
Quectel ping/TCP check と SORACOM ping/Unified Endpoint TCP check は
vendor/provider-specific であり、repository-managed example Sequence definition
として `examples/sequences/` に含まれます。利用する場合は
`--sequence-dir examples/sequences` で明示的に読み込みます。

これらの example は、confirmed Sequence run の中で `AT+QIACT?` により Quectel
PDP context state を確認し、すでに active な context を再利用します。
`AT+QIACT=<contextID>` を無条件に再送しません。TCP example は failure cleanup
として `AT+QICLOSE=<connectID>` なども Response transcript に表示します。
Fixed-length TCP payload step は declared payload bytes だけを送信し、
send-acknowledgement step は `AT+QISEND=<connectID>,0` counters により payload
が完全に acknowledged されたことを確認してから、Sequence を `Result: OK` で
完了できます。External application receipt は、non-empty response data または
destination-side logs による確認が別途必要です。repository-managed ping example
は `AT+QPING` を使います。received replies は IP または SORACOM network
reachability evidence であり、TCP payload delivery や destination application
proof ではありません。ping step は `+QPING:` result line を待ちます。command
accepted の `OK` だけを reachability success とは扱いません。

Product preset reference、file preset TOML format、loading rules、
repository-managed example preset files の詳細は
[docs/PRESETS.md](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
を参照してください。

## License

MIT. 詳細は [LICENSE](LICENSE) を参照してください。
