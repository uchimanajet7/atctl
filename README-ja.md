# atctl

[English README](https://github.com/uchimanajet7/atctl/blob/main/README.md)

`atctl` は、Apple Silicon搭載MacからUSBセルラーモデムへATコマンドを
送信・管理するためのRust製CLI/TUIです。

![デバイス情報、コマンド分類、コマンド一覧、応答、保存済みログを表示するatctl TUI](https://github.com/uchimanajet7/atctl/raw/main/docs/assets/atctl-tui-overview.png)

検証済み環境:

- Mac: Apple Silicon搭載Mac
- USBモデム: SORACOM Onyx LTE USB Dongle（Quectel EG25-G）
- USB ID: `0x2c7c:0x0125`
- 確認したファームウェア: `EG25GGBR07A08M2G`（`ATI`、2026-06-17）

この環境では、次のワークフローを実機で確認しています。

- `devices`によるUSBデバイス検出、`inspect`によるディスクリプター確認、
  `AT` / `ATI`の直接実行
- プリセット実行と、マスク済み履歴・セッションログ
- TUIでの候補更新と関連するSequence入力操作
- `screen`を使ったPTYブリッジ操作
- SMSの送信、受信・一覧、読み取り、返信、およびQuectel / SORACOMの
  ping・TCP Sequence

## インストール

Homebrewで`atctl`をインストールします。

```sh
brew install uchimanajet7/atctl/atctl
```

Homebrew Formulaは実行時依存関係の`libusb`もインストールします。必要条件と
インストール後の確認方法は
[インストールガイド](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
を参照してください。

## 最初の操作

モデムが表示され、ATコマンドに応答することを確認します。

```sh
atctl devices
atctl inspect
atctl send AT
atctl send ATI
atctl tui
```

現在のUSB接続先は`atctl devices`で確認します。想定したモデムが表示されない
場合は`atctl devices --all-usb`を実行し、
[トラブルシューティング](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md)
を参照してください。

## 主なワークフロー

- `atctl tui`で対話的に操作する。
- `atctl send <COMMAND>`でATコマンドを1つ送信する。
- `atctl preset list`と`atctl preset run <NAME>`で繰り返し使う確認処理を
  実行する。
- `atctl sequence list`と`atctl sequence run <SEQUENCE>`でSMSやデータ送信の
  複数手順を実行する。
- `atctl bridge --symlink <PATH>`でターミナル用PTYブリッジを利用する。
- raw診断証跡は、出力先と`raw-log`の確認語を明示した場合だけ収集する。

## ログ

`atctl send`、`atctl preset run`、`atctl sequence run`、TUIでの実行は、
マスク済みのコマンド履歴とセッションログを初期状態で書き込みます。

```text
~/.local/state/atctl/history.jsonl
~/.local/state/atctl/logs/<timestamp>.session.log
```

この保存先は
[XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)
に従います。実行単位で保存先を変更する場合は、`XDG_STATE_HOME`に空ではない
絶対パスを指定します。`atctl`は指定したパスへ`atctl`ディレクトリを追加します。

```sh
env XDG_STATE_HOME="$HOME/Documents" atctl send AT
env XDG_STATE_HOME="$HOME/Documents" atctl logs list
```

この例では`$HOME/Documents/atctl/`を使用します。現在のコマンドまたはTUI
セッションで新しいマスク済み履歴・セッションログを書き込まない場合は、
`--no-log`を指定します。

```sh
atctl send AT --no-log
atctl preset run modem-info --no-log
atctl sequence run sms-receive-check --no-log
atctl tui --no-log
```

`--no-log`は既存ログを非表示にせず、`--raw-log-file`またはTUIのraw診断
エクスポート操作で明示的に指定したraw診断ファイルも無効にしません。
通常のマスク済みログは、利用者が削除するまでXDG stateディレクトリに保持され、
保持期限や自動ローテーションは適用されません。TUIでは選択中のログを直接
Finderで表示できます。現在のResponseを別ファイルとして保持するには、TUIで
`Export response...`を選んで保存先フォルダーを指定するか、`send`、
`preset run`、`sequence run`に`--export-response <PATH>`を指定します。
Response exportは選択中のforeground masking modeに従い、新しいファイルだけを
作成します。通常の標準出力や自動生成されるmasked logは置き換えません。
TUIのResponseがunmaskedの場合、copyには`copy`、exportには保存先選択後の
`export`確認が必要です。
CLIとTUIでの確認、Response export、Finderでの表示、削除による影響は、
[ログの確認と管理](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md#review-and-manage-logs)
を参照してください。

## 安全

コマンドとSequenceの行には、`[safe]`、`[sensitive]`、`[write]`、
`[persistent]`、`[dangerous]`、`[unknown]`のいずれか1つだけを表示します。
output maskingの状態と必要な確認操作は別に表示します。

ATコマンドは機微な識別子を読み取ったり、モデムの状態を変更したりします。
`atctl`は機微な出力を初期状態でマスクし、状態を変更する操作では確認を求め、
利用者が出力ファイルを選んでリスクを確認した場合だけraw診断ファイルを作成
します。未確認のコマンドや状態変更コマンドを実行する前に、
[安全ガイド](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
を参照してください。

## プリセットとSequence

プリセットは1つのATコマンドを実行します。Sequenceは、SMSの送信・読み取り・
返信など、複数の手順からなる処理を実行します。製品に組み込まれたプリセットと
標準Sequenceは、CLIとTUIからそのまま利用できます。

```sh
atctl preset list
atctl preset run modem-info
atctl sequence list
atctl sequence run sms-receive-check
```

リポジトリ内の例やプロジェクト固有の定義を利用する場合は、現在のコマンドまたは
TUIセッションでファイルかディレクトリを指定します。

```sh
atctl preset list --preset-dir examples/presets
atctl sequence list --sequence-dir examples/sequences
atctl tui --preset-dir examples/presets --sequence-dir examples/sequences
```

外部定義は読み込み元を区別したまま、製品組み込み定義と同じリスク判定、確認、
マスキング、ログ、raw診断エクスポートの保護を受けます。実行前に定義の入手元と
送信先の値を確認してください。プリセット一覧、TOML形式、読み込みオプション、
Sequenceのパラメーター、証跡の読み方は
[プリセットとSequenceのリファレンス](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
を参照してください。

## 利用者向けドキュメント

- [インストール](https://github.com/uchimanajet7/atctl/blob/main/docs/INSTALL.md)
- [プリセットとSequenceのリファレンス](https://github.com/uchimanajet7/atctl/blob/main/docs/PRESETS.md)
- [安全ガイド](https://github.com/uchimanajet7/atctl/blob/main/docs/SAFETY.md)
- [トラブルシューティング](https://github.com/uchimanajet7/atctl/blob/main/docs/TROUBLESHOOTING.md)

## コントリビューション

コードおよびドキュメントのコントリビューションを受け付けています。
Pull Requestを作成する前に、[コントリビューションガイド](CONTRIBUTING.md)を
参照してください。

## メンテナ向け情報

- [開発ガイド](https://github.com/uchimanajet7/atctl/blob/main/docs/DEVELOPMENT.md)
- [パッケージングとリリース](https://github.com/uchimanajet7/atctl/blob/main/docs/PACKAGING.md)
- [製品・技術仕様](https://github.com/uchimanajet7/atctl/blob/main/docs/SPEC.md)
- [確定済みの製品・アーキテクチャ判断](https://github.com/uchimanajet7/atctl/blob/main/docs/DECISIONS.md)

## ライセンス

MIT。詳細は[LICENSE](LICENSE)を参照してください。
