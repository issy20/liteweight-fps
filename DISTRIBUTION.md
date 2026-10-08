# 配布パッケージ

Releaseパッケージには実行ファイル、README、起動・動作要件、同梱フォントのSIL Open Font Licenseを含めます。シェーダーとフォントは実行ファイルへコンパイル時に埋め込まれており、実行時の`assets/`フォルダーは不要です。

## 起動

- Windows: ZIPを展開して`liteweight-fps.exe`を実行します。
- macOS: tar.gzを展開してターミナルから`./liteweight-fps`を実行します。
- Linux: tar.gzを展開して`./liteweight-fps`を実行します。X11またはWaylandセッションと、対応GPUドライバーが必要です。

Linuxで音声デバイスを初期化できない場合は、ゲームは無音で続行します。動的リンクされたシステムライブラリとGPUドライバーはOS側に必要です。

## ローカルでパッケージを作る

Rust stable、対象OSのネイティブビルド環境、Python 3が必要です。Linuxでは`pkg-config`、ALSA、XKB、Waylandの開発パッケージを用意します。

```sh
cargo build --locked --release --target aarch64-apple-darwin --bin liteweight-fps
python3 scripts/package_release.py --target aarch64-apple-darwin --release-dir target/aarch64-apple-darwin/release
```

ターゲットは`x86_64-pc-windows-msvc`、`x86_64-unknown-linux-gnu`、`aarch64-apple-darwin`、`x86_64-apple-darwin`です。出力先`dist/`に、WindowsはZIP、その他はtar.gzを作ります。同名の出力がある場合は上書きせず失敗します。

GitHub Actionsの「Release packages」workflowを手動実行するか、`v*`タグをpushすると、同じ4ターゲットのパッケージをworkflow artifactとして保存します。artifactを取得し、対象OSの実機で起動・入力・音声・終了を確認してから配布してください。

## 現在の配布範囲

これらは動作検証用の未署名パッケージです。macOSの署名・公証、最低OS/GPUの確定、Windows/Linux/macOS各構成の実機確認、プロジェクト本体の配布ライセンス決定は未完了です。フォントのライセンス通知は各パッケージに含めます。

ゲーム設定と結果は実行ファイルの隣ではなく、OSのユーザー設定領域に保存されます。

- Windows: `%APPDATA%\liteweight-fps\`
- macOS: `~/Library/Application Support/liteweight-fps/`
- Linux: `${XDG_CONFIG_HOME:-~/.config}/liteweight-fps/`

`settings.json`は設定、`results.json`はチャレンジ結果です。
