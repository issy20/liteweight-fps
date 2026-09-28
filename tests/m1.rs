use liteweight_fps::engine::layout::FrameLayout;

#[test]
fn resized_window_preserves_internal_resolution_and_aspect() {
    let mut layout = FrameLayout::new(1280, 720).unwrap();
    layout.resize(1000, 1000);
    assert_eq!(layout.internal_size(), (1280, 720));
    assert_eq!(layout.viewport(), Some((0, 219, 1000, 562)));
    layout.resize(2000, 800);
    assert_eq!(layout.viewport(), Some((289, 0, 1422, 800)));
}

#[test]
fn zero_display_suspends_and_restore_resumes_without_changing_projection() {
    let mut layout = FrameLayout::new(1280, 720).unwrap();
    for size in [(0, 720), (1280, 0), (0, 0)] {
        layout.resize(size.0, size.1);
        assert_eq!(layout.viewport(), None);
    }
    layout.resize(1920, 1080);
    assert_eq!(layout.viewport(), Some((0, 0, 1920, 1080)));
    assert_eq!(layout.internal_size(), (1280, 720));
    layout.resize(1, 1);
    assert_eq!(layout.viewport(), Some((0, 0, 1, 1)));
}

#[test]
fn internal_resolution_updates_atomically_and_rejects_invalid_sizes() {
    for size in [(0, 720), (1280, 0), (4097, 720)] {
        assert!(FrameLayout::new(size.0, size.1).is_err());
    }
    let mut layout = FrameLayout::new(1280, 720).unwrap();
    layout.resize(1000, 1000);
    layout.set_internal(800, 600).unwrap();
    assert_eq!(layout.viewport(), Some((0, 125, 1000, 750)));
    assert!(layout.set_internal(0, 600).is_err());
    assert_eq!(layout.internal_size(), (800, 600));
}

#[test]
fn arena_has_floor_four_walls_and_six_covers_without_targets() {
    let mesh = liteweight_fps::world::arena();
    assert_eq!(mesh.vertices.len(), 11 * 24);
    assert_eq!(mesh.indices.len(), 11 * 36);
    assert!(mesh
        .indices
        .iter()
        .all(|i| usize::from(*i) < mesh.vertices.len()));
    let min_y = mesh
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = mesh
        .vertices
        .iter()
        .map(|v| v.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!((min_y, max_y), (-0.5, 4.));
    let benchmark = liteweight_fps::scene(5);
    assert_eq!(&benchmark.vertices[..mesh.vertices.len()], &mesh.vertices);
    assert_eq!(&benchmark.indices[..mesh.indices.len()], &mesh.indices);
}
