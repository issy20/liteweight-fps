pub mod engine;
pub mod physics;
pub mod simulation;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Stats {
    pub average_fps: f64,
    pub p99_ms: f64,
    pub one_percent_low_fps: f64,
}

pub fn stats(samples: &[f64]) -> Option<Stats> {
    if samples.is_empty() || samples.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let tail = n.div_ceil(100);
    Some(Stats {
        average_fps: 1000.0 * n as f64 / sorted.iter().sum::<f64>(),
        p99_ms: sorted[(n * 99).div_ceil(100) - 1],
        one_percent_low_fps: 1000.0 * tail as f64 / sorted[n - tail..].iter().sum::<f64>(),
    })
}
use glam::{Mat4, Vec3};

pub mod world;
pub use world::{scene, Scene, Vertex};

pub fn camera(time: f32, aspect: f32, gl_depth: bool, look: Option<(f32, f32)>) -> Mat4 {
    let (eye, target) = if let Some((yaw, pitch)) = look {
        let eye = Vec3::new(0., 3., 20.);
        (
            eye,
            eye + Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                -yaw.cos() * pitch.cos(),
            ),
        )
    } else {
        let angle = time * 0.2;
        (
            Vec3::new(angle.sin() * 20., 8., angle.cos() * 20.),
            Vec3::new(0., 1., 0.),
        )
    };
    let projection = if gl_depth {
        Mat4::perspective_rh_gl(70_f32.to_radians(), aspect, 0.1, 100.)
    } else {
        Mat4::perspective_rh(70_f32.to_radians(), aspect, 0.1, 100.)
    };
    projection * Mat4::look_at_rh(eye, target, Vec3::Y)
}

#[derive(Default, Serialize)]
pub struct Mouse {
    pub captured: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub events: u64,
    pub delta_sum: [f64; 2],
}
impl Mouse {
    pub fn motion(&mut self, dx: f64, dy: f64) {
        if self.captured && dx.is_finite() && dy.is_finite() {
            self.yaw += dx as f32 * 0.002;
            self.pitch = (self.pitch - dy as f32 * 0.002).clamp(-1.55, 1.55);
            self.events += 1;
            self.delta_sum[0] += dx;
            self.delta_sum[1] += dy;
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Options {
    pub width: u32,
    pub height: u32,
    pub dummies: u32,
    pub warmup: f64,
    pub seconds: f64,
    pub vsync: bool,
    pub input: bool,
    pub output: String,
    pub window_system: String,
}
impl Options {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut o = Self {
            width: 1280,
            height: 720,
            dummies: 5,
            warmup: 5.,
            seconds: 30.,
            vsync: true,
            input: false,
            output: "results/run".into(),
            window_system: "auto".into(),
        };
        if !args.len().is_multiple_of(2) {
            return Err("オプションには値が必要です".into());
        }
        for pair in args.as_chunks::<2>().0 {
            let value = &pair[1];
            let invalid = || format!("不正な引数: {} {}", pair[0], value);
            match pair[0].as_str() {
                "--width" => o.width = value.parse().map_err(|_| invalid())?,
                "--height" => o.height = value.parse().map_err(|_| invalid())?,
                "--dummies" => o.dummies = value.parse().map_err(|_| invalid())?,
                "--warmup" => o.warmup = value.parse().map_err(|_| invalid())?,
                "--seconds" => o.seconds = value.parse().map_err(|_| invalid())?,
                "--vsync" => {
                    o.vsync = match value.as_str() {
                        "on" => true,
                        "off" => false,
                        _ => return Err(invalid()),
                    }
                }
                "--mode" => {
                    o.input = match value.as_str() {
                        "auto" => false,
                        "input" => true,
                        _ => return Err(invalid()),
                    }
                }
                "--window-system" => match value.as_str() {
                    "auto" | "x11" | "wayland" => o.window_system.clone_from(value),
                    _ => return Err(invalid()),
                },
                "--output" => o.output.clone_from(value),
                _ => return Err(invalid()),
            }
        }
        if !(64..=3840).contains(&o.width)
            || !(64..=2160).contains(&o.height)
            || ![5, 10, 20].contains(&o.dummies)
            || !o.warmup.is_finite()
            || !(0.0..=60.0).contains(&o.warmup)
            || !o.seconds.is_finite()
            || !(1.0..=600.0).contains(&o.seconds)
            || o.output.is_empty()
        {
            return Err("解像度64〜3840×2160、ダミー5/10/20、warmup 0〜60秒、seconds 1〜600秒を指定してください".into());
        }
        Ok(o)
    }
    pub fn from_env() -> Self {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.iter().any(|s| s == "--help") {
            println!("M0 benchmark: --width 1280 --height 720 --dummies 5 --warmup 5 --seconds 30 --vsync on|off --mode auto|input --window-system auto|x11|wayland --output results/run\ninput: click=capture, Esc=release, Q=save and quit");
            std::process::exit(0);
        }
        let mut options = Self::parse(&args).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        });
        if !cfg!(target_os = "linux") && options.window_system != "auto" {
            eprintln!("--window-system x11/wayland はLinuxのみ対応します");
            std::process::exit(2);
        }
        if !args.iter().any(|s| s == "--output") {
            let name = std::env::args().next().unwrap_or_default();
            let name = std::path::Path::new(&name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("run");
            options.output = format!("results/{name}");
        }
        options
    }
}

#[derive(Serialize)]
pub struct Frame {
    pub elapsed: f64,
    pub interval_ms: f64,
    pub cpu_submit_ms: f64,
}
pub struct Measurement {
    pub frames: Vec<Frame>,
    pub valid: bool,
    warmup: f64,
    end: f64,
    size: (u32, u32),
    actual_size: (u32, u32),
    focused: bool,
}
impl Measurement {
    pub fn new(warmup: f64, seconds: f64, size: (u32, u32)) -> Self {
        Self {
            frames: Vec::with_capacity(120_000),
            valid: true,
            warmup,
            end: warmup + seconds,
            size,
            actual_size: size,
            focused: true,
        }
    }
    pub fn record(&mut self, previous: f64, now: f64, cpu_submit_ms: f64) {
        if previous >= self.warmup && now <= self.end && now > previous {
            if !self.focused {
                self.valid = false;
            }
            self.frames.push(Frame {
                elapsed: now - self.warmup,
                interval_ms: (now - previous) * 1000.,
                cpu_submit_ms,
            });
        }
    }
    pub fn focus(&mut self, focused: bool, elapsed: f64) {
        self.focused = focused;
        if !focused && elapsed >= self.warmup {
            self.valid = false;
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.actual_size = (width, height);
        if (width, height) != self.size {
            self.valid = false;
        }
    }
}

use std::{io::Write, time::Instant};
/// 両イベントループが呼ぶ同一の計測器。描画完了のGPU時間は計測しない。
pub struct Run {
    pub options: Options,
    pub mouse: Mouse,
    pub measurement: Measurement,
    pub backend: String,
    pub notes: Vec<String>,
    start: Instant,
    first_present: Option<Instant>,
    previous_present: Option<Instant>,
    startup_ms: Option<f64>,
    saved: bool,
    memory_system: sysinfo::System,
    memory: Vec<(f64, u64)>,
    last_memory: Option<Instant>,
}
impl Run {
    pub fn new(options: Options, start: Instant, backend: String) -> Self {
        let measurement = Measurement::new(
            options.warmup,
            options.seconds,
            (options.width, options.height),
        );
        Self {
            options,
            mouse: Mouse::default(),
            measurement,
            backend,
            notes: Vec::new(),
            start,
            first_present: None,
            previous_present: None,
            startup_ms: None,
            saved: false,
            memory_system: sysinfo::System::new(),
            memory: Vec::new(),
            last_memory: None,
        }
    }
    pub fn focus(&mut self, focused: bool) {
        let elapsed = self
            .first_present
            .map_or(-1., |t| t.elapsed().as_secs_f64());
        self.measurement.focus(focused, elapsed);
        if !focused {
            self.notes
                .push("focus lost or minimized (including startup transitions)".into());
        }
    }
    pub fn set_initial_display(&mut self, width: u32, height: u32) {
        self.measurement.size = (width, height);
        self.measurement.actual_size = (width, height);
    }
    pub fn matrix(&self, width: u32, height: u32, gl: bool) -> Mat4 {
        let elapsed = self.first_present.map_or(0., |t| t.elapsed().as_secs_f64());
        let time = (elapsed - self.options.warmup).max(0.) as f32;
        camera(
            time,
            width as f32 / height.max(1) as f32,
            gl,
            self.options
                .input
                .then_some((self.mouse.yaw, self.mouse.pitch)),
        )
    }
    pub fn presented(&mut self, cpu_start: Instant) -> bool {
        let now = Instant::now();
        let cpu_ms = now.duration_since(cpu_start).as_secs_f64() * 1000.;
        let first = *self.first_present.get_or_insert_with(|| {
            self.startup_ms = Some(now.duration_since(self.start).as_secs_f64() * 1000.);
            now
        });
        let elapsed = now.duration_since(first).as_secs_f64();
        if let Some(previous) = self.previous_present.replace(now) {
            self.measurement.record(
                previous.duration_since(first).as_secs_f64(),
                elapsed,
                cpu_ms,
            );
        }
        // 同じ頻度で現在プロセスだけを取得。計測コストは次フレーム間隔に含まれる。
        if elapsed >= self.options.warmup
            && self
                .last_memory
                .is_none_or(|t| now.duration_since(t).as_secs_f64() >= 1.)
        {
            if let Ok(pid) = sysinfo::get_current_pid() {
                self.memory_system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    true,
                    sysinfo::ProcessRefreshKind::nothing().with_memory(),
                );
                if let Some(p) = self.memory_system.process(pid) {
                    self.memory.push((elapsed, p.memory()));
                }
            }
            self.last_memory = Some(now);
        }
        elapsed >= self.options.warmup + self.options.seconds
    }
    pub fn save(&mut self, completed: bool) {
        if self.saved {
            return;
        }
        self.saved = true;
        if let Err(e) = self.write(completed) {
            eprintln!("計測結果の保存失敗: {e}");
            std::process::exit(1);
        }
    }
    fn write(&self, completed: bool) -> Result<(), Box<dyn std::error::Error>> {
        let path = std::path::PathBuf::from(format!("{}.json", self.options.output));
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let samples: Vec<_> = self
            .measurement
            .frames
            .iter()
            .map(|f| f.interval_ms)
            .collect();
        let cpu: Vec<_> = self
            .measurement
            .frames
            .iter()
            .map(|f| f.cpu_submit_ms)
            .collect();
        let report = serde_json::json!({
            "schema": 1, "scene": "m0-cubes-v1", "backend": self.backend,
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "release": !cfg!(debug_assertions), "options": self.options,
            "completed": completed, "comparison_valid": self.measurement.valid && completed && !self.options.input && !cfg!(debug_assertions) && !samples.is_empty(),
            "frame_count": samples.len(), "frame_statistics": stats(&samples),
            "cpu_submit_mean_ms": (!cpu.is_empty()).then(|| cpu.iter().sum::<f64>() / cpu.len() as f64),
            "gpu_frame_ms": null, "startup_to_first_submit_return_ms": self.startup_ms,
            "rss_samples_bytes": self.memory, "rss_sampled_max_bytes": self.memory.iter().map(|x| x.1).max(),
            "display_pixels_initial": self.measurement.size, "display_pixels_final": self.measurement.actual_size,
            "draw_calls_per_frame": 2, "triangles": scene(self.options.dummies).indices.len()/3,
            "mouse": self.mouse, "notes": self.notes,
        });
        std::fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
        let mut csv = std::io::BufWriter::new(std::fs::File::create(format!(
            "{}.csv",
            self.options.output
        ))?);
        writeln!(csv, "elapsed_seconds,frame_interval_ms,cpu_submit_ms")?;
        for f in &self.measurement.frames {
            writeln!(
                csv,
                "{:.9},{:.9},{:.9}",
                f.elapsed, f.interval_ms, f.cpu_submit_ms
            )?;
        }
        csv.flush()?;
        println!("保存: {} ({} frames)", path.display(), samples.len());
        Ok(())
    }
}

pub mod ui;
