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
fn adaptive_product_layout_tracks_display_aspect_without_letterboxing() {
    let mut layout = FrameLayout::new(1280, 720).unwrap();

    layout.resize_to_display_aspect(1000, 1000).unwrap();
    assert_eq!(layout.internal_size(), (1000, 1000));
    assert_eq!(layout.viewport(), Some((0, 0, 1000, 1000)));

    layout.resize_to_display_aspect(2100, 900).unwrap();
    assert_eq!(layout.internal_size(), (2100, 900));
    assert_eq!(layout.viewport(), Some((0, 0, 2100, 900)));

    layout.resize_to_display_aspect(6000, 1000).unwrap();
    assert_eq!(layout.internal_size(), (4096, 683));
    layout.resize_to_display_aspect(1920, 1080).unwrap();
    assert_eq!(layout.internal_size(), (1920, 1080));

    layout.resize_to_display_aspect(0, 900).unwrap();
    assert_eq!(layout.viewport(), None);
    layout.resize_to_display_aspect(900, 900).unwrap();
    assert_eq!(layout.viewport(), Some((0, 0, 900, 900)));
}

#[test]
fn internal_resolution_scale_changes_pixel_density_without_changing_projection_aspect() {
    let mut layout = FrameLayout::new(1280, 720).unwrap();
    layout.resize_to_display_scale(1920, 1080, 0.5).unwrap();
    assert_eq!(layout.internal_size(), (960, 540));
    assert_eq!(layout.viewport(), Some((0, 0, 1920, 1080)));
    assert!(layout.resize_to_display_scale(1920, 1080, 0.2).is_err());
    assert_eq!(layout.viewport(), Some((0, 0, 1920, 1080)));
    assert_eq!(layout.internal_size(), (960, 540));
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
    assert!(mesh.vertices.len() > 11 * 24);
    assert!(mesh.indices.len() > 11 * 36);
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
    assert_eq!((min_y, max_y), (-0.5, 12.));
    let benchmark = liteweight_fps::scene(5);
    assert_eq!(&benchmark.vertices[..mesh.vertices.len()], &mesh.vertices);
    assert_eq!(&benchmark.indices[..mesh.indices.len()], &mesh.indices);
}

#[test]
fn arena_size_scales_with_bot_population_while_default_stays_50_meters() {
    use liteweight_fps::world::{arena_colliders_scaled, arena_scale_for_bot_count, arena_scaled};

    assert_eq!(arena_scale_for_bot_count(5), 1.0);
    assert!(arena_scale_for_bot_count(1) < 1.0);
    assert!(arena_scale_for_bot_count(10) > 1.0);
    assert!((arena_scale_for_bot_count(20) - 2.2).abs() < f32::EPSILON);
    assert_eq!(arena_scale_for_bot_count(21), arena_scale_for_bot_count(20));
    let normal = liteweight_fps::world::arena();
    let large = arena_scaled(arena_scale_for_bot_count(20));
    let min_z = |mesh: &liteweight_fps::Scene| {
        mesh.vertices
            .iter()
            .map(|vertex| vertex.position[2])
            .fold(f32::INFINITY, f32::min)
    };
    assert_eq!(min_z(&normal), -25.25);
    assert!(min_z(&large) < min_z(&normal));
    assert!(
        arena_colliders_scaled(2.2)[1].min.x < liteweight_fps::world::arena_colliders()[1].min.x
    );
}

#[test]
fn arena_contains_stair_steps_and_a_low_poly_ramp() {
    let mesh = liteweight_fps::world::arena();
    let has_vertex = |expected: [f32; 3]| {
        mesh.vertices
            .iter()
            .any(|vertex| (0..3).all(|axis| (vertex.position[axis] - expected[axis]).abs() < 1e-5))
    };
    assert!(
        has_vertex([20.9, 0.0, 4.5]),
        "ramp must begin beside the east wall"
    );
    assert!(
        has_vertex([20.9, 1.8, -3.5]),
        "ramp must reach the wall-side landing"
    );
    assert!(
        has_vertex([-24.1, 1.8, -0.5]),
        "stairs must reach their landing beside the west wall"
    );
    assert!(mesh
        .indices
        .iter()
        .all(|index| usize::from(*index) < mesh.vertices.len()));
}
