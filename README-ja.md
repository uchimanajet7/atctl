# atctl

[English README](README.md)

`atctl` は、Apple Silicon搭載MacからUSBセルラーモデムへATコマンドを
送信・管理するためのRust製CLI/TUIです。

![デバイス情報、コマンド分類、コマンド一覧、応答、保存済みログを表示するatctl TUI](docs/assets/atctl-tui-overview.png)

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
[インストールガイド](docs/INSTALL.md)
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
[トラブルシューティング](docs/TROUBLESHOOTING.md)
を参照してください。

## 主なワークフロー

- `atctl tui`で対話的に操作する。
- `atctl send <COMMAND>`でATコマンドを1つ送信する。
- `atctl preset list`で繰り返し使う確認処理を探し、
  `atctl preset show <NAME>`で完全な内容を確認してから
  `atctl preset run <NAME>`で実行する。
- `atctl sequence list`でSMSやデータ送信の複数手順を探し、
  `atctl sequence show <SEQUENCE>`で完全な内容を確認してから
  `atctl sequence run <SEQUENCE>`で実行する。
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
[ログの確認と管理](docs/TROUBLESHOOTING.md#review-and-manage-logs)
を参照してください。

## 安全

通常のCLI一覧とTUI行は名前とATコマンドまたは目的のプレビューを表示し、
実行前に明示的な確認が必要な項目だけに`[confirm]`を追加します。このタグは
危険度ではなく次の操作を示し、TUI内の候補取得操作にも同じ規則を適用します。
確認あり／なしが混在する一覧では、CLI、TUI、TUI内の候補取得操作の
`[confirm]`を固定の右端レーンへ揃え、可変長のATコマンドや目的とは分けて
走査できます。TUIでは確認フロー専用色を補助的に使い、色なしでも完全な文字列と
太字を維持します。TUIで選択しても行の高さは変わりません。完全な実行レビューはCLIの`show`
またはTUIの`d`で読み取り専用表示でき、開くだけではモデムにアクセスしません。
TUIのDetailsとCLIの人向け`show`は、完全な情報をOverview、Definition、
Before running、Inputs、Executionに分け、空のセクションを省略します。
Sequenceの値入力ではSequenceを識別できる状態を保ち、現在の値、直接関係する支援、
候補、エラー、次の操作だけを表示します。最終レビューでは解決済みの値、
該当する実行前確認、実行結果への影響を表示します。レビューが長い場合は、
実行指示と現在の入力を表示したまま、Review領域だけを`PageUp`/`PageDown`で
ページ送りします。複数値を入力するSequenceでは、値の解決中にモーダル右上の
`Value <current> of <total>`で現在地を固定表示し、単一値と最終レビューでは
カウンターを表示しません。
候補付きのSequence値では、TUIと対話型`sequence run`が手入力に加えて既存の
実行項目を提示します。SMS storage indexには`sms-receive-check`、PDP context
IDには`active-pdp-contexts`／`pdp-contexts`を使い、候補専用に複製した
1コマンド処理へ置き換えません。TUIで読み込み済み候補に`Enter`を押すと、その
値を採用して次の値またはSequenceレビューへ同じキー操作で進みますが、選択中の
Sequenceは実行しません。候補providerの完了後も値を自動選択せず、値選択へ戻り
ます。値が不足しただけでは自動実行せず、非対話型とJSON実行ではproviderと必要な
`--param`再実行方法を表示します。
確認が必要な場合は、何が起きるかを先に示し、
機微データや復旧に関する情報は該当する場合だけ表示します。未知の操作は、既知の
事項、起こり得る影響、atctlが確認できなかった事項を分けて表示します。影響を把握
している操作の確認語は`run`、未知の操作の確認語は`unknown`であり、未知を既知の
影響と誤認しないようにします。

ATコマンドは機微な識別子を読み取ったり、モデムの状態を変更したりします。
`atctl`は機微な出力を初期状態でマスクし、状態変更など実質的な影響がある操作では
確認を求め、利用者が出力ファイルを選んで警告を確認した場合だけraw診断ファイルを作成
します。未確認のコマンドや状態変更コマンドを実行する前に、
[安全ガイド](docs/SAFETY.md)
を参照してください。
既知の電話番号・SMSアドレス欄は短い番号でもマスクし、index、status、
message referenceなど診断に必要な短い値は表示したままにします。

## プリセットとSequence

プリセットは1つのATコマンドを実行します。Sequenceは、SMSの送信・読み取り・
返信など、複数の手順からなる処理を実行します。製品に組み込まれたプリセットと
標準Sequenceは、CLIとTUIからそのまま利用できます。

```sh
atctl preset list
atctl preset show modem-info
atctl preset run modem-info
atctl sequence list
atctl sequence show sms-receive-check
atctl sequence run sms-receive-check
```

通常の一覧は、名前、ATコマンドまたは目的のプレビュー、確認が必要な場合の
`[confirm]`だけを1行で表示します。確認あり／なしが混在する場合はタグを固定の
右端レーンへ揃え、確認が必要な項目が1件もなければその領域をプレビューへ戻します。
`show`はUSBへアクセスせず、完全な内容を
読み取り専用で表示します。完全な構造化データが必要な場合は、`list`または
`show`に`--json`を追加します。

リポジトリ内の例やプロジェクト固有の定義を利用する場合は、現在のコマンドまたは
TUIセッションでファイルかディレクトリを指定します。

```sh
atctl preset list --preset-dir examples/presets
atctl sequence list --sequence-dir examples/sequences
atctl tui --preset-dir examples/presets --sequence-dir examples/sequences
```

外部定義は読み込み元を区別したまま、製品組み込み定義と同じ製品側の実行レビュー、
確認、マスキング、ログ、raw診断エクスポートの保護を受けます。定義ファイルに
単一のリスクラベルは記述しません。実行前に定義の入手元と送信先の値を確認してください。
プリセット一覧、TOML形式、読み込みオプション、
Sequenceのパラメーター、証跡の読み方は
[プリセットとSequenceのリファレンス](docs/PRESETS.md)
を参照してください。

## 利用者向けドキュメント

- [インストール](docs/INSTALL.md)
- [プリセットとSequenceのリファレンス](docs/PRESETS.md)
- [安全ガイド](docs/SAFETY.md)
- [トラブルシューティング](docs/TROUBLESHOOTING.md)

## 質問・報告

利用上の質問、不具合報告、機能提案は、既存の
[GitHub Issues](https://github.com/uchimanajet7/atctl/issues)を検索し、
該当するものがなければ新しいIssueを作成してください。

セキュリティ脆弱性の疑いは、公開Issueではなく
[GitHubの非公開脆弱性報告](https://github.com/uchimanajet7/atctl/security/advisories/new)から
報告してください。

不具合報告には、確認できる場合は`atctl`のバージョン、対象のコマンドまたは
TUI操作、再現手順、期待する結果、実際の結果を記載し、必要な場合だけ
確認済みのマスクされた出力を添えてください。公開Issueには、raw診断
エクスポート、マスクしていない出力、認証情報、加入者識別子、電話番号、
メッセージ本文を含めないでください。

## コントリビューション

コードおよびドキュメントのコントリビューションを受け付けています。
Pull Requestを作成する前に、
[コントリビューションガイド](CONTRIBUTING.md)を
参照してください。

## メンテナ向け情報

- [開発ガイド](docs/DEVELOPMENT.md)
- [パッケージングとリリース](docs/PACKAGING.md)
- [製品・技術仕様](docs/SPEC.md)
- [確定済みの製品・アーキテクチャ判断](docs/DECISIONS.md)

## ライセンス

MIT。詳細は[LICENSE](LICENSE)を参照してください。
