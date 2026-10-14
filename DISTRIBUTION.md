# 配布

`liteweight-fps`は**ソース配布**を主とします。ビルド済みバイナリは配布しないため、macOSの公証やWindowsのAuthenticodeなど**署名は不要**です。利用者はRust stableツールチェーンで各自ビルドします。ローカルビルドの実行ファイルはOSのダウンロード検証（Gatekeeper / SmartScreen）の対象外です。

## 動作要件

- Rust stable（rustup）
- 各OSのネイティブビルド環境
  - macOS: Xcode Command Line Tools
  - Windows: MSVC Build Tools（Visual Studioの「C++によるデスクトップ開発」）
  - Linux: Cコンパイラ、`pkg-config`、ALSA、XKB、Waylandの開発パッケージ
- Linux: X11またはWayland、対応GPUドライバー
- 音声デバイスは任意。初期化できない場合は画面に通知して無音で続行します。

Debian/Ubuntuでのビルド前提:

```sh
sudo apt install build-essential pkg-config libasound2-dev libwayland-dev libxkbcommon-dev
```

## ビルドと起動

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone --depth 1 https://github.com/issy20/liteweight-fps.git
cd liteweight-fps
cargo run --release --locked
```

初回は依存のコンパイルで数分かかります。シェーダーとフォントは実行ファイルへコンパイル時に埋め込まれるため、実行時の`assets/`フォルダーは不要です。

## 設定と保存先

設定は実行ファイルの隣ではなく、OSのユーザー設定領域に保存されます。`settings.json`に設定を保存します。

- Windows: `%APPDATA%\liteweight-fps\`
- macOS: `~/Library/Application Support/liteweight-fps/`
- Linux: `${XDG_CONFIG_HOME:-~/.config}/liteweight-fps/`

## ライセンス

ソースはMIT OR Apache-2.0で提供します（`LICENSE-MIT` / `LICENSE-APACHE`）。埋め込みフォントはSIL Open Font Licenseです（`assets/fonts/OFL.txt`）。依存クレートのライセンスは各利用者がcrates.ioから取得するため、配布物への同梱は行いません。

## 検証状態

- 現行の性能は未測定です。内蔵GPUでの快適動作や最低FPSを示す表示・宣伝には使わないでください。
- Windows / Linux（X11・Wayland） / macOS の実機での起動・入力・表示・音声・終了は未検証です。CIのビルド・自動テスト結果は実機確認とは区別してください。
- 開発用に4ターゲットのパッケージを生成するスクリプト（`scripts/package_release.py`）とworkflowを残していますが、**公開リリースではバイナリを添付しません**。内部検証専用です。
