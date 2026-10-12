use liteweight_fps::persistence::{BindingAction, BindingKey, GameSettings};
use liteweight_fps::ui::menu::{PauseMenuAction, PauseSelection, TitleMenuAction, TitleSelection};
use liteweight_fps::{
    engine::{
        audio::{Audio, Sound},
        window::WindowRenderer,
    },
    physics::Aabb,
    simulation::{
        combat::Combat,
        game_mode::{SessionMode, TrainingSession},
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
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

fn binding_key_to_code(key: BindingKey) -> KeyCode {
    match key {
        BindingKey::A => KeyCode::KeyA,
        BindingKey::B => KeyCode::KeyB,
        BindingKey::C => KeyCode::KeyC,
        BindingKey::D => KeyCode::KeyD,
        BindingKey::E => KeyCode::KeyE,
        BindingKey::F => KeyCode::KeyF,
        BindingKey::G => KeyCode::KeyG,
        BindingKey::H => KeyCode::KeyH,
        BindingKey::I => KeyCode::KeyI,
        BindingKey::J => KeyCode::KeyJ,
        BindingKey::K => KeyCode::KeyK,
        BindingKey::L => KeyCode::KeyL,
        BindingKey::M => KeyCode::KeyM,
        BindingKey::N => KeyCode::KeyN,
        BindingKey::O => KeyCode::KeyO,
        BindingKey::P => KeyCode::KeyP,
        BindingKey::Q => KeyCode::KeyQ,
        BindingKey::R => KeyCode::KeyR,
        BindingKey::S => KeyCode::KeyS,
        BindingKey::T => KeyCode::KeyT,
        BindingKey::U => KeyCode::KeyU,
        BindingKey::V => KeyCode::KeyV,
        BindingKey::W => KeyCode::KeyW,
        BindingKey::X => KeyCode::KeyX,
        BindingKey::Y => KeyCode::KeyY,
        BindingKey::Z => KeyCode::KeyZ,
        BindingKey::Space => KeyCode::Space,
        BindingKey::LeftShift => KeyCode::ShiftLeft,
        BindingKey::RightShift => KeyCode::ShiftRight,
        BindingKey::Escape => KeyCode::Escape,
        BindingKey::Digit1 => KeyCode::Digit1,
        BindingKey::Digit2 => KeyCode::Digit2,
        BindingKey::Digit3 => KeyCode::Digit3,
    }
}

fn code_to_binding_key(code: KeyCode) -> Option<BindingKey> {
    Some(match code {
        KeyCode::KeyA => BindingKey::A,
        KeyCode::KeyB => BindingKey::B,
        KeyCode::KeyC => BindingKey::C,
        KeyCode::KeyD => BindingKey::D,
        KeyCode::KeyE => BindingKey::E,
        KeyCode::KeyF => BindingKey::F,
        KeyCode::KeyG => BindingKey::G,
        KeyCode::KeyH => BindingKey::H,
        KeyCode::KeyI => BindingKey::I,
        KeyCode::KeyJ => BindingKey::J,
        KeyCode::KeyK => BindingKey::K,
        KeyCode::KeyL => BindingKey::L,
        KeyCode::KeyM => BindingKey::M,
        KeyCode::KeyN => BindingKey::N,
        KeyCode::KeyO => BindingKey::O,
        KeyCode::KeyP => BindingKey::P,
        KeyCode::KeyQ => BindingKey::Q,
        KeyCode::KeyR => BindingKey::R,
        KeyCode::KeyS => BindingKey::S,
        KeyCode::KeyT => BindingKey::T,
        KeyCode::KeyU => BindingKey::U,
        KeyCode::KeyV => BindingKey::V,
        KeyCode::KeyW => BindingKey::W,
        KeyCode::KeyX => BindingKey::X,
        KeyCode::KeyY => BindingKey::Y,
        KeyCode::KeyZ => BindingKey::Z,
        KeyCode::Space => BindingKey::Space,
        KeyCode::ShiftLeft => BindingKey::LeftShift,
        KeyCode::ShiftRight => BindingKey::RightShift,
        KeyCode::Escape => BindingKey::Escape,
        KeyCode::Digit1 => BindingKey::Digit1,
        KeyCode::Digit2 => BindingKey::Digit2,
        KeyCode::Digit3 => BindingKey::Digit3,
        _ => return None,
    })
}

struct App {
    renderer: Option<WindowRenderer>,
    world_mesh: liteweight_fps::Scene,
    hud_mesh: liteweight_fps::Scene,
    text_mesh: liteweight_fps::engine::text::TextMesh,
    benchmark: Option<Run>,
    smoke: bool,
    smoke_step: usize,
    smoke_started: Option<Instant>,
    smoke_deadline: Instant,
    clock: Instant,
    player: Player,
    colliders: Vec<Aabb>,
    input: Input,
    weapons: Loadout,
    fire_input: FireInput,
    combat: Combat,
    settings: GameSettings,
    session: TrainingSession,
    title_screen: bool,
    settings_menu: bool,
    settings_from_title: bool,
    results_screen: bool,
    title_selection: TitleSelection,
    pause_selection: PauseSelection,
    binding_selection: usize,
    waiting_for_binding: bool,
    audio: Option<Audio>,
    title: String,
    capture_requested: bool,
    captured: bool,
    window_focused: bool,
    fixed: FixedStep,
    next_frame_deadline: Option<Instant>,
    occluded: bool,
    suspended: bool,
    error: Option<String>,
    notice: Option<(String, Instant)>,
}
impl App {
    fn switch_weapon(&mut self, kind: WeaponKind) {
        if self.weapons.switch(kind) {
            self.fire_input.discard_press();
        }
    }
    fn fire(&mut self, held: bool, pressed: bool) {
        let kind = self.weapons.active_kind();
        let view_direction = self.player.aim_direction();
        let damage = self.weapons.active().data.damage;
        let recoil_kick = self.weapons.active().data.recoil_kick;
        if !self
            .weapons
            .fire_with_infinite_ammo(held, pressed, self.infinite_ammo_active())
        {
            return;
        }
        let shot_direction = self.weapons.active().shot_direction(view_direction);
        let (aspect, weapon_y_offset) = self
            .renderer
            .as_ref()
            .map(|renderer| {
                let (width, height) = renderer.internal_size();
                let (_, offset) = self.weapons.presentation();
                (
                    width as f32 / height.max(1) as f32,
                    offset + self.player.dash_weapon_offset(),
                )
            })
            .unwrap_or((16.0 / 9.0, 0.0));
        let fov = self.weapons.fov_with_base(self.settings.fov);
        let muzzle = self.weapons.active().muzzle_position(
            self.player.position,
            view_direction,
            aspect,
            fov,
            weapon_y_offset,
        );
        let shot = self.combat.shoot_from_muzzle_with_aim(
            self.player.position,
            muzzle,
            view_direction,
            shot_direction,
            &self.colliders,
            damage,
        );
        if self.session.mode().recoil_enabled() {
            self.player.apply_recoil(recoil_kick);
        } else {
            self.weapons.active_mut().visual_recoil = 0.0;
        }
        self.session
            .record_shot(kind, shot.hit, shot.headshot, shot.killed);
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
    fn set_notice(&mut self, message: &str, detail: &str) {
        eprintln!("{message}: {detail}");
        self.notice = Some((message.to_owned(), Instant::now()));
    }
    fn save_settings(&mut self) {
        match self.settings.save() {
            Ok(()) => self.notice = None,
            Err(error) => self.set_notice("SETTINGS NOT SAVED - CHECK STORAGE", &error),
        }
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
    fn resume_from_pause(&mut self) {
        self.save_settings();
        self.settings_menu = false;
        if self.settings_from_title {
            self.settings_from_title = false;
            self.title_screen = true;
            self.capture_requested = false;
            self.release_capture();
            return;
        }
        self.capture_requested = true;
        self.try_capture();
    }
    fn change_mode(&mut self) {
        self.save_settings();
        self.settings_menu = false;
        self.settings_from_title = false;
        self.title_screen = true;
        self.results_screen = false;
        self.title_selection = TitleSelection::default();
        self.capture_requested = false;
        self.release_capture();
    }
    fn begin_session(&mut self, mode: SessionMode) {
        self.player = Player::new();
        self.weapons = Loadout::default();
        self.fire_input = FireInput::default();
        self.combat = Combat::default();
        if mode == SessionMode::TrackingAim {
            self.combat.configure_tracking_bots(
                self.settings.bot_count,
                self.settings.bot_hp,
                self.settings.bot_speed,
                self.settings.bot_respawn_seconds,
            );
            self.colliders = world::tracking_colliders();
            if let Some(renderer) = &mut self.renderer {
                if let Err(error) = renderer.update_scene(&world::tracking_range()) {
                    self.error = Some(error);
                }
            }
        } else {
            let arena_scale = world::arena_scale_for_bot_count(self.settings.bot_count);
            self.combat.configure_bots_with_arena_scale(
                self.settings.bot_count,
                self.settings.bot_behavior,
                self.settings.bot_hp,
                self.settings.bot_speed,
                self.settings.bot_respawn_seconds,
                arena_scale,
            );
            self.colliders = world::arena_colliders_scaled(arena_scale);
            if let Some(renderer) = &mut self.renderer {
                if let Err(error) = renderer.update_scene(&world::arena_scaled(arena_scale)) {
                    self.error = Some(error);
                }
            }
        }
        self.session = TrainingSession::new(mode);
        self.title_screen = false;
        self.settings_menu = false;
        self.settings_from_title = false;
        self.results_screen = false;
        self.capture_requested = true;
        self.try_capture();
        self.clock = Instant::now();
    }
    fn infinite_ammo_active(&self) -> bool {
        self.session
            .mode()
            .infinite_ammo_enabled(self.settings.infinite_ammo)
    }
    fn apply_runtime_setting(&mut self, setting: usize) {
        match setting {
            3 | 4 | 6 | 7 | 8 => {
                if self.session.mode() == SessionMode::TrackingAim {
                    self.combat.configure_tracking_bots(
                        self.settings.bot_count,
                        self.settings.bot_hp,
                        self.settings.bot_speed,
                        self.settings.bot_respawn_seconds,
                    );
                    self.colliders = world::tracking_colliders();
                    if let Some(renderer) = &mut self.renderer {
                        if let Err(error) = renderer.update_scene(&world::tracking_range()) {
                            self.error = Some(error);
                        }
                    }
                } else {
                    let arena_scale = world::arena_scale_for_bot_count(self.settings.bot_count);
                    self.combat.configure_bots_with_arena_scale(
                        self.settings.bot_count,
                        self.settings.bot_behavior,
                        self.settings.bot_hp,
                        self.settings.bot_speed,
                        self.settings.bot_respawn_seconds,
                        arena_scale,
                    );
                    self.colliders = world::arena_colliders_scaled(arena_scale);
                    if let Some(renderer) = &mut self.renderer {
                        if let Err(error) = renderer.update_scene(&world::arena_scaled(arena_scale))
                        {
                            self.error = Some(error);
                        }
                    }
                }
            }
            5 => {
                if let Some(audio) = &mut self.audio {
                    audio.set_volume(self.settings.volume);
                }
            }
            9 => {
                if !self.settings.fullscreen {
                    if let Some(renderer) = &self.renderer {
                        let (width, height) = self.settings.resolution_size();
                        let _ = renderer
                            .window
                            .request_inner_size(LogicalSize::new(width as f64, height as f64));
                    }
                }
            }
            10 => {
                if let Some(renderer) = &mut self.renderer {
                    let size = renderer.window.inner_size();
                    if let Err(error) =
                        renderer.resize_follow_display_aspect(size, self.settings.render_scale())
                    {
                        self.error = Some(error);
                    }
                }
            }
            11 => self.next_frame_deadline = None,
            _ => {}
        }
    }
    fn apply_fullscreen(&mut self) {
        if let Some(renderer) = &self.renderer {
            let fullscreen = self
                .settings
                .fullscreen
                .then(|| Fullscreen::Borderless(renderer.window.current_monitor()));
            renderer.window.set_fullscreen(fullscreen);
            if !self.settings.fullscreen {
                let (width, height) = self.settings.resolution_size();
                let _ = renderer
                    .window
                    .request_inner_size(LogicalSize::new(width as f64, height as f64));
            }
        }
    }
    fn apply_vsync(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_vsync(self.settings.vsync);
        }
    }
    fn toggle_setting(&mut self, setting: usize) {
        self.settings.toggle_setting(setting);
        match setting {
            1 => self.apply_fullscreen(),
            3 => self.apply_vsync(),
            _ => {}
        }
        self.save_settings();
    }
    fn update_title(&mut self) {
        if let Some(r) = &self.renderer {
            let (w, h) = r.internal_size();
            let title = format!(
                "liteweight-fps | {w}×{h} | {} {}/{} | Bots {}/{} | Kills {} | Hits {}/{} | 1:Pistol 2:AR 3:Sniper LMB:Fire RMB:ADS R:Reload Esc:Settings",
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
            let size = r.window.inner_size();
            let result = if self.benchmark.is_none() {
                r.resize_follow_display_aspect(size, self.settings.render_scale())
            } else {
                r.resize(size)
            };
            if let Err(e) = result {
                self.fail(event_loop, e);
            }
            return;
        }
        let (window_width, window_height) = if self.benchmark.is_none() {
            self.settings.resolution_size()
        } else {
            (960, 540)
        };
        let mut attributes = Window::default_attributes()
            .with_title("liteweight-fps")
            .with_resizable(self.benchmark.is_none())
            .with_inner_size(LogicalSize::new(
                f64::from(window_width),
                f64::from(window_height),
            ));
        if self.benchmark.is_none() && self.settings.fullscreen {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
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
            (
                world::arena_scaled(world::arena_scale_for_bot_count(self.settings.bot_count)),
                (1280, 720),
                self.settings.vsync,
            )
        };
        match pollster::block_on(WindowRenderer::new(window, &mesh, internal, vsync)) {
            Ok(mut r) => {
                println!("{}; {}", r.adapter_info, r.presentation_info);
                if let Some(run) = &mut self.benchmark {
                    run.backend = r.adapter_info.clone();
                    run.notes.push(r.presentation_info.clone());
                    run.notes
                        .push("M2 Player+collision, M0 scene/camera/measurement conditions".into());
                    let size = r.window.inner_size();
                    run.set_initial_display(size.width, size.height);
                }
                if self.benchmark.is_none() {
                    let size = r.window.inner_size();
                    if let Err(error) =
                        r.resize_follow_display_aspect(size, self.settings.render_scale())
                    {
                        self.fail(event_loop, error);
                        return;
                    }
                    r.prepare_text();
                }
                self.renderer = Some(r);
                if self.benchmark.is_none() {
                    match Audio::new() {
                        Ok(mut audio) => {
                            audio.set_volume(self.settings.volume);
                            self.audio = Some(audio);
                        }
                        Err(e) => self.set_notice("AUDIO UNAVAILABLE - GAME WILL BE SILENT", &e),
                    }
                }
                self.smoke_started = Some(Instant::now());
                if self.benchmark.is_some() {
                    self.capture_requested = true;
                    self.try_capture();
                }
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
                    delta.0 as f32
                        * 0.002
                        * self
                            .settings
                            .look_multiplier(self.weapons.active().ads_progress),
                    delta.1 as f32
                        * 0.002
                        * self
                            .settings
                            .look_multiplier(self.weapons.active().ads_progress)
                        * self.settings.look_y_sign(),
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
                    let result = if self.benchmark.is_none() {
                        r.resize_follow_display_aspect(size, self.settings.render_scale())
                    } else {
                        r.resize(size)
                    };
                    if let Err(e) = result {
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
                self.window_focused = focused;
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
                    if !self.settings_menu && pressed && button == MouseButton::Left {
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
                self.window_focused = true;
                let pressed = event.state == ElementState::Pressed;
                let gameplay_active =
                    !self.title_screen && !self.settings_menu && !self.results_screen;
                if self.benchmark.is_none() && self.waiting_for_binding {
                    if pressed && !event.repeat {
                        if let PhysicalKey::Code(code) = event.physical_key {
                            if code == KeyCode::Escape {
                                self.waiting_for_binding = false;
                                self.set_notice("KEY BINDING CANCELLED", "");
                            } else if let Some(key) = code_to_binding_key(code) {
                                let action = BindingAction::ALL
                                    [self.binding_selection % BindingAction::ALL.len()];
                                if self.settings.key_bindings.set(action, key) {
                                    self.waiting_for_binding = false;
                                    self.save_settings();
                                    self.set_notice("KEY BINDING UPDATED", "");
                                } else {
                                    self.set_notice("KEY ALREADY IN USE", "Choose another key");
                                }
                            }
                        }
                    }
                } else if self.benchmark.is_none() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ArrowUp)
                            if pressed && !event.repeat && self.title_screen =>
                        {
                            self.title_selection.move_up();
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown)
                            if pressed && !event.repeat && self.title_screen =>
                        {
                            self.title_selection.move_down();
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                            if pressed && !event.repeat && self.title_screen =>
                        {
                            match self.title_selection.activate() {
                                TitleMenuAction::FreeTraining => {
                                    self.begin_session(SessionMode::FreeTraining)
                                }
                                TitleMenuAction::FiveMinuteChallenge => {
                                    self.begin_session(SessionMode::FiveMinuteChallenge)
                                }
                                TitleMenuAction::TrackingAim => {
                                    self.begin_session(SessionMode::TrackingAim)
                                }
                                TitleMenuAction::Settings => {
                                    self.title_screen = false;
                                    self.settings_menu = true;
                                    self.settings_from_title = true;
                                    self.pause_selection = PauseSelection::default();
                                    self.capture_requested = false;
                                }
                                TitleMenuAction::Quit => self.finish(event_loop, false),
                            }
                        }
                        PhysicalKey::Code(KeyCode::Escape)
                            if pressed && !event.repeat && self.results_screen =>
                        {
                            self.results_screen = false;
                            self.title_screen = true;
                            self.title_selection = TitleSelection::default();
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                            if pressed && !event.repeat && self.results_screen =>
                        {
                            self.begin_session(self.session.mode());
                        }
                        PhysicalKey::Code(code)
                            if pressed
                                && !event.repeat
                                && (code == KeyCode::Escape
                                    || code
                                        == binding_key_to_code(
                                            self.settings.key_bindings.get(BindingAction::Pause),
                                        )) =>
                        {
                            if self.settings_menu {
                                self.resume_from_pause();
                            } else {
                                self.capture_requested = false;
                                self.release_capture();
                                if !self.title_screen && !self.results_screen {
                                    self.pause_selection = PauseSelection::default();
                                    self.settings_from_title = false;
                                    self.settings_menu = true;
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            match self.pause_selection.activate() {
                                Some(PauseMenuAction::Resume) => self.resume_from_pause(),
                                Some(PauseMenuAction::ChangeMode) => self.change_mode(),
                                Some(PauseMenuAction::Exit) => self.finish(event_loop, false),
                                None => {
                                    if self.pause_selection.selected_index() == 5 {
                                        self.waiting_for_binding = true;
                                        self.set_notice("PRESS A KEY TO REBIND", "ESC cancels");
                                    } else if let Some(index) =
                                        self.pause_selection.toggle_setting_index()
                                    {
                                        self.toggle_setting(index);
                                    }
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            self.pause_selection.move_up();
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            self.pause_selection.move_down();
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            if self.pause_selection.selected_index() == 5 {
                                self.binding_selection =
                                    (self.binding_selection + BindingAction::ALL.len() - 1)
                                        % BindingAction::ALL.len();
                            } else if let Some(index) = self.pause_selection.setting_index() {
                                self.settings.adjust_setting(index, -1.0);
                                self.apply_runtime_setting(index);
                                self.save_settings();
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            if self.pause_selection.selected_index() == 5 {
                                self.binding_selection =
                                    (self.binding_selection + 1) % BindingAction::ALL.len();
                            } else if let Some(index) = self.pause_selection.setting_index() {
                                self.settings.adjust_setting(index, 1.0);
                                self.apply_runtime_setting(index);
                                self.save_settings();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Home)
                            if pressed && !event.repeat && self.settings_menu =>
                        {
                            self.settings = GameSettings::default();
                            self.binding_selection = 0;
                            self.waiting_for_binding = false;
                            for index in [3, 4, 5, 6, 7, 8] {
                                self.apply_runtime_setting(index);
                            }
                            self.apply_fullscreen();
                            self.apply_vsync();
                            self.apply_runtime_setting(9);
                            self.apply_runtime_setting(10);
                            self.next_frame_deadline = None;
                            self.save_settings();
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Left),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.left = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Right),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.right = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Forward),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.forward = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Backward),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.back = pressed
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Dash),
                                )
                                && Input::should_update_movement_key(pressed, gameplay_active) =>
                        {
                            self.input.dash = pressed
                        }
                        PhysicalKey::Code(code)
                            if pressed
                                && !event.repeat
                                && !self.title_screen
                                && !self.settings_menu
                                && !self.results_screen
                                && code
                                    == binding_key_to_code(
                                        self.settings.key_bindings.get(BindingAction::Jump),
                                    ) =>
                        {
                            self.input.jump = true
                        }
                        _ if !self.captured => {}
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Pistol),
                                )
                                && pressed
                                && !event.repeat =>
                        {
                            self.switch_weapon(WeaponKind::Pistol);
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::AssaultRifle),
                                )
                                && pressed
                                && !event.repeat =>
                        {
                            self.switch_weapon(WeaponKind::AssaultRifle);
                        }
                        PhysicalKey::Code(code)
                            if code
                                == binding_key_to_code(
                                    self.settings.key_bindings.get(BindingAction::Sniper),
                                )
                                && pressed
                                && !event.repeat =>
                        {
                            self.switch_weapon(WeaponKind::Sniper);
                        }
                        PhysicalKey::Code(code)
                            if pressed
                                && !event.repeat
                                && !self.infinite_ammo_active()
                                && code
                                    == binding_key_to_code(
                                        self.settings.key_bindings.get(BindingAction::Reload),
                                    ) =>
                        {
                            self.fire_input.reload = true;
                        }
                        _ => {}
                    }
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
                let gameplay_active =
                    !self.title_screen && !self.settings_menu && !self.results_screen;
                if Input::should_simulate_gameplay(gameplay_active, self.window_focused)
                    && self.benchmark.is_none()
                {
                    if std::mem::take(&mut self.fire_input.reload)
                        && !self.infinite_ammo_active()
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
                        if self.session.advance(1.0 / 60.0) {
                            if let Err(error) =
                                liteweight_fps::persistence::save_result(self.session.stats())
                            {
                                self.set_notice("RESULTS NOT SAVED - CHECK STORAGE", &error);
                            }
                            self.results_screen = true;
                            self.capture_requested = false;
                            self.release_capture();
                            break;
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
                let infinite_ammo_active = self.infinite_ammo_active();
                let notice = self
                    .notice
                    .as_ref()
                    .filter(|(_, started)| started.elapsed() < Duration::from_secs(8))
                    .map(|(message, _)| message.clone());
                if self
                    .notice
                    .as_ref()
                    .is_some_and(|(_, started)| started.elapsed() >= Duration::from_secs(8))
                {
                    self.notice = None;
                }
                let r = self.renderer.as_mut().unwrap();
                let (w, h) = r.internal_size();
                let matrix = if let Some(run) = &self.benchmark {
                    run.matrix(w, h, false)
                } else {
                    let aspect = w as f32 / h as f32;
                    let (displayed_weapon, switch_offset) = self.weapons.presentation();
                    liteweight_fps::ui::hud_mesh_with_menu_and_ammo_into(
                        &mut self.hud_mesh,
                        displayed_weapon,
                        &self.combat,
                        aspect,
                        switch_offset,
                        self.settings_menu
                            .then_some((&self.settings, self.pause_selection.selected_index())),
                        infinite_ammo_active,
                    );
                    if self.title_screen {
                        liteweight_fps::ui::title_overlay(
                            &mut self.hud_mesh,
                            self.title_selection.selected_index(),
                            aspect,
                        );
                    } else if self.results_screen {
                        liteweight_fps::ui::results_overlay(&mut self.hud_mesh, aspect);
                    }
                    liteweight_fps::ui::world_mesh_into(&mut self.world_mesh, &self.combat);
                    r.update_extras(&self.world_mesh, &self.hud_mesh);
                    if self.title_screen {
                        liteweight_fps::ui::title_text_into(
                            &mut self.text_mesh,
                            self.title_selection.selected_index(),
                            [w, h],
                        );
                    } else if self.results_screen {
                        liteweight_fps::ui::results_text_into(
                            &mut self.text_mesh,
                            self.session.stats(),
                            [w, h],
                        );
                    } else if self.settings_menu {
                        liteweight_fps::ui::settings_text_with_bindings_into(
                            &mut self.text_mesh,
                            &self.settings,
                            self.pause_selection.selected_index(),
                            [w, h],
                            self.settings_from_title,
                            self.binding_selection,
                            self.waiting_for_binding,
                        )
                    } else {
                        liteweight_fps::ui::game_hud_text_into(
                            &mut self.text_mesh,
                            liteweight_fps::ui::GameHudInfo {
                                mode: self.session.mode(),
                                remaining: self.session.remaining_seconds(),
                                weapon: self.weapons.active_kind(),
                                ammo: self.weapons.active().ammo,
                                magazine: self.weapons.active().data.magazine,
                                kills: self.combat.kills,
                                infinite_ammo: infinite_ammo_active,
                                key_bindings: self.settings.key_bindings,
                            },
                            [w, h],
                        );
                    }
                    if let Some(notice) = &notice {
                        let notice_y = if self.title_screen {
                            0.34
                        } else if self.results_screen {
                            0.55
                        } else if self.settings_menu {
                            0.575
                        } else {
                            0.76
                        };
                        liteweight_fps::ui::append_notice_text(
                            &mut self.text_mesh,
                            notice,
                            [w, h],
                            notice_y,
                        );
                    }
                    r.update_text(&self.text_mesh);
                    self.player
                        .view_matrix_with(aspect, self.weapons.fov_with_base(self.settings.fov))
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
        if self.smoke && Instant::now() >= self.smoke_deadline {
            self.fail(
                event_loop,
                "smoke test timed out after 30 seconds".to_owned(),
            );
            return;
        }
        if self.smoke {
            if let Some(start) = self.smoke_started {
                if start.elapsed().as_secs_f64() >= (self.smoke_step + 1) as f64 {
                    let sizes = [(640., 480.), (800., 800.), (1200., 500.), (960., 540.)];
                    if self.smoke_step < sizes.len() {
                        if let Some(r) = &mut self.renderer {
                            let (w, h) = sizes[self.smoke_step];
                            let _ = r.window.request_inner_size(LogicalSize::new(w, h));
                            // APIレベルの0サイズ休止も確認。実際の最小化は手動確認する。
                            let result = r.resize(PhysicalSize::new(0, 0)).and_then(|_| {
                                let size = r.window.inner_size();
                                if self.benchmark.is_none() {
                                    r.resize_follow_display_aspect(
                                        size,
                                        self.settings.render_scale(),
                                    )
                                } else {
                                    r.resize(size)
                                }
                            });
                            if let Err(e) = result {
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
                        println!("startup smoke completed");
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
            if let Some(fps) = self.settings.fps_limit() {
                let now = Instant::now();
                let deadline = match self.next_frame_deadline {
                    Some(deadline) if deadline > now => deadline,
                    _ => {
                        let deadline = now + Duration::from_secs_f64(1.0 / f64::from(fps));
                        self.next_frame_deadline = Some(deadline);
                        if let Some(renderer) = &self.renderer {
                            renderer.window.request_redraw();
                        }
                        deadline
                    }
                };
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            } else {
                self.next_frame_deadline = None;
                event_loop.set_control_flow(ControlFlow::Poll);
                if let Some(renderer) = &self.renderer {
                    renderer.window.request_redraw();
                }
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
        println!("liteweight-fps: タイトル画面でFree Trainingまたは5-Minute Challengeを選択。\nWASD:移動 Shift:ダッシュ Space:ジャンプ LMB:射撃 RMB:ADS R:リロード Esc:ポーズ（メニューから再開・終了）\n--smoke: 自動リサイズ確認後に終了\n--benchmark [M0と同じ引数]: M0シーンで描画基盤の回帰測定（--mode inputは非対応）\n1/2/3:拳銃/AR/スナイパー切替 LMB:射撃 RMB:ADS R:リロード");
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
    let title_screen = !smoke && options.is_none();
    let settings = GameSettings::load();
    let arena_scale = world::arena_scale_for_bot_count(settings.bot_count);
    let mut app = App {
        renderer: None,
        world_mesh: liteweight_fps::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        },
        hud_mesh: liteweight_fps::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        },
        text_mesh: liteweight_fps::engine::text::TextMesh::default(),
        benchmark: options.map(|o| Run::new(o, start, String::new())),
        smoke,
        smoke_step: 0,
        smoke_started: None,
        smoke_deadline: start + Duration::from_secs(30),
        clock: Instant::now(),
        player: Player::new(),
        colliders: world::arena_colliders_scaled(arena_scale),
        input: Input::default(),
        weapons: Loadout::default(),
        fire_input: FireInput::default(),
        combat: Combat::default(),
        settings,
        session: TrainingSession::new(SessionMode::FreeTraining),
        title_screen,
        settings_menu: false,
        settings_from_title: false,
        results_screen: false,
        title_selection: TitleSelection::default(),
        pause_selection: PauseSelection::default(),
        binding_selection: 0,
        waiting_for_binding: false,
        audio: None,
        title: String::new(),
        capture_requested: false,
        captured: false,
        window_focused: false,
        fixed: FixedStep::new(60, 5),
        next_frame_deadline: None,
        occluded: false,
        suspended: false,
        error: None,
        notice: None,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    if app.error.is_some() {
        std::process::exit(1);
    }
}
