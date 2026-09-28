use liteweight_fps::stats;

#[test]
fn frame_statistics_use_slowest_one_percent_and_nearest_rank() {
    assert!(stats(&[]).is_none());
    let mut samples = vec![10.0; 99];
    samples.push(50.0);
    let s = stats(&samples).unwrap();
    assert!((s.average_fps - 1000.0 / 10.4).abs() < 1e-9);
    assert_eq!(s.p99_ms, 10.0);
    assert_eq!(s.one_percent_low_fps, 20.0);
}

#[test]
fn scene_is_shared_deterministic_and_indices_valid() {
    let a = liteweight_fps::scene(5);
    let b = liteweight_fps::scene(5);
    assert_eq!(a.vertices, b.vertices);
    assert_eq!(a.indices, b.indices);
    assert!(a.indices.iter().all(|i| (*i as usize) < a.vertices.len()));
    assert_eq!(
        liteweight_fps::scene(10).indices.len() - a.indices.len(),
        5 * 72
    );
    assert_eq!(
        liteweight_fps::scene(20).indices.len() - a.indices.len(),
        15 * 72
    );
}

#[test]
fn camera_and_depth_conventions_match() {
    use glam::Vec4;
    use liteweight_fps::camera;
    let gl = camera(3.0, 16.0 / 9.0, true, None);
    let gpu = camera(3.0, 16.0 / 9.0, false, None);
    assert_eq!(gl, camera(3.0, 16.0 / 9.0, true, None));
    let a = gl * Vec4::new(0., 1., 0., 1.);
    let b = gpu * Vec4::new(0., 1., 0., 1.);
    assert!((a.x / a.w - b.x / b.w).abs() < 1e-5);
    assert!((a.z / a.w * 0.5 + 0.5 - b.z / b.w).abs() < 1e-5);
}

#[test]
fn relative_mouse_is_additive_and_ignored_when_released() {
    let mut input = liteweight_fps::Mouse::default();
    input.motion(100., 0.);
    assert_eq!(input.yaw, 0.);
    input.captured = true;
    input.motion(10., -5.);
    input.motion(10., -5.);
    assert!((input.yaw - 0.04).abs() < 1e-6);
    assert!((input.pitch - 0.02).abs() < 1e-6);
    assert_eq!(input.events, 2);
}

#[test]
fn options_validate_finite_bounded_comparison_conditions() {
    use liteweight_fps::Options;
    let default = Options::parse(&[]).unwrap();
    assert_eq!(
        (default.width, default.height, default.dummies),
        (1280, 720, 5)
    );
    assert_eq!(default.seconds, 30.);
    for args in [
        vec!["--seconds", "NaN"],
        vec!["--width", "0"],
        vec!["--dummies", "21"],
        vec!["--vsync", "maybe"],
        vec!["--wat"],
        vec!["--seconds"],
    ] {
        assert!(Options::parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).is_err());
    }
    let args = ["--seconds", "10", "--mode", "input", "--vsync", "off"];
    let o = Options::parse(&args.map(String::from)).unwrap();
    assert!(o.input);
    assert!(!o.vsync);
    assert_eq!(o.seconds, 10.);
}

#[test]
fn measurement_excludes_warmup_and_marks_resize_invalid() {
    let mut m = liteweight_fps::Measurement::new(5., 10., (1280, 720));
    m.record(4.99, 5.01, 1.0);
    m.record(5.01, 5.03, 2.0);
    m.record(14.99, 15.01, 3.0);
    assert_eq!(m.frames.len(), 1);
    assert!((m.frames[0].interval_ms - 20.).abs() < 1e-6);
    assert_eq!(m.frames[0].cpu_submit_ms, 2.);
    assert!(m.valid);
    m.resize(1280, 720);
    assert!(m.valid);
    m.resize(1920, 1080);
    assert!(!m.valid);
}

#[test]
fn invalid_frame_samples_do_not_produce_misleading_statistics() {
    for samples in [vec![0.], vec![-1.], vec![f64::NAN], vec![f64::INFINITY]] {
        assert!(stats(&samples).is_none());
    }
    let s = stats(&[16.]).unwrap();
    assert_eq!(s.average_fps, 62.5);
    assert_eq!(s.one_percent_low_fps, 62.5);
    assert_eq!(s.p99_ms, 16.);
}

#[test]
fn linux_window_system_is_an_explicit_comparison_condition() {
    use liteweight_fps::Options;
    let args = ["--window-system".into(), "wayland".into()];
    assert!(Options::parse(&args).is_ok());
    let args = ["--window-system".into(), "invalid".into()];
    assert!(Options::parse(&args).is_err());
}

#[test]
fn initial_focus_transition_does_not_invalidate_later_measurement() {
    let mut m = liteweight_fps::Measurement::new(5., 10., (1280, 720));
    m.focus(false, 0.);
    m.focus(true, 1.);
    m.record(5., 5.02, 1.);
    assert!(m.valid);
    m.focus(false, 6.);
    m.focus(true, 6.01);
    assert!(!m.valid);
}
