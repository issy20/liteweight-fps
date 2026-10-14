# 配布パッケージ

Releaseパッケージには実行ファイル、README、起動・動作要件、プロジェクトのMIT/Apache-2.0ライセンス本文、フォントのSIL Open Font License、Cargo.lock依存関係のライセンス表と入手できたライセンス本文を含めます。Linuxパッケージにはビルド環境の`ldd`結果も含みます。本文がcrate sourceに含まれない依存はNOTICEに明記し、公開前に確認してください。シェーダーとフォントは実行ファイルへコンパイル時に埋め込まれており、実行時の`assets/`フォルダーは不要です。

## 起動

- Windows: ZIPを展開して`liteweight-fps.exe`を実行します。
- macOS: tar.gzを展開してターミナルから`./liteweight-fps`を実行します。
- Linux: tar.gzを展開して`./liteweight-fps`を実行します。現在のCI成果物はUbuntu 22.04でビルドし、これを最低動作環境の暫定基準とします。X11またはWayland、対応GPUドライバー、実行時共有ライブラリが必要です。未確認の古いディストリビューションは対応対象としません。

Linuxで音声デバイスを初期化できない場合は、画面に通知して無音で続行します。動的リンクされたシステムライブラリとGPUドライバーはOS側に必要です。Release workflowは`ldd`の結果をログに残し、未解決ライブラリがあれば失敗します。ドラフトを公開する前にログの依存一覧を確認してください。

## ローカルでパッケージを作る

Rust stable、対象OSのネイティブビルド環境、Python 3が必要です。Linuxでは`pkg-config`、ALSA、XKB、Waylandの開発パッケージを用意します。

```sh
cargo build --locked --release --target aarch64-apple-darwin --bin liteweight-fps
python3 scripts/dependency_notices.py --output target/THIRD_PARTY_NOTICES.txt
python3 scripts/package_release.py --target aarch64-apple-darwin --release-dir target/aarch64-apple-darwin/release
```

ターゲットは`x86_64-pc-windows-msvc`、`x86_64-unknown-linux-gnu`、`aarch64-apple-darwin`、`x86_64-apple-darwin`です。出力先`dist/`に、WindowsはZIP、その他はtar.gzを作ります。同名の出力がある場合は上書きせず失敗します。
Linuxパッケージでは`ldd target/x86_64-unknown-linux-gnu/release/liteweight-fps | tee target/linux-runtime-dependencies.txt`を先に実行してください。

GitHub Actionsの「Release packages」workflowを手動実行すると、4ターゲットの検証用artifactを保存します。Cargoのバージョンと一致する`v*`タグをpushすると、SHA-256一覧を含む**ドラフトGitHub Release**を作成します。ドラフトは署名・実機検証・ライセンス確認が済むまで公開しないでください。

## 現在の配布範囲

これらは動作検証用の未署名プレビューパッケージです。macOSの署名・公証、WindowsのAuthenticode署名、最低GPUの確定、Windows/Linux/macOS各構成の実機確認は未完了です。プロジェクト本体のMIT/Apache-2.0本文は同梱しますが、NOTICEに本文未収録と記載された依存のライセンス通知は公開前に確認が必要です。これらの確認が終わるまでは一般公開しないでください。フォントのライセンス通知は各パッケージに含めます。

現行の性能は未測定です。内蔵GPUでの快適動作や最低FPSを示す表示・宣伝には使わないでください。

ゲーム設定は実行ファイルの隣ではなく、OSのユーザー設定領域に保存されます。

- Windows: `%APPDATA%\liteweight-fps\`
- macOS: `~/Library/Application Support/liteweight-fps/`
- Linux: `${XDG_CONFIG_HOME:-~/.config}/liteweight-fps/`

`settings.json`に設定を保存します。
