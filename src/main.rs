use liteweight_fps::{
    engine::window::WindowRenderer,
    physics::Aabb,
    simulation::{player::Input, time::FixedStep, Player},
    world, Options, Run,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

struct App {
    renderer: Option<WindowRenderer>,
    benchmark: Option<Run>,
    smoke: bool,
    smoke_step: usize,
    smoke_started: Option<Instant>,
    clock: Instant,
    player: Player,
    colliders: Vec<Aabb>,
    input: Input,
    capture_requested: bool,
    captured: bool,
    fixed: FixedStep,
    occluded: bool,
    suspended: bool,
    error: Option<String>,
}
impl App {
    fn finish(&mut self, event_loop: &ActiveEventLoop, completed: bool) {
        if let Some(run) = &mut self.benchmark {
            run.save(completed);
        }
        event_loop.exit();
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        eprintln!("{message}");
        self.error = Some(message);
        self.finish(event_loop, false);
    }
    /// 捕捉が要求されていれば再試行する。フォーカス確定後に呼ばれる想定。
    fn try_capture(&mut self) {
        if !self.capture_requested || self.captured {
            return;
        }
        let Some(r) = &self.renderer else {
            return;
        };
        let result = r
            .window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| r.window.set_cursor_grab(CursorGrabMode::Confined));
        match result {
            Ok(()) => {
                r.window.set_cursor_visible(false);
                self.captured = true;
            }
            Err(e) => eprintln!("cursor grab failed: {e}"),
        }
    }
    fn release_capture(&mut self) {
        if let Some(r) = &self.renderer {
            let _ = r.window.set_cursor_grab(CursorGrabMode::None);
            r.window.set_cursor_visible(true);
        }
        self.captured = false;
    }
    fn update_title(&self) {
        if let Some(r) = &self.renderer {
            let (w, h) = r.internal_size();
            r.window.set_title(&format!(
                "liteweight-fps M2 | {w}×{h} | WASD移動 Shift:ダッシュ Space:ジャンプ Esc:解除/終了"
            ));
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = false;
        self.clock = Instant::now();
        if let Some(r) = &mut self.renderer {
            if let Err(e) = r.resize(r.window.inner_size()) {
                self.fail(event_loop, e);
            }
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("liteweight-fps M2")
            .with_resizable(self.benchmark.is_none())
            .with_inner_size(LogicalSize::new(960., 540.));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.fail(event_loop, e.to_string());
                return;
            }
        };
        let (mesh, internal, vsync) = if let Some(run) = &self.benchmark {
            (
                liteweight_fps::scene(run.options.dummies),
                (run.options.width, run.options.height),
                run.options.vsync,
            )
        } else {
            (world::arena(), (1280, 720), true)
        };
        match pollster::block_on(WindowRenderer::new(window, &mesh, internal, vsync)) {
            Ok(r) => {
                println!("{}; {}", r.adapter_info, r.presentation_info);
                if let Some(run) = &mut self.benchmark {
                    run.backend = r.adapter_info.clone();
                    run.notes.push(r.presentation_info.clone());
                    run.notes
                        .push("M2 Player+collision, M0 scene/camera/measurement conditions".into());
                    let size = r.window.inner_size();
                    run.set_initial_display(size.width, size.height);
                }
                self.renderer = Some(r);
                self.smoke_started = Some(Instant::now());
                self.capture_requested = true;
                self.try_capture();
                self.update_title();
            }
            Err(e) => self.fail(event_loop, e),
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        if let Some(r) = &mut self.renderer {
            let _ = r.resize(PhysicalSize::new(0, 0));
        }
        if let Some(run) = &mut self.benchmark {
            run.measurement.valid = false;
        }
    }
    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if self.captured && self.benchmark.is_none() {
                self.player
                    .look(delta.0 as f32 * 0.002, delta.1 as f32 * 0.002);
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.finish(event_loop, false),
            WindowEvent::Resized(size) => {
                if self.smoke {
                    println!("actual display resize: {}×{}", size.width, size.height);
                }
                if let Some(run) = &mut self.benchmark {
                    run.measurement.resize(size.width, size.height);
                }
                if let Some(r) = &mut self.renderer {
                    if let Err(e) = r.resize(size) {
                        self.fail(event_loop, e);
                    }
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.clock = Instant::now();
                if occluded {
                    if let Some(run) = &mut self.benchmark {
                        run.measurement.valid = false;
                    }
                }
            }
            WindowEvent::Focused(focused) => {
                if let Some(run) = &mut self.benchmark {
                    run.focus(focused);
                }
                if self.benchmark.is_none() {
                    if focused {
                        // 起動直後やフォーカス復帰で捕捉を再試行する。
                        self.try_capture();
                    } else {
                        self.release_capture();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if self.benchmark.is_none() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) if pressed && !event.repeat => {
                            if self.captured {
                                self.capture_requested = false;
                                self.release_capture();
                            } else {
                                self.finish(event_loop, false);
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyA) => self.input.left = pressed,
                        PhysicalKey::Code(KeyCode::KeyD) => self.input.right = pressed,
                        PhysicalKey::Code(KeyCode::KeyW) => self.input.forward = pressed,
                        PhysicalKey::Code(KeyCode::KeyS) => self.input.back = pressed,
                        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => {
                            self.input.dash = pressed
                        }
                        PhysicalKey::Code(KeyCode::Space) if pressed && !event.repeat => {
                            self.input.jump = true
                        }
                        _ => {}
                    }
                }
                if pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(KeyCode::KeyQ)
                {
                    self.finish(event_loop, false);
                }
            }
            WindowEvent::RedrawRequested => {
                if self.suspended || self.occluded {
                    return;
                }
                let Some(r) = &mut self.renderer else {
                    return;
                };
                if !r.drawable() {
                    return;
                }
                let now = Instant::now();
                let dt = now.duration_since(self.clock).as_secs_f64();
                self.clock = now;
                let colliders = std::mem::take(&mut self.colliders);
                let updates = self.fixed.updates(dt);
                let input = self.input;
                self.input.jump = false;
                for _ in 0..updates {
                    self.player
                        .move_with_collision(&colliders, input, 1.0 / 60.0);
                }
                self.colliders = colliders;
                let (w, h) = r.internal_size();
                let matrix = if let Some(run) = &self.benchmark {
                    run.matrix(w, h, false)
                } else {
                    self.player.view_matrix(w as f32 / h as f32)
                };
                match r.render(matrix) {
                    Ok(true) => {
                        if let Some(run) = &mut self.benchmark {
                            if run.presented(now) {
                                self.finish(event_loop, true);
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        if let Some(run) = &mut self.benchmark {
                            run.measurement.valid = false;
                        }
                        if let Err(e) = r.resize(r.window.inner_size()) {
                            self.fail(event_loop, e);
                        }
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        if let Some(run) = &mut self.benchmark {
                            run.measurement.valid = false;
                        }
                        eprintln!("Surface timeout; retrying");
                    }
                    Err(e) => self.fail(event_loop, format!("描画失敗: {e}")),
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.smoke {
            if let Some(start) = self.smoke_started {
                if start.elapsed().as_secs_f64() >= (self.smoke_step + 1) as f64 {
                    let sizes = [(640., 480.), (800., 800.), (1200., 500.), (960., 540.)];
                    if self.smoke_step < sizes.len() {
                        if let Some(r) = &mut self.renderer {
                            let (w, h) = sizes[self.smoke_step];
                            let _ = r.window.request_inner_size(LogicalSize::new(w, h));
                            // APIレベルの0サイズ休止も確認。実際の最小化は手動確認する。
                            if let Err(e) = r
                                .resize(PhysicalSize::new(0, 0))
                                .and_then(|_| r.resize(r.window.inner_size()))
                            {
                                self.fail(event_loop, e);
                                return;
                            }
                            println!(
                                "smoke step {}: display request {w}×{h}",
                                self.smoke_step + 1
                            );
                        }
                        self.smoke_step += 1;
                        self.update_title();
                    } else {
                        println!("M2 smoke completed");
                        self.finish(event_loop, true);
                        return;
                    }
                }
            }
        }
        let drawable = self.renderer.as_ref().is_some_and(|r| r.drawable())
            && !self.suspended
            && !self.occluded;
        if drawable {
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(r) = &self.renderer {
                r.window.request_redraw();
            }
        } else if self.smoke {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(100),
            ));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
fn main() {
    let start = Instant::now();
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!("liteweight-fps M2: 一人称視点のアリーナ。起動直後にマウスを捕捉します。\nWASD:移動 Shift:ダッシュ Space:ジャンプ Esc:捕捉解除/解除中は終了 Q:終了\n--smoke: 自動リサイズ確認後に終了\n--benchmark [M0と同じ引数]: M0シーンで描画基盤の回帰測定（--mode inputは非対応）");
        return;
    }
    let smoke = args == ["--smoke"];
    let options = if args.first().is_some_and(|a| a == "--benchmark") {
        let mut options = Options::parse(&args[1..]).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        });
        if options.input || (!cfg!(target_os = "linux") && options.window_system != "auto") {
            eprintln!("M2 benchmark: input非対応、window-systemはLinuxのみ指定可能");
            std::process::exit(2);
        }
        if !args.iter().any(|a| a == "--output") {
            options.output = "results/m2-benchmark".into();
        }
        Some(options)
    } else if args.is_empty() || smoke {
        None
    } else {
        eprintln!("不正な引数。--helpを参照してください");
        std::process::exit(2);
    };
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    if let Some(options) = &options {
        match options.window_system.as_str() {
            "x11" => {
                use winit::platform::x11::EventLoopBuilderExtX11;
                builder.with_x11();
            }
            "wayland" => {
                use winit::platform::wayland::EventLoopBuilderExtWayland;
                builder.with_wayland();
            }
            _ => {}
        }
    }
    let event_loop = builder.build().expect("event loop");
    let mut app = App {
        renderer: None,
        benchmark: options.map(|o| Run::new(o, start, String::new())),
        smoke,
        smoke_step: 0,
        smoke_started: None,
        clock: Instant::now(),
        player: Player::new(),
        colliders: world::arena_colliders(),
        input: Input::default(),
        capture_requested: false,
        captured: false,
        fixed: FixedStep::new(60, 5),
        occluded: false,
        suspended: false,
        error: None,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    if app.error.is_some() {
        std::process::exit(1);
    }
}
