# liteweight-fps

The source code in this project is available under the terms of either the
[MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE).

`liteweight-fps` is an offline, single-player FPS for practicing against BOTs.
It uses `wgpu` and `winit` to render a low-poly arena and focuses on responsive
movement, aiming, and weapon handling.

## Game features

- **Free Training** and **Tracking Aim** practice modes.
- First-person mouse look, WASD movement, sprinting, jumping, and collision
  against arena walls and cover using AABB approximations.
- Three weapons with hitscan firing, ADS, recoil, reloads, and weapon switching:

  | Weapon | Magazine | Fire rate | Body damage | Reload |
  | --- | ---: | ---: | ---: | ---: |
  | Pistol | 7 | 180 RPM | 50 | 1.8 s |
  | Assault rifle | 30 | 600 RPM | 25 | 2.0 s |
  | Sniper rifle | 5 | 50 RPM | 100 | 2.8 s |

  Headshots deal twice the body damage. The sniper rifle's ADS-in and ADS-out
  times are 400 ms and 200 ms.
- Three BOT behaviors: Static, Strafe, and Chase. BOT decisions run at 20 Hz;
  movement uses the fixed simulation timestep.
- Separate head and body hitboxes. A defeated BOT respawns after two seconds at
  a free location at least 8 m from the player.
- A session tracks the current kill count, shown in the HUD.
- Settings for mouse and ADS sensitivity, FOV, vertical inversion, remappable
  gameplay keys, window resolution, internal resolution scale, fullscreen, VSync,
  FPS limit, Free Training infinite ammo, audio volume, hit-marker strength, and
  BOT parameters.
- Slim, faceless mannequin BOTs with rounded low-poly body parts and an orange
  forehead marker.
- A HUD with ammo, reticle, and hit markers (thin red strokes for body hits,
  warmer red for headshots, and larger, heavier red strokes for kills), along
  with hit reactions, muzzle flashes, impact effects, simple weapon models,
  and synthesized sound effects.
- A resizable window and fullscreen toggle. The product renderer adjusts its
  internal render-target aspect and camera projection to the display, avoiding
  black bars without distorting or cropping the scene.

The sniper scope moves into alignment progressively. During ADS, the scope
opening grows and the area outside it darkens smoothly. Weapon switching has a
draw transition that lowers the old weapon before raising the new one.

## Run from source

The game is distributed as source. Build it locally with the Rust stable
toolchain; no prebuilt or signed binaries are provided. See
[Distribution](DISTRIBUTION.md) for platform requirements.

1. Install the native build tools for your OS.

   - macOS: run `xcode-select --install`.
   - Windows: install the MSVC Build Tools (Visual Studio "Desktop development with C++").
   - Linux (Debian/Ubuntu):
     ```sh
     sudo apt install build-essential pkg-config libasound2-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev
     ```

2. Install Rust stable:
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Clone and run:
   ```sh
   git clone --depth 1 https://github.com/issy20/liteweight-fps.git
   cd liteweight-fps
   cargo run --release --locked
   ```

The first build compiles all dependencies and takes a few minutes. The audio
device is optional: if none is available, the game reports the reason and
continues silently.

For convenience, `scripts/run.sh` (macOS/Linux) and `scripts/run.ps1` (Windows)
check for Rust and run `cargo run --release --locked` for you.

| Input | Action |
| --- | --- |
| Up / Down, Enter | Navigate and select menu items |
| Mouse movement | Look around; relative input is used while captured |
| W / A / S / D | Move forward / left / backward / right (rebindable) |
| Left Shift | Sprint while held (rebindable) |
| Space | Jump (rebindable) |
| Left mouse button | Fire; hold for automatic fire with the assault rifle |
| Right mouse button | Aim down sights while held |
| 1 / 2 / 3 | Select pistol / assault rifle / sniper rifle (rebindable) |
| R | Reload; reload takes priority over firing and is ignored when full (rebindable) |
| Esc | Pause and release the mouse; go back or resume in menus (rebindable) |
| Left / Right, Enter | Adjust the selected setting / confirm a menu item |
| Window close button | Close the game (or choose Quit in the title menu or Exit in the pause menu) |

You can resize the window by dragging its edges. Rendering pauses while the
window is minimized, hidden, or has zero size, and resumes when it becomes
visible again. Resizing the window updates the internal render-target dimensions
to match the display aspect without letterboxing. The internal resolution scale
controls pixel density while preserving that aspect ratio.

## Build and test

The project uses `wgpu` 24 and `winit` 0.30 for the game; Miniquad 0.4.11 is
used only by the benchmark binary behind the optional `bench` feature.
Dependencies are locked in `Cargo.lock`. The M0 renderer comparison uses these
pinned versions as its baseline; results from different library versions should
not be mixed.

On Linux, install a C compiler, `pkg-config`, ALSA development files, and the
Wayland/XKB development libraries. For Debian or Ubuntu:

```sh
sudo apt install build-essential pkg-config libasound2-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev
```

On macOS, install Xcode Command Line Tools. On Windows, install the MSVC Build
Tools. The audio device is optional at runtime: if initialization fails, the
game reports the reason and continues silently.

Run formatting, lint, tests, and a release build with:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release --bin liteweight-fps
```

The `bench` feature is only needed for the M0 benchmark binaries and RSS memory
sampling. Build them with:

```sh
cargo build --locked --release --features bench --bins
```

GPU integration tests are ignored during a normal `cargo test` run. To run them
on a machine with a supported GPU:

```sh
cargo test --test m1_gpu --test m3_gpu -- --ignored
```

The tests fail if they cannot acquire a GPU adapter; they do not report that
case as a pass. A short window lifecycle smoke test is available with:

```sh
cargo run --release -- --smoke
```

It checks zero-size surface handling and must finish within 30 seconds. It does
not replace manual testing of minimizing and restoring the window.

The repository includes internal benchmark, packaging, and dependency-notice
scripts for maintainers. Run their unit tests and regenerate the dependency
license inventory with:

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/dependency_notices.py --output target/THIRD_PARTY_NOTICES.txt
```

## Release status

The game is distributed as source only; no prebuilt or signed binaries are
provided. Build it with the Rust stable toolchain, so local builds are not
subject to OS download warnings. Representative hardware performance
measurements and runtime checks on every target OS are not complete. Do not
interpret the build as a guarantee of performance or verified compatibility
across all listed systems.

## Source layout

- `src/world/` — Static arena geometry and meshes.
- `src/engine/layout.rs` — Internal resolution and display layout.
- `src/engine/renderer.rs` — 3D rendering and presentation, independent of
  window and game state.
- `src/engine/window.rs` — GPU and surface initialization, resizing, and
  presentation.
- `src/simulation/player.rs` — First-person view, movement, jumping, gravity,
  and collision.
- `src/simulation/time.rs` — Fixed-timestep simulation.
- `src/physics/mod.rs` — AABB collision checks.
- `src/simulation/weapon.rs` — Weapon data, loadout, firing, ADS, recoil,
  reload, and bolt state.
- `src/simulation/bot.rs` — BOT behaviors, collision avoidance, health, and
  respawning.
- `src/simulation/combat.rs` — Hitscan, damage, stats, and safe respawn
  selection.
- `src/ui/` — HUD, menus, and BOT mesh generation.
- `src/engine/audio.rs` — Short synthesized sound effects using `rodio`.
- `src/main.rs` — Application entry point and event handling.

See the milestone plans for implementation and verification details:
[M1](docs/m1-plan.md), [M2](docs/m2-plan.md), [M3](docs/m3-plan.md),
[M4](docs/m4-plan.md), and [M5](docs/m5-plan.md).

## Renderer benchmark (M0)

M0 is a renderer comparison prototype for the Rust FPS project. The
`bench-wgpu` and `bench-miniquad` binaries draw the same low-poly 3D scene and
save results using a shared measurement format. They do not implement movement,
weapons, BOT AI, or game modes. The red humanoid is a static rendering dummy.

M0 is complete. `wgpu` + `winit` was selected for the game renderer. See the
[benchmark results](docs/m0-results.md), [acceptance criteria](docs/m0-acceptance.md),
and [benchmark notes](docs/benchmark.md). The user has visually and manually
verified both implementations' rendering and input behavior; those checks are
separate from performance measurements.

The M0 benchmarks remain available as a renderer regression baseline. M1/M2
can be compared against the same scene, camera, and measurement tool using
`--benchmark`. This mode does not run M5 gameplay systems such as weapons, BOTs,
HUD, or audio, so it does not measure overall game performance. The benchmark
binaries are gated behind the `bench` feature, so build them first:

```sh
cargo build --locked --release --features bench --bins
./target/release/bench-wgpu --warmup 5 --seconds 30 --output results/m0-reference
./target/release/liteweight-fps --benchmark --warmup 5 --seconds 30 --output results/m1-candidate
```

On macOS, run three alternating trials with:

```sh
caffeinate -di python3 scripts/compare_m1.py --baseline target/release/bench-wgpu --output results/m1-regression
```

Benchmark mode does not support mouse input, resizing, or preview interaction.
Do not switch to another window while measuring.

### Run matched M0 measurements

Run the commands one at a time. In Windows PowerShell, use
`./target/release/bench-wgpu.exe` and `./target/release/bench-miniquad.exe`.
Keep the benchmark window in the foreground and do not run both benchmarks at
the same time.

```sh
./target/release/bench-wgpu --width 1280 --height 720 --dummies 5 --warmup 5 --seconds 30 --vsync on --output results/wgpu-720p-5-on-1
./target/release/bench-miniquad --width 1280 --height 720 --dummies 5 --warmup 5 --seconds 30 --vsync on --output results/miniquad-720p-5-on-1
```

Each run warms up for five seconds after the first draw submission, measures
for 30 seconds, and exits automatically. It writes a `.json` file containing
conditions, summary statistics, and RSS samples, plus a `.csv` file containing
per-frame intervals. Use a new output prefix for each trial; an existing prefix
would be overwritten. The default prefixes are `results/bench-wgpu` and
`results/bench-miniquad`.

1. Record OS, CPU, GPU, RAM, power mode, display refresh rate and scaling,
   driver, Rust version, and source revision.
2. Measure 720p and 1080p with 5, 10, and 20 dummies, with VSync both on and
   off. For 1080p, use `--width 1920 --height 1080`; set dummy count with
   `--dummies 10` or `--dummies 20`.
3. Run each condition at least three times, alternating A→B and B→A order.
   Keep background load and thermal state consistent.
4. Compare trials only when `comparison_valid` is true and the `scene`,
   resolution, dummy count, duration, VSync request, and
   `display_pixels_initial/final` match. Also compare the selected GPU and
   actual VSync behavior. A true value checks a single trial's conditions; it
   does not guarantee a fair comparison or that performance goals were met.
5. Record each trial and the median for average FPS, 1% low, p99, RSS, and
   startup time. Keep the frame CSVs to investigate stutter. A VSync-limited
   result of 60 FPS alone does not show performance headroom or which renderer
   is faster.

### Matched rendering conditions

- Shared vertex and u16 index data: a 50 m square floor, four walls, six cover
  objects, and two boxes per dummy.
- 252 triangles with five dummies. No textures, lighting calculations,
  shadows, MSAA, blending, or face culling. Per-face vertex colors provide
  simple shading.
- The same 70-degree vertical FOV, near/far planes of 0.1/100, and depth test
  (`Less`) with depth writes enabled. Only the OpenGL depth range is converted.
- Internal rendering uses a fixed-size RGBA8 offscreen texture. Each frame has
  one scene draw and one nearest-neighbor presentation draw, two draws total.
- Both windows use a 960×540 logical size with HiDPI enabled. Actual pixel
  dimensions are recorded in JSON. Use the same display and scaling; resizing
  during a run invalidates the comparison.
- After warmup, both use the same real-time camera loop (20 m radius, 8 m
  height, 0.2 rad/s), so the camera does not move faster at higher FPS. Do not
  mix input mode results with automated benchmark results.
- No frame cap is set. Both renderers receive the same VSync request.

### Limitations when comparing renderers

- `wgpu` uses the adapter-selected API (Metal on macOS); Miniquad uses OpenGL
  on all operating systems. The GPU and API may differ, so record the backend
  and selected GPU in the JSON.
- Miniquad 0.4.11's macOS OpenGL path uses an approximately 60 Hz draw timer,
  and may not apply swap interval requests as expected. `--vsync off` does not
  guarantee uncapped rendering. Check frame intervals and exclude trials where
  presentation behavior does not match.
- Miniquad/OpenGL depth format is implementation-dependent; `wgpu` uses
  Depth32Float. This leaves a bandwidth difference.
- Miniquad 0.4.11's Metal offscreen path stopped on an API validation format
  mismatch and is not used. See the [benchmark notes](docs/benchmark.md).
- `wgpu` prefers a non-sRGB presentation format. If unavailable, record the
  selected surface format in the run notes. Differences in buffering, draw
  submission, and presentation are part of the comparison.

### Measurement definitions

| Output | Definition and limitations |
| --- | --- |
| `frame_statistics.average_fps` | Number of valid completed draw-submission intervals divided by their total duration. This is not the number of frames actually displayed. |
| `p99_ms` | Nearest-rank 99th percentile of sorted frame intervals. |
| `one_percent_low_fps` | Reciprocal of the average interval among the slowest `ceil(N × 0.01)` frames. |
| `cpu_submit_mean_ms` | Wall time from the start of the draw function until submission/presentation returns. Includes GPU, driver, and VSync waits; it is not pure CPU time. |
| `gpu_frame_ms` | Not measured; `null`. GPU timestamp queries are not implemented. |
| `rss_samples_bytes` | Process RSS sampled every second after warmup, in bytes; depends on the OS implementation in `sysinfo`. |
| `rss_sampled_max_bytes` | Maximum sampled RSS, not the process lifetime peak or dedicated VRAM usage. |
| `startup_to_first_submit_return_ms` | Time from `main` start until the first draw-submission API returns. Excludes process creation, dynamic loader time, and actual display presentation. |
| `mouse` | Number of relative input events while captured, accumulated delta, and final view angles. For manual input checks only. |

Only intervals whose endpoints both fall inside the measurement period are
included; intervals crossing its boundary are excluded. RSS sampling overhead
affects the next frame interval for both implementations. CSV
`elapsed_seconds` starts at the end of warmup. A run is invalid if it ends
early, loses focus when detectable, changes size, encounters a surface error,
uses a debug build or input mode, or has no samples. Manually exclude
interruptions the OS does not report.

### Manual mouse-input check

```sh
./target/release/bench-wgpu --mode input --warmup 0 --seconds 60 --output results/wgpu-input
./target/release/bench-miniquad --mode input --warmup 0 --seconds 60 --output results/miniquad-input
```

Click the left mouse button to capture, press Esc to release, and press Q to
save and exit. Each run exits after 60 seconds. Relative mouse deltas are
multiplied directly by 0.002 rad/unit, without multiplying by `dt`, smoothing,
or additional acceleration. Vertical view is clamped to ±1.55 rad. OS-specific
delta units and acceleration must be checked on real hardware.

- After capture, check whether the cursor is hidden and can move beyond screen
  edges.
- Compare small movements, fast flicks, and reversals. Confirm event count and
  accumulated delta are saved.
- Test Esc and recapture, focus changes, minimize/restore, and release on Q or
  window close.
- Repeat with VSync on and off. Input event records are not end-to-end latency
  measurements. Latency remains unmeasured unless separately tested, for
  example with a high-speed camera.

winit tries `Locked` cursor mode and falls back to `Confined`; failures are
recorded in run notes. Miniquad's capture API does not report success, so
verify capture and release manually.

On Linux, add `--window-system x11` or `--window-system wayland` to **both**
commands and measure each session separately. Automatic selection may differ.
Miniquad's Wayland support is marked unstable upstream; do not claim support
without verifying it. Windows and macOS use automatic window-system selection.

### M0 repeated acceptance measurements

The [M0 acceptance table](docs/m0-acceptance.md) defines required measurements
and manual checks. The scripts use only the Python 3.9+ standard library and
run 24 trials sequentially (about 14 minutes). Do not switch windows, minimize,
or move the benchmark window during measurement.

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/benchmark.py --output results/m0-session --dry-run
python3 scripts/benchmark.py --output results/m0-session
```

The output directory must be new. `manifest.json` records source and binary
SHA-256 hashes, run order, and commands; trial JSON/CSV files and logs are also
saved. The script stops on failures or `comparison_valid: false` and does not
modify raw data. If interrupted, record the cause and rerun using a new output
directory.

On macOS, prefix the command with `caffeinate -di` to prevent display sleep
during the run; the setting is released when the command exits. Use a new
output directory to retry selected conditions:

```sh
python3 scripts/benchmark.py --output results/m0-retry --height 1080 --dummies 20
python3 scripts/summarize.py results/m0-session
```

Summaries use the median of three trials for each metric. The summarizer rejects
invalid trials and mismatched conditions or display dimensions.
`comparison_valid` alone does not guarantee actual presentation or VSync
behavior, so inspect raw data and the real device state for implausible
submission rates.

## Additional references

- [Specification](SPECIFICATION.md)
- [Test list](docs/test-list.md)
- [M0 benchmark results](docs/m0-results.md)
- [Distribution instructions](DISTRIBUTION.md)
- [M8 release verification plan](docs/m8-plan.md)

Primary API references: [wgpu 24](https://docs.rs/wgpu/24.0.5/wgpu/),
[winit 0.30](https://docs.rs/winit/0.30.13/winit/), and
[Miniquad 0.4.11](https://docs.rs/miniquad/0.4.11/miniquad/).
