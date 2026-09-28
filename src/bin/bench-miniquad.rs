use liteweight_fps::{scene, Options, Run};
use miniquad::*;
use std::time::Instant;

struct App {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    pass: RenderPass,
    blit_pipeline: Pipeline,
    blit_bindings: Bindings,
    count: i32,
    run: Run,
}
impl App {
    fn new(options: Options, start: Instant) -> Self {
        let mut ctx = window::new_rendering_backend();
        let info = ctx.info();
        assert_eq!(info.backend, Backend::OpenGl);
        let mesh = scene(options.dummies);
        let bindings = Bindings {
            vertex_buffers: vec![ctx.new_buffer(
                BufferType::VertexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&mesh.vertices),
            )],
            index_buffer: ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&mesh.indices),
            ),
            images: vec![],
        };
        let source = ShaderSource::Glsl {
            vertex: include_str!("../../assets/shaders/scene.vert"),
            fragment: include_str!("../../assets/shaders/scene.frag"),
        };
        let shader = ctx
            .new_shader(
                source,
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("vp", UniformType::Mat4)],
                    },
                },
            )
            .expect("scene shader");
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("color", VertexFormat::Float3),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::Less,
                depth_write: true,
                cull_face: CullFace::Nothing,
                ..Default::default()
            },
        );
        let color = ctx.new_render_texture(TextureParams {
            width: options.width,
            height: options.height,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let depth = ctx.new_render_texture(TextureParams {
            width: options.width,
            height: options.height,
            format: TextureFormat::Depth,
            ..Default::default()
        });
        let pass = ctx.new_render_pass(color, Some(depth));
        let blit_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: include_str!("../../assets/shaders/blit.vert"),
                    fragment: include_str!("../../assets/shaders/blit.frag"),
                },
                ShaderMeta {
                    images: vec!["scene".into()],
                    uniforms: UniformBlockLayout { uniforms: vec![] },
                },
            )
            .expect("blit shader");
        let blit_pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("position", VertexFormat::Float2)],
            blit_shader,
            PipelineParams::default(),
        );
        let blit_bindings = Bindings {
            vertex_buffers: vec![ctx.new_buffer(
                BufferType::VertexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&[[-1_f32, -1.], [3., -1.], [-1., 3.]]),
            )],
            index_buffer: ctx.new_buffer(
                BufferType::IndexBuffer,
                BufferUsage::Immutable,
                BufferSource::slice(&[0_u16, 1, 2]),
            ),
            images: vec![color],
        };
        let mut run = Run::new(
            options,
            start,
            format!("miniquad {:?} {}", info.backend, info.gl_version_string),
        );
        run.notes.push(
            "swap_interval is a request; actual presentation interval must be verified".into(),
        );
        run.notes
            .push("Miniquad cursor grab API has no success result; verify manually".into());
        run.notes.push("macOS Miniquad uses OpenGL: Metal 0.4.11 offscreen pipeline fails Metal API validation. macOS GL uses a ~60 Hz timer; swap interval request is not reliable.".into());
        let (w, h) = window::screen_size();
        run.set_initial_display(w as u32, h as u32);
        Self {
            ctx,
            pipeline,
            bindings,
            pass,
            blit_pipeline,
            blit_bindings,
            count: mesh.indices.len() as i32,
            run,
        }
    }
    fn capture(&mut self, captured: bool) {
        window::set_cursor_grab(captured);
        window::show_mouse(!captured);
        self.run.mouse.captured = captured;
    }
}
impl EventHandler for App {
    fn update(&mut self) {}
    fn draw(&mut self) {
        let (w, h) = window::screen_size();
        if w <= 0. || h <= 0. {
            return;
        }
        let start = Instant::now();
        let matrix = self
            .run
            .matrix(self.run.options.width, self.run.options.height, true)
            .to_cols_array();
        self.ctx.begin_pass(
            Some(self.pass),
            PassAction::clear_color(0.08, 0.10, 0.14, 1.),
        );
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.apply_uniforms(UniformsSource::table(&matrix));
        self.ctx.draw(0, self.count, 1);
        self.ctx.end_render_pass();
        self.ctx
            .begin_default_pass(PassAction::clear_color(0., 0., 0., 1.));
        self.ctx.apply_pipeline(&self.blit_pipeline);
        self.ctx.apply_bindings(&self.blit_bindings);
        self.ctx.draw(0, 3, 1);
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
        if self.run.presented(start) {
            self.capture(false);
            self.run.save(true);
            window::order_quit();
        }
    }
    fn resize_event(&mut self, w: f32, h: f32) {
        self.run.measurement.resize(w as u32, h as u32);
    }
    fn mouse_button_down_event(&mut self, button: MouseButton, _: f32, _: f32) {
        if self.run.options.input && button == MouseButton::Left {
            self.capture(true);
        }
    }
    fn raw_mouse_motion(&mut self, dx: f32, dy: f32) {
        self.run.mouse.motion(dx as f64, dy as f64);
    }
    fn key_down_event(&mut self, key: KeyCode, _: KeyMods, _: bool) {
        match key {
            KeyCode::Escape => self.capture(false),
            KeyCode::Q => {
                self.capture(false);
                self.run.save(false);
                window::order_quit();
            }
            _ => {}
        }
    }
    fn window_minimized_event(&mut self) {
        self.capture(false);
        self.run.focus(false);
    }
    fn window_restored_event(&mut self) {
        self.run.focus(true);
    }
    fn quit_requested_event(&mut self) {
        self.capture(false);
        self.run.save(false);
    }
}
fn main() {
    let start = Instant::now();
    let options = Options::from_env();
    let conf = conf::Conf {
        window_title: "liteweight-fps M0 — Miniquad".into(),
        window_width: 960,
        window_height: 540,
        high_dpi: true,
        sample_count: 1,
        window_resizable: false,
        platform: conf::Platform {
            linux_backend: match options.window_system.as_str() {
                "wayland" => conf::LinuxBackend::WaylandOnly,
                "x11" => conf::LinuxBackend::X11Only,
                _ => conf::LinuxBackend::X11WithWaylandFallback,
            },
            apple_gfx_api: conf::AppleGfxApi::OpenGl,
            swap_interval: Some(i32::from(options.vsync)),
            ..Default::default()
        },
        ..Default::default()
    };
    miniquad::start(conf, move || Box::new(App::new(options, start)));
}
