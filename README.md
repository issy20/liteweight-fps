# liteweight-fps

本プロジェクトのソースコードは、`LICENSE-MIT`または`LICENSE-APACHE`の条件で利用できます。

## オフラインのBOT射撃トレーニング

wgpu+winitでアリーナを描画するオフラインの一人称BOT射撃ゲームです。
起動後はタイトル画面でFree Training、5-Minute Challenge、Settingsを選択します。
ゲーム開始時にマウスを捕捉し、WASDで移動・Shiftでダッシュ・Spaceでジャンプできます。
壁・遮蔽物との衝突判定（AABB近似）があり、突き抜けません。
拳銃、AR、スナイパーの武器切替とヒットスキャンに加え、5体のBOTを相手にした自由練習を実装しています。
拳銃は7発・180 RPM・胴体50、ARは30発・600 RPM・胴体25、スナイパーは5発・50 RPM・胴体100ダメージです。頭部倍率は2倍。各武器のリロードは1.8／2.0／2.8秒。
スナイパーはADS-in 350ms / ADS-out 200ms。覗き込み中は小さなサイトピクチャーと暗い位置ずれの影が現れ、ADS進行に合わせて中央へ寄りながら開口部が広がります。ADSが進むほどスコープ外の視界が滑らかに暗くなり、完了時はほぼ黒くなります。武器ごとに形状の異なる簡易モデルを表示し、切替時は旧武器を下げてから新武器を構えるモーションが入ります。
BOTはStatic、Strafe、Chaseの3種類で、判断は20Hz、移動は固定シミュレーション刻みで更新します。
頭部と胴体は別々に命中判定し、死亡したBOTは2秒後に、プレイヤーから8m以上離れた空き地点へ復帰します。
命中数、ヘッドショット数、キル数を記録します。5分チャレンジ終了時は総合・武器別の結果を表示し、ローカルに最新50件を保存します。
Settingsではマウス／ADS感度、FOV、Free Trainingの無限弾薬、音量、BOT数・行動・HP・速度・リスポーン間隔を調整できます。
BOTは顔のない細身の人体模型です。丸みのある頭・胴体と先細りの手足を温かみのある中立色でまとめ、額の小さなオレンジ色マーカーで狙いどころを示します。
弾数、照準、ヒットマーカー（白:胴体／黄:頭部／赤:撃破）、被弾反応、マズルフラッシュ、
着弾表現、武器ごとの簡易モデルと合成効果音を実装しています。

**自動テスト、Clippy、Releaseビルドは確認済みです。Releaseでの手動起動確認と他OSでの実動作確認は未完了です。**
内部描画解像度は1280×720固定です。ウィンドウサイズ変更と設定メニューからのフルスクリーン切替に対応します。
内蔵GPUを含む実機性能は未測定のため、快適動作やFPSを保証しません。

```sh
cargo run --release
```

| 操作 | 動作 |
|---|---|
| 上下キー / Enter | タイトル・設定メニュー操作 |
| マウス移動 | 一人称視点（プレイ中は捕捉、相対入力） |
| W / A / S / D | 前後左右移動（斜めは正規化） |
| Shift | ダッシュ（押下中） |
| Space | ジャンプ |
| 左クリック | 射撃（ARは保持で連射） |
| 右クリック | ADS（押下中） |
| 1 / 2 / 3 | 拳銃 / AR / スナイパー切替 |
| R | リロード（射撃より優先、満弾時は無効） |
| Esc | 一時停止・マウス解放、メニュー中は戻る／再開 |
| メニュー上下 / Enter | 項目選択と決定。左右キーで選択中の設定を調整 |
| Q / 閉じるボタン | 終了 |

ウィンドウの端をドラッグして表示サイズを変更できます。
最小化・非表示・0サイズ中は描画を休止し、復帰時に再開します。
通常のウィンドウリサイズでは内部テクスチャを再作成しません。

```sh
# GPUのないCIでも実行できるテスト（既存M0/M1テストを含む）
cargo test --locked --all-targets
# 実機GPUで深度とリサイズ後の出力を読み戻して検証
cargo test --test m1_gpu --test m3_gpu -- --ignored
# 約5秒でウィンドウを自動変更して終了
cargo run --release -- --smoke
```

GPUテストは通常の`cargo test`ではignoreし、GPUを取得できる環境で明示実行します。
アダプターを取得できなければ失敗として報告し、成功扱いにはしません。
`--smoke`はAPI上の0サイズも確認し、30秒以内に完了しなければ失敗します。実際の最小化・復帰は別途確認が必要です。

- `src/world/`: 静的アリーナとメッシュ。M0のダミー付きシーンも同じ形状を使用。
- `src/engine/layout.rs`: 内部解像度と表示領域。
- `src/engine/renderer.rs`: ウィンドウやゲーム状態に依存しない3D描画と画面転送。
- `src/engine/window.rs`: GPU/Surface初期化、表示先リサイズ、提示。
- `src/simulation/player.rs`: 一人称プレイヤー（視点・移動・ジャンプ・重力・衝突）。
- `src/simulation/time.rs`: 固定タイムステップの管理。
- `src/physics/mod.rs`: AABB衝突判定。
- `src/simulation/weapon.rs`: 3武器のデータ、ロードアウト、発射・ADS・反動・リロード・ボルト状態。
- `src/simulation/bot.rs`: BOT行動・衝突回避・生存状態・リスポーン。
- `src/simulation/combat.rs`: 最前面ヒットスキャン・ダメージ・戦績・安全なリスポーン地点選択。
- `src/ui/`: BOTと最小HUDのメッシュ生成。
- `src/engine/audio.rs`: rodioによる短いオリジナル合成効果音。
- `src/main.rs`: M5 Free Trainingのイベント処理と操作。

[M1の計画・検証記録](docs/m1-plan.md)、[M2の計画・検証記録](docs/m2-plan.md)、[M3の計画・検証記録](docs/m3-plan.md)、[M4の計画・検証記録](docs/m4-plan.md)、[M5の計画・検証記録](docs/m5-plan.md)を参照してください。

### M1/M2の性能回帰測定

M0ベンチマークの描画実装は比較基準として保持しています。M1/M2は同じM0シーン・カメラ・計測器を使う
`--benchmark`モードで比較できます。このモードではM5の武器・BOT・HUD・音声は動かさないため、
M5ゲームプレイ全体の性能測定には使えません。

```sh
./target/release/bench-wgpu --warmup 5 --seconds 30 --output results/m0-reference
./target/release/liteweight-fps --benchmark --warmup 5 --seconds 30 --output results/m1-candidate
```

macOSで各3回の交互測定をする場合（出力先は新規ディレクトリのみ）:

```sh
caffeinate -di python3 scripts/compare_m1.py --baseline target/release/bench-wgpu --output results/m1-regression
```

benchmarkモードはマウス入力モードに対応せず、リサイズとプレビュー操作も無効にします。
性能比較中は他のウィンドウに切り替えないでください。

## M0: 描画基盤の比較

Rust製の軽量FPSに向けた**描画基盤の比較用プロトタイプ**です。
以下は[SPECIFICATION.md](SPECIFICATION.md)のM0比較用実装の説明です。
`bench-wgpu`と`bench-miniquad`は同じ低ポリゴン3Dシーンを描画し、共通の計測器で結果を保存します。
移動・射撃・BOT AI・ゲームモードは実装していません。赤い人型は描画負荷用の静的ダミーです。

**M0は完了し、M1以降の描画基盤にwgpu+winitを採用しました。** [反復測定結果](docs/m0-results.md)を参照してください。
両実装の目視・入力実機確認もユーザー確認済みです。実行確認と性能評価を区別し、結果・未測定事項を
[docs/benchmark.md](docs/benchmark.md)に記録します。

## ビルドと検証

Rustのstableツールチェーンと各OSのネイティブ開発環境が必要です。
音声用のrodio/cpalを追加しています。LinuxではALSA開発パッケージ（Debian/Ubuntu: `libasound2-dev`、`pkg-config`）、macOSではXcode Command Line Toolsが必要です。
音声デバイス初期化に失敗した場合は理由を標準エラーに表示し、無音で続行します。
開発時の検証環境はRust 1.98.1 / macOS Apple Siliconです。
WindowsではMSVC Build Tools、macOSではXcode Command Line Toolsを用意してください。
LinuxではCコンパイラ・pkg-config・Wayland/XKB開発ライブラリと、実行時に対応するGPUドライバーが必要です。
Debian/Ubuntuの例: `sudo apt install build-essential pkg-config libwayland-dev libxkbcommon-dev`。

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bins
```

依存版はCargo.lockに固定しています。wgpu 24 / winit 0.30、Miniquad 0.4.11をこの比較の基準とし、
ライブラリ更新時は結果を混ぜずに再計測します。最新バージョンの優劣を示す比較ではありません。
`block 0.1.6`の上流FFI警告は`vendor/block`の小さな互換パッチで解消しています。詳細は[パッチ記録](vendor/block/PATCHES.md)を参照してください。

## M8 Releaseパッケージ

Windows x64、Linux x64、macOS Apple Silicon / Intel向けに、実行ファイルと起動案内をまとめる
GitHub Actions workflowを用意しています。Releaseパッケージの作成、内容、OS要件は
[配布手順](DISTRIBUTION.md)、OS別の未検証項目と性能計測の状態は[M8検証計画](docs/m8-plan.md)を参照してください。
プレビューの機能と制限は[リリースノート](RELEASE_NOTES.md)を参照してください。RustSec依存監査はCargoファイルの変更時と週次に実行します。
パッケージ生成スクリプトのテストは次で実行できます。

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/dependency_notices.py --output target/THIRD_PARTY_NOTICES.txt
```

## 同一条件での測定

以下のコマンドを**一つずつ**実行します。Windows PowerShellでは
`./target/release/bench-wgpu.exe`、`./target/release/bench-miniquad.exe`に読み替えてください。
測定するウィンドウを前面に保ち、他方のベンチマークは同時に動かさないでください。

```sh
./target/release/bench-wgpu --width 1280 --height 720 --dummies 5 --warmup 5 --seconds 30 --vsync on --output results/wgpu-720p-5-on-1
./target/release/bench-miniquad --width 1280 --height 720 --dummies 5 --warmup 5 --seconds 30 --vsync on --output results/miniquad-720p-5-on-1
```

各実行は最初の描画送信から5秒ウォームアップし、その後30秒計測して自動終了します。
出力は指定prefixの`.json`（条件・集計・RSSサンプル）と`.csv`（全フレーム間隔）です。
同じprefixを再利用すると上書きされるため、試行番号を変えてください。
指定しない場合は`results/bench-wgpu`または`results/bench-miniquad`です。

1. OS/CPU/GPU/RAM、電源・省電力設定、ディスプレイ更新率・拡大率、ドライバー、Rust版、ソースの版を記録する。
2. 720p/1080p × ダミー5/10/20 × VSync on/offをそれぞれ測る。
   1080pは`--width 1920 --height 1080`、ダミー数は`--dummies 10`または`20`。
3. 各条件を最低3回、A→B / B→Aの順序を交互にして測る。バックグラウンド負荷と熱状態を揃える。
4. `comparison_valid`がtrueで、`scene`、解像度、ダミー数、時間、VSync要求、
   `display_pixels_initial/final`が両実装で一致している試行を選ぶ。GPU・実際のVSyncも照合する。
   trueは単体試行の条件チェックであり、比較可能性や性能目標達成の保証ではない。
5. 平均FPS、1% Low、p99、RSS、起動時間について各試行値と中央値を記録する。
   フレームCSVを残し、カクつきの原因を調べる。VSync onの60 FPSだけで余力や優劣を判断しない。

### 揃えている描画条件

- 共通の頂点・u16インデックス。50m角の床、4枚の壁、6個の遮蔽物、ダミー1体につき2個の箱。
- 5体で252三角形。テクスチャ素材・照明計算・影・MSAA・ブレンド・カリングなし。
  面ごとの頂点色で簡素な陰影を表現。
- 同じ70度の垂直FOV、near 0.1 / far 100、深度テストLess・深度書き込み有効。
  OpenGLの深度範囲のみ変換する。
- **内部描画は指定ピクセル数のRGBA8オフスクリーンテクスチャに固定**。
  シーン1 draw + 最近傍拡大の画面転送1 draw、合計2 draw / frame。
- 表示窓は両方960×540論理サイズ。HiDPI有効。実ピクセル寸法をJSONへ記録する。
  同じディスプレイ・拡大率で測定する。途中のサイズ変更は比較無効。
- ウォームアップ後、共通の実時間による周回カメラ経路（半径20m、高さ8m、0.2rad/s）。
  高FPS側だけ移動が速くなることはない。入力モードの値は自動ベンチマークと混ぜない。
- フレーム上限は設定しない。VSyncは両方に同じ値を要求する。

### 揃えられない項目と読み方

- wgpuはアダプターが選ぶAPI（macOSではMetal）、Miniquadは全OSでOpenGL。
  GPU/APIは同一とは限らず、JSONのbackendと実機のGPU選択を記録する。
- Miniquad 0.4.11のmacOS OpenGL経路には約60Hzの描画タイマーがあり、swap intervalも要求通りには適用されません。
  `--vsync off`でも無制限描画を保証しません。
  フレーム間隔で実効動作を確認し、同じ提示条件を確認できない試行は性能比較から除外する。
- Miniquad/OpenGLの深度形式は実装依存、wgpuはDepth32Float。帯域差は残る。
- Miniquad 0.4.11のMetalオフスクリーン描画はAPI検証で形式不一致により停止したため採用していません。
  詳細は[検証記録](docs/benchmark.md)を参照してください。
- 表示先はwgpuで非sRGBを優先。利用できない場合はサーフェス形式をnotesに記録する。
  API内部のバッファリング、描画送信・提示方式の違いも比較対象の一部。

## 計測値の定義

| 出力 | 定義・限界 |
|---|---|
| `frame_statistics.average_fps` | 有効な描画送信完了間隔の個数 ÷ 合計秒数。画面に実際に提示されたフレーム数ではない |
| `p99_ms` | フレーム間隔を昇順にしてnearest-rankの99パーセンタイル |
| `one_percent_low_fps` | 最も遅いceil(N×0.01)個のフレーム間隔平均の逆数 |
| `cpu_submit_mean_ms` | 描画関数開始から送信・提示APIが戻るまでの実時間。GPU/ドライバー/VSync待機を含み、純CPU時間ではない |
| `gpu_frame_ms` | 未測定、null。GPU timestamp queryは未実装 |
| `rss_samples_bytes` | 現在プロセスのRSSをウォームアップ後に1秒間隔で採取。bytes。sysinfoのOS実装による |
| `rss_sampled_max_bytes` | そのサンプルの最大値。プロセス全生存期間のピークや専用VRAM量ではない |
| `startup_to_first_submit_return_ms` | main開始から初回描画送信API終了まで。プロセス生成・動的ローダー・実際の画面提示を含まない |
| `mouse` | 捕捉中の相対入力イベント数・累積デルタ・最終視点角。実機入力確認用 |

フレーム区間の両端が計測区間に入ったものだけを採用し、境界をまたぐ間隔は除外します。
RSS取得の負荷は両実装とも次のフレーム間隔に含みます。
CSVの`elapsed_seconds`はウォームアップ終了からの時刻です。
途中終了、最小化/検出可能なフォーカス喪失、サイズ変化、サーフェスエラー、Debug実行、
入力モード、サンプルなしは比較無効です。OSが通知しない中断は手動で除外してください。

## マウス入力の実機確認

```sh
./target/release/bench-wgpu --mode input --warmup 0 --seconds 60 --output results/wgpu-input
./target/release/bench-miniquad --mode input --warmup 0 --seconds 60 --output results/miniquad-input
```

**左クリックで捕捉、Escで解除、Qで保存して終了**。60秒でも自動終了します。
相対マウスデルタへ同じ0.002rad/unitを直接掛けます。dt乗算・平滑化・追加の加速なし。
上下角は±1.55radで制限。OS由来のデルタ単位や加速の違いは実機で確認が必要です。

- 捕捉後、画面端を越えて回転できるか、カーソルが隠れるか。
- 小さい動き・高速フリック・往復で反応を比較し、イベント数と累積デルタが記録されるか。
- Esc→クリックの再捕捉、フォーカス切替、最小化・復帰、Q/ウィンドウの閉じる操作で解除されるか。
- VSync on/offで同じ操作を試す。mouseイベント記録は入力から画面までの遅延測定ではない。
  end-to-end遅延は未測定として、必要なら高速度カメラ等で別途測る。

winitはLockedを試し、非対応ならConfinedを試行。失敗はnotesへ記録します。
Miniquadの捕捉APIには成功結果がないため、実際の捕捉・解除を手動で確認してください。

Linuxでは**両コマンドに**`--window-system x11`または`--window-system wayland`を追加し、
各セッションで別々に測定します。auto時は選ばれる方式が異なる可能性があります。
MiniquadのWayland対応は上流でunstableとされているため、動作確認なしに対応済みとは扱いません。
Windows/macOSではautoのみ利用できます。

## 構成・参照

- `src/lib.rs`: 共通シーン、カメラ、入力、CLI、統計・出力
- `src/bin/bench-wgpu.rs` / `bench-miniquad.rs`: 描画・ウィンドウ処理
- `assets/shaders/`: 同等のWGSL / GLSLシェーダー
- `tests/core.rs`: レンダラーに依存しない自動テスト
- [テストリスト](docs/test-list.md)、[検証結果・技術選定](docs/benchmark.md)

APIの一次資料: [wgpu 24](https://docs.rs/wgpu/24.0.5/wgpu/)、
[winit 0.30](https://docs.rs/winit/0.30.13/winit/)、
[Miniquad 0.4.11](https://docs.rs/miniquad/0.4.11/miniquad/)。

## M0完了判定用の反復測定

[完了判定表](docs/m0-acceptance.md)に、今回の必須範囲と実機操作チェックを定義しています。
Python 3.9以降の標準ライブラリのみで、24試行を逐次実行できます（約14分）。
測定中は他のウィンドウに切り替えず、ウィンドウを最小化・移動しないでください。

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/benchmark.py --output results/m0-session --dry-run
python3 scripts/benchmark.py --output results/m0-session
```

出力ディレクトリは新規作成のみです。ソース・実行バイナリのSHA-256、実行順、コマンドを
`manifest.json`へ記録し、各試行のJSON/CSVとログを保存します。
失敗や`comparison_valid: false`を検出すると停止し、生データを変更しません。
中断した場合は原因を記録し、新しい出力ディレクトリで再実行してください。

macOSで測定中の画面スリープを防ぐ場合は、実行コマンドの先頭に`caffeinate -di`を付けます
（コマンド終了時に解除）。特定条件を取り直す場合は、新しい出力先を指定します。

```sh
python3 scripts/benchmark.py --output results/m0-retry --height 1080 --dummies 20
python3 scripts/summarize.py results/m0-session
```

集計は各指標の3試行の中央値です。無効な試行、条件や表示寸法の不一致を拒否します。
`comparison_valid`だけでは実際の表示状態やVSync動作を保証できないため、
不自然な送信FPSなどがないか、生データと実機状況も確認してください。
