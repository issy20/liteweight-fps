use liteweight_fps::{
    engine::{
        audio::{Audio, Sound},
        window::WindowRenderer,
    },
    physics::Aabb,
    simulation::{
        combat::Combat,
        player::Input,
        time::FixedStep,
        weapon::{FireInput, Loadout, WeaponKind},
        Player,
    },
    world, Options, Run,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
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
    weapons: Loadout,
    fire_input: FireInput,
    combat: Combat,
    audio: Option<Audio>,
    title: String,
    capture_requested: bool,
    captured: bool,
    fixed: FixedStep,
    occluded: bool,
    suspended: bool,
    error: Option<String>,
}
impl App {
    fn switch_weapon(&mut self, kind: WeaponKind) {
        if self.weapons.switch(kind) {
            self.fire_input.discard_press();
        }
    }
    fn fire(&mut self, held: bool, pressed: bool) {
        let kind = self.weapons.active_kind();
        let view_direction = self.player.aim_direction(self.weapons.active().recoil);
        let forward = self.weapons.active().shot_direction(view_direction);
        let moving = self.player.velocity.length_squared() > 0.01;
        let spread = self.weapons.active().spread(moving);
        let damage = self.weapons.active().data.damage;
        if !self.weapons.fire(held, pressed) {
            return;
        }
        let direction = if kind == WeaponKind::Sniper {
            forward
        } else {
            // 発砲番号から再現可能な円盤内の拡散。反動を加える前の射線を使う。
            let n = self
                .combat
                .shots
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            let angle = (n & 65535) as f32 / 65536. * std::f32::consts::TAU;
            let radius = ((n >> 16) as f32 / 65536.).sqrt() * spread;
            let right = forward.cross(glam::Vec3::Y).normalize();
            let up = right.cross(forward);
            (forward + (right * angle.cos() + up * angle.sin()) * radius).normalize()
        };
        let shot = self
            .combat
            .shoot(self.player.position, direction, &self.colliders, damage);
        if let Some(audio) = &self.audio {
            audio.play(match kind {
                WeaponKind::AssaultRifle => Sound::Fire,
                WeaponKind::Pistol => Sound::Pistol,
                WeaponKind::Sniper => Sound::Sniper,
            });
            if shot.killed {
                audio.play(Sound::Kill);
            } else if shot.headshot {
                audio.play(Sound::Head);
            } else if shot.hit {
                audio.play(Sound::Hit);
            }
        }
    }
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
        self.input = Input::default();
        self.fire_input.clear();
        self.fixed = FixedStep::new(60, 5);
        self.clock = Instant::now();
    }
    fn update_title(&mut self) {
        if let Some(r) = &self.renderer {
            let (w, h) = r.internal_size();
            let title = format!(
                "liteweight-fps M5 | {w}×{h} | {} {}/{} | Bots {}/{} | Kills {} | Hits {}/{} | 1:Pistol 2:AR 3:Sniper LMB:Fire RMB:ADS R:Reload Esc:Pause",
                self.weapons.active().kind.label(), self.weapons.active().ammo, self.weapons.active().data.magazine,
                self.combat.targets.iter().filter(|bot| bot.alive).count(), self.combat.targets.len(), self.combat.kills,
                self.combat.hits, self.combat.shots
            );
            if title != self.title {
                r.window.set_title(&title);
                self.title = title;
            }
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
            .with_title("liteweight-fps M5 Free Training")
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
                if self.benchmark.is_none() {
                    match Audio::new() {
                        Ok(audio) => self.audio = Some(audio),
                        Err(e) => eprintln!("音声なしで続行: {e}"),
                    }
                }
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
        self.release_capture();
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
                self.player.look(
                    delta.0 as f32 * 0.002 * (1. - 0.35 * self.weapons.active().ads_progress),
                    delta.1 as f32 * 0.002 * (1. - 0.35 * self.weapons.active().ads_progress),
                );
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
                    self.release_capture();
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
            WindowEvent::MouseInput { state, button, .. } if self.benchmark.is_none() => {
                let pressed = state == ElementState::Pressed;
                if !self.captured {
                    if pressed && button == MouseButton::Left {
                        self.capture_requested = true;
                        self.try_capture();
                        self.clock = Instant::now();
                    }
                } else {
                    match button {
                        MouseButton::Left => self.fire_input.set_trigger(pressed),
                        MouseButton::Right => self.fire_input.ads = pressed,
                        _ => {}
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
                        _ if !self.captured => {}
                        PhysicalKey::Code(KeyCode::Digit1) if pressed && !event.repeat => {
                            self.switch_weapon(WeaponKind::Pistol);
                        }
                        PhysicalKey::Code(KeyCode::Digit2) if pressed && !event.repeat => {
                            self.switch_weapon(WeaponKind::AssaultRifle);
                        }
                        PhysicalKey::Code(KeyCode::Digit3) if pressed && !event.repeat => {
                            self.switch_weapon(WeaponKind::Sniper);
                        }
                        PhysicalKey::Code(KeyCode::KeyR) if pressed && !event.repeat => {
                            self.fire_input.reload = true
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
                if !self.renderer.as_ref().is_some_and(|r| r.drawable()) {
                    return;
                }
                let now = Instant::now();
                let dt = now.duration_since(self.clock).as_secs_f64();
                self.clock = now;
                if self.captured && self.benchmark.is_none() {
                    if std::mem::take(&mut self.fire_input.reload)
                        && !self.weapons.is_switching()
                        && self.weapons.active_mut().reload()
                    {
                        if let Some(audio) = &self.audio {
                            audio.play(Sound::Reload);
                        }
                    }
                    let updates = self.fixed.updates(dt);
                    for _ in 0..updates {
                        let input = self.input;
                        self.input.jump = false;
                        self.player
                            .move_with_collision(&self.colliders, input, 1. / 60.);
                        self.weapons.advance(1. / 60., self.fire_input.ads);
                        self.combat
                            .advance(1. / 60., self.player.position, &self.colliders);
                        let (held, pressed) = self.fire_input.take_state();
                        if held || pressed {
                            self.fire(held, pressed);
                        }
                    }
                    // 60Hz更新の間でも押下を受け付ける。冷却時間の更新は固定ステップ。
                    if updates == 0 {
                        let (held, pressed) = self.fire_input.take_state();
                        if held || pressed {
                            self.fire(held, pressed);
                        }
                    }
                }
                self.update_title();
                let r = self.renderer.as_mut().unwrap();
                let (w, h) = r.internal_size();
                let matrix = if let Some(run) = &self.benchmark {
                    run.matrix(w, h, false)
                } else {
                    let aspect = w as f32 / h as f32;
                    let (displayed_weapon, switch_offset) = self.weapons.presentation();
                    r.update_extras(
                        &liteweight_fps::ui::world_mesh(&self.combat),
                        &liteweight_fps::ui::hud_mesh(
                            displayed_weapon,
                            &self.combat,
                            aspect,
                            switch_offset,
                        ),
                    );
                    self.player.view_matrix_with(
                        aspect,
                        self.weapons.fov(),
                        self.weapons.active().recoil,
                    )
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
                        println!("M5 smoke completed");
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
        println!("liteweight-fps M5 Free Training: 一人称視点のBOT射撃場。起動直後にマウスを捕捉します。\nWASD:移動 Shift:ダッシュ Space:ジャンプ LMB:射撃 RMB:ADS R:リロード Esc:捕捉解除/解除中は終了 左クリック:再捕捉 Q:終了\n--smoke: 自動リサイズ確認後に終了\n--benchmark [M0と同じ引数]: M0シーンで描画基盤の回帰測定（--mode inputは非対応）\n1/2/3:拳銃/AR/スナイパー切替 LMB:射撃 RMB:ADS R:リロード");
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
        weapons: Loadout::default(),
        fire_input: FireInput::default(),
        combat: Combat::default(),
        audio: None,
        title: String::new(),
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
