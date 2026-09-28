use glam::Vec3;
use liteweight_fps::{
    physics::Aabb,
    simulation::{
        player::{
            Input, DASH_SPEED, GRAVITY, JUMP_VELOCITY, PLAYER_HEIGHT, PLAYER_RADIUS, WALK_SPEED,
        },
        time::FixedStep,
        Player,
    },
    world,
};

fn input(left: bool, right: bool, forward: bool, back: bool, dash: bool, jump: bool) -> Input {
    Input {
        left,
        right,
        forward,
        back,
        dash,
        jump,
    }
}

#[test]
fn player_initializes_deterministically() {
    let p = Player::new();
    assert_eq!(p.position, Vec3::new(0.0, 1.6, 0.0));
    assert_eq!(p.yaw, 0.0);
    assert_eq!(p.pitch, 0.0);
    assert_eq!(p.velocity, Vec3::ZERO);
    assert!(p.on_ground);
}

#[test]
fn look_updates_from_mouse_delta_and_clamps_pitch() {
    let mut p = Player::new();
    p.look(0.1, -0.05);
    assert!((p.yaw - 0.1).abs() < 1e-6);
    assert!((p.pitch - 0.05).abs() < 1e-6);
    p.look(0., -2.);
    assert_eq!(p.pitch, 1.55);
    p.look(0., 4.);
    assert_eq!(p.pitch, -1.55);
    assert_eq!(p.yaw, 0.1);
}

#[test]
fn camera_matrix_uses_eye_position_and_look_direction() {
    let p = Player::new();
    let matrix = p.view_matrix(16.0 / 9.0);
    let eye = Vec3::new(0.0, 1.6, 0.0);
    let forward = Vec3::new(0.0, 0.0, -1.0);
    let ahead = matrix * (eye + forward * 10.).extend(1.);
    assert!((ahead.x / ahead.w).abs() < 1e-3);
    assert!((ahead.y / ahead.w).abs() < 1e-3);
    assert!(ahead.z / ahead.w < 1.0);
    let p = Player {
        position: Vec3::new(5., 1.6, -8.),
        ..Player::new()
    };
    let matrix = p.view_matrix(16.0 / 9.0);
    let ahead = matrix * (p.position + forward * 10.).extend(1.);
    assert!((ahead.x / ahead.w).abs() < 1e-3);
    assert!((ahead.y / ahead.w).abs() < 1e-3);
    let _ = world::arena();
}

#[test]
fn wasd_moves_relative_to_yaw_and_normalizes_diagonal() {
    let mut p = Player::new();
    let dt = 1.0 / 60.0;
    // 初期向きは前方-Z。Wで-Z方向へ歩行速度で進む。
    p.move_with_input(input(false, false, true, false, false, false), dt);
    assert!((p.position.z + WALK_SPEED * dt).abs() < 1e-4);
    assert!((p.position.x).abs() < 1e-6);
    // W+D は斜め移動。x方向と-z方向の速度の大きさは歩行速度。
    let mut p = Player::new();
    p.move_with_input(input(false, true, true, false, false, false), dt);
    let moved = glam::Vec2::new(p.position.x, p.position.z);
    assert!((moved.length() - WALK_SPEED * dt).abs() < 1e-4);
    assert!((moved.x / moved.y.abs()).abs() - 1.0 < 1e-4);
    // ダッシュ中は速い。
    let mut p = Player::new();
    p.move_with_input(input(false, false, true, false, true, false), dt);
    assert!((p.position.z + DASH_SPEED * dt).abs() < 1e-4);
    // yaw=90度で前方は+X方向。
    let mut p = Player::new();
    p.yaw = std::f32::consts::FRAC_PI_2;
    p.move_with_input(input(false, false, true, false, false, false), dt);
    assert!(p.position.x - WALK_SPEED * dt < 1e-4);
    assert!((p.position.z).abs() < 1e-6);
}

#[test]
fn left_and_right_keys_move_in_opposite_directions_and_cancel() {
    let dt = 1.0 / 60.0;
    // Aだけで左（-X方向）へ進む。
    let mut p = Player::new();
    p.move_with_input(input(true, false, false, false, false, false), dt);
    assert!(p.position.x < -1e-3);
    // Dだけで右（+X方向）へ進む。
    let mut p = Player::new();
    p.move_with_input(input(false, true, false, false, false, false), dt);
    assert!(p.position.x > 1e-3);
    // A+D同時押しでは水平方向に動かない。
    let mut p = Player::new();
    p.move_with_input(input(true, true, false, false, false, false), dt);
    assert!(p.position.x.abs() < 1e-6);
}

#[test]
fn jump_adds_initial_velocity_and_gravity_brings_player_back_to_ground() {
    let mut p = Player::new();
    let dt = 1.0 / 60.0;
    // 初回のジャンプ入力で上方向の初速が加わる（同ステップで重力が減算される）。
    p.move_with_input(input(false, false, false, false, false, true), dt);
    assert!((p.velocity.y - (JUMP_VELOCITY - GRAVITY * dt)).abs() < 1e-3);
    assert!(!p.on_ground);
    assert!(p.position.y > 1.6);
    // その後は重力で減速し、十分な時間後に床へ着地する。
    let mut ticks = 0;
    while !p.on_ground && ticks < 600 {
        p.move_with_input(input(false, false, false, false, false, false), dt);
        ticks += 1;
    }
    assert!(ticks < 600);
    assert!((p.position.y - 1.6).abs() < 1e-3);
    assert!(p.velocity.y.abs() < 1e-3);
    // 空中では落下速度が重力に従って増える。
    let mut p = Player::new();
    p.position.y = 3.0;
    p.on_ground = false;
    p.move_with_input(input(false, false, false, false, false, false), dt);
    assert!((p.velocity.y + GRAVITY * dt).abs() < 1e-3);
}

#[test]
fn horizontal_movement_works_while_airborne() {
    let mut p = Player::new();
    let dt = 1.0 / 60.0;
    p.move_with_input(input(false, false, false, false, false, true), dt);
    assert!(!p.on_ground);
    p.move_with_input(input(false, false, true, false, false, false), dt);
    assert!(!p.on_ground);
    assert!(p.position.z < -1e-3);
    assert!((p.velocity.y).abs() > 1e-3);
}

#[test]
fn wall_collision_stops_movement_with_sliding() {
    let dt = 1.0 / 60.0;
    // 遮蔽物の手前から前方へ十分な時間進んでも、壁を突き抜けない。
    let colliders = vec![Aabb::new(Vec3::new(-10., 0., -1.), Vec3::new(10., 4., 1.))];
    let mut p = Player::new();
    p.position.z = 2.0;
    for _ in 0..120 {
        p.move_with_collision(
            &colliders,
            input(false, false, true, false, false, false),
            dt,
        );
    }
    assert!(p.position.z >= 1.0 + PLAYER_RADIUS - 1e-4);
    // 斜め移動で壁に当たった場合は残った軸の方向へ滑る。
    let mut p = Player::new();
    p.position = Vec3::new(0.0, 1.6, 2.0);
    for _ in 0..120 {
        p.move_with_collision(
            &colliders,
            input(false, true, true, false, false, false),
            dt,
        );
    }
    assert!(p.position.z >= 1.0 + PLAYER_RADIUS - 1e-4);
    assert!(p.position.x > 1e-3);
}

#[test]
fn arena_colliders_cover_floor_walls_and_covers() {
    let colliders = world::arena_colliders();
    let hits_cover = colliders
        .iter()
        .any(|c| c.hits_cylinder(Vec3::new(0., 1., -8.), PLAYER_RADIUS, PLAYER_HEIGHT));
    assert!(hits_cover);
    let hits_wall = colliders
        .iter()
        .any(|c| c.hits_cylinder(Vec3::new(24.5, 2., 0.), PLAYER_RADIUS, PLAYER_HEIGHT));
    assert!(hits_wall);
}

#[test]
fn fixed_step_accumulates_and_caps_updates() {
    let mut step = FixedStep::new(60, 5);
    assert_eq!(step.updates(0.0), 0);
    // 1/120秒の経過で2回分には足りない。
    assert_eq!(step.updates(1.0 / 120.0), 0);
    // さらに1/120秒で合計1/60秒となり1回更新される。
    assert_eq!(step.updates(1.0 / 120.0), 1);
    // 残りは蓄積され、次の経過でまた1回。
    assert_eq!(step.updates(0.0), 0);
    assert_eq!(step.updates(1.0 / 60.0), 1);
    // 過負荷時は上限までに抑えられ、残りは捨てられる。
    let mut step = FixedStep::new(60, 5);
    assert_eq!(step.updates(10.0), 5);
    assert_eq!(step.updates(1.0 / 60.0), 1);
}
