# M7: 操作感の磨き込み

M7ではフリック、トラッキング、スナイパーのクイックショット、連続入力を確認し、操作値を記録する。数値だけの自動検証では視覚・手触りの評価を代替しないため、Release版での手動確認結果も残す。

## 現在の調整値

| 項目 | 現在値 | 根拠 |
|---|---:|---|
| マウス入力 | 0.002 rad / raw delta × 感度倍率 | 相対入力を直接反映し、加速・平滑化・dt乗算を行わない |
| 通常感度 | 1.0 | 設定の既定値。ゲーム内で0.1〜5.0へ調整可能 |
| ADS感度倍率 | 0.65 | ADS進行度に応じて通常感度から連続補間 |
| 歩行 / ダッシュ | 4.0 / 6.5 m/s | M2の操作確認用初期値 |
| ジャンプ初速 / 重力 | 4.2 m/s / 9.8 m/s² | M2の操作確認用初期値 |
| AR ADS-in / ADS-out | 0.18 / 0.18 s | 武器データ |
| AR腰撃ち / ADS拡散 | 0.018 / 0.001 rad相当 | 移動中は2.5倍 |
| AR反動キック | 0.010 rad / shot | プレイフィードバックで旧値から約17%低下。射撃中にマウスで相殺でき、停止後に自動復帰しない |
| 拳銃反動キック | 0.045 rad / shot | 今回の調整対象外 |
| スナイパーADS-in / ADS-out | 0.50 / 0.20 s | 500msの覗き込み。照準収束は進行度の残差2.5乗 |
| スナイパー反動キック | 0.075 rad / shot | プレイフィードバックで旧値から約17%低下 |
| スナイパー腰撃ち照準ずれ | 4° | ADS進行度0.4で約1.1°まで収束し、ランダム拡散は使わない |

これらは現在の基準値であり、今回の自動テストから人間の最適値を推定したものではない。実機プレイで不具合が出ていない項目の値は、根拠なく変更しない。

## 自動検証

- [x] フリックの相対マウス反映とピッチ上限: `tests/m2.rs::look_updates_from_mouse_delta_and_clamps_pitch`
- [x] 通常 / ADS感度の連続補間と設定範囲: `tests/m6.rs::look_sensitivity_blends_from_normal_to_ads_multiplier`, `sensitivity_adjustment_changes_selected_value_and_respects_limits`
- [x] Strafe BOTの反転が滑らかで、行動別の移動が異なる: `tests/m5.rs::strafing_bot_reverses_direction_smoothly_for_tracking_practice`, `static_strafe_and_chase_behaviors_have_distinct_motion`
- [x] ARの腰撃ち拡散、ADS縮小、移動倍率: `tests/m3.rs`, `tests/m4.rs`
- [x] スナイパーはADS途中でも撃て、0.4付近から照準が早く収束する: `tests/m3.rs::ads_interpolates_allows_fire_and_yields_to_reload`, `tests/m4.rs::sniper_quick_shot_alignment_is_usable_before_half_ads`
- [x] 反動の相殺、移動・空中制御、短いクリック保持、武器切替中の射撃抑止: `tests/m2.rs`, `tests/m3.rs`, `tests/m4.rs`
- [x] AR・スナイパーの反動値を控えめに調整し、拳銃値を維持: `tests/m3.rs::ar_and_sniper_recoil_tuning_is_reduced_while_pistol_stays_unchanged`

## Release手動確認

`cargo run --release`で起動し、Free Trainingで確認する。感度はまず既定値1.0で試す。

- [x] 静止BOTへ拳銃で左右の高速フリックを繰り返し、止めたい位置で止められる。過剰な行き過ぎや入力遅れがない。
- [x] Strafe BOTをARで追い、腰撃ちとADSを確認。照準追従の操作感に問題なし。
- [x] スナイパーを覗き始めてADS途中から発射し、覗き切る前の弾道と、完了後の照準一致を確認。クロスヘアが遅れて表示される間も発射できる。
- [x] ARとスナイパーを射撃し、低下後の反動（AR 0.010 / sniper 0.075 rad per shot）を確認。ARの反動相殺と移動・ジャンプ・ADS・武器切替を組み合わせた操作に問題なし。

## 完了判定

自動検証と上記Release手動項目は完了。反動を調整したRelease版についてユーザーから「操作感は問題ありません」と確認を受けたため、M7を完了とする。5分連続プレイや他OSでの検証は別の品質確認項目として扱う。

## 検証記録（2026-10-08）

- `cargo fmt --check`: 成功。
- `cargo test` / `cargo test --release`: 成功。GPU実機を要するテストは通常どおりignore。
- `cargo clippy --all-targets --all-features -- -D warnings`: 成功。
- `cargo run --release -- --smoke`: macOSの通知サービス接続エラー後も終了せず、手動停止した。成功扱いにしない。
- ユーザーからAR・スナイパーの反動が少し強いとの報告を受け、各キックを約17%下げた。ユーザーがRelease版を操作し、変更後も操作感に問題ないことを確認した。
