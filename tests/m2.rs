use glam::Vec3;
use liteweight_fps::{
    engine::grid_shade,
    physics::Aabb,
    simulation::{
        player::{
            Input, DASH_SPEED, EYE_HEIGHT, GRAVITY, JUMP_VELOCITY, PLAYER_HEIGHT, PLAYER_RADIUS,
            WALK_SPEED,
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
fn body_center_and_foot_height_derive_from_eye_position_consistently() {
    let p = Player::new();
    // 接地時: 視点1.6m、身体高1.8m。足元は0m、身体中心は0.9m。
    assert_eq!(p.foot_height(), 0.0);
    assert_eq!(p.body_center().y, 0.9);
    assert_eq!(p.body_center().x, p.position.x);
    assert_eq!(p.body_center().z, p.position.z);
    // ジャンプなどで視点が上がっても、足元・中心は視点から一貫して算出される。
    let mut p = Player::new();
    p.position.y = EYE_HEIGHT + 0.5;
    assert!((p.foot_height() - 0.5).abs() < 1e-4);
    assert!((p.body_center().y - (0.5 + PLAYER_HEIGHT * 0.5)).abs() < 1e-4);
}

#[test]
fn low_barrier_blocks_grounded_player_with_body_center_collision() {
    let dt = 1.0 / 60.0;
    // 高さ0.5mの低い障害物。視点(1.6m)は障害物の上にあるが、足元〜身体下部が当たるべき。
    let colliders = vec![Aabb::new(Vec3::new(-10., 0., -1.), Vec3::new(10., 0.5, 1.))];
    let mut p = Player::new();
    p.position.z = 2.0;
    for _ in 0..120 {
        p.move_with_collision(
            &colliders,
            input(false, false, true, false, false, false),
            dt,
        );
    }
    // 身体中心(0.9m)が障害物(0〜0.5m)に当たり、それ以上進まない。
    assert!(p.position.z >= 1.0 + PLAYER_RADIUS - 1e-4);
    // 視点は障害物の上端より上にある（めり込んでいない）。
    assert!(p.position.y >= 0.5);
}

#[test]
fn eye_never_enters_wall_in_front_side_and_corner_contacts() {
    let dt = 1.0 / 60.0;
    // 壁AABB。視点（position）が内部(x:-1〜1, z:-1〜1)に入らないことを検証する。
    let wall = Aabb::new(Vec3::new(-1., 0., -1.), Vec3::new(1., 4., 1.));
    let colliders = vec![wall];
    let cases = [
        (
            "front",
            Vec3::new(0., 1.6, 3.),
            input(false, false, true, false, false, false),
        ),
        (
            "dash",
            Vec3::new(0., 1.6, 3.),
            input(false, false, true, false, true, false),
        ),
        (
            "side",
            Vec3::new(3., 1.6, 0.),
            input(true, false, false, false, false, false),
        ),
        (
            "diagonal",
            Vec3::new(3., 1.6, 3.),
            input(false, true, true, false, false, false),
        ),
        (
            "corner",
            Vec3::new(-2., 1.6, -2.),
            input(true, true, false, false, false, false),
        ),
    ];
    for (name, start, inp) in cases {
        let mut p = Player::new();
        p.position = start;
        for _ in 0..120 {
            p.move_with_collision(&colliders, inp, dt);
        }
        assert!(
            !wall.hits_cylinder(p.body_center(), PLAYER_RADIUS, PLAYER_HEIGHT),
            "{name}: 視点が壁内部へ入った position={:?}",
            p.position
        );
        // 視点（eye）もAABBの水平内部(x:-1〜1, z:-1〜1)に入らない。
        assert!(
            !(p.position.x > wall.min.x
                && p.position.x < wall.max.x
                && p.position.z > wall.min.z
                && p.position.z < wall.max.z),
            "{name}: 視点が壁の水平範囲内部へ入った position={:?}",
            p.position
        );
    }
}

#[test]
fn jumping_into_wall_does_not_embed_eye() {
    let dt = 1.0 / 60.0;
    // 高さ4mの壁。ジャンプしながら正面へ進んでも視点が内部へ入らない。
    let wall = Aabb::new(Vec3::new(-10., 0., -1.), Vec3::new(10., 4., 1.));
    let colliders = vec![wall];
    let mut p = Player::new();
    p.position.z = 3.0;
    for i in 0..180 {
        // 最初のステップでジャンプを発火し、その後も前進を続ける。
        let jump = i == 0;
        p.move_with_collision(
            &colliders,
            input(false, false, true, false, false, jump),
            dt,
        );
    }
    // 視点が壁の水平内部に入らない。
    assert!(p.position.z >= 1.0 + PLAYER_RADIUS - 1e-4);
    // 身体中心も壁と重ならない。
    assert!(
        !wall.hits_cylinder(p.body_center(), PLAYER_RADIUS, PLAYER_HEIGHT),
        "ジャンプ中の接触で身体が壁内部へ入った position={:?}",
        p.position
    );
}

#[test]
fn sliding_along_wall_does_not_jitter() {
    let dt = 1.0 / 60.0;
    // 壁面z=1。その手前(z=1.3)を保ったまま+x方向へ滑る。
    let wall = Aabb::new(Vec3::new(-10., 0., -1.), Vec3::new(10., 4., 1.));
    let colliders = vec![wall];
    let mut p = Player::new();
    p.position = Vec3::new(0., 1.6, 1.0 + PLAYER_RADIUS);
    let mut previous_z = p.position.z;
    for _ in 0..120 {
        p.move_with_collision(
            &colliders,
            input(false, true, false, false, false, false),
            dt,
        );
        // 壁から離れすぎず、めり込んでもいない。
        assert!((p.position.z - (1.0 + PLAYER_RADIUS)).abs() < 1e-3);
        // +xへ進んでいる。
        assert!(p.position.x > 0.);
        previous_z = p.position.z;
    }
    let _ = previous_z;
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
fn sustained_recoil_can_be_countered_with_mouse_input() {
    let mut player = Player::new();
    for _ in 0..120 {
        player.apply_recoil(0.012);
        player.look(0.0, 0.012);
    }
    assert!(player.pitch.abs() < 1e-5);

    player.apply_recoil(0.045);
    assert!(player.pitch > 0.0);
    player.look(0.0, 0.045);
    assert!(player.pitch.abs() < 1e-5);
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
    // 初期向きは前方-Z。加速後の定常速度は歩行速度。
    for _ in 0..120 {
        p.move_with_input(input(false, false, true, false, false, false), dt);
    }
    assert!((p.velocity.length() - WALK_SPEED).abs() < 1e-3);
    assert!((p.position.x).abs() < 1e-6);
    // W+D は斜め移動。x方向と-z方向の速度の大きさは歩行速度。
    let mut p = Player::new();
    for _ in 0..120 {
        p.move_with_input(input(false, true, true, false, false, false), dt);
    }
    assert!((p.velocity.length() - WALK_SPEED).abs() < 1e-3);
    assert!((p.velocity.x / p.velocity.z.abs()).abs() - 1.0 < 1e-4);
    // ダッシュ中は速い。
    let mut p = Player::new();
    for _ in 0..120 {
        p.move_with_input(input(false, false, true, false, true, false), dt);
    }
    assert!((p.velocity.length() - DASH_SPEED).abs() < 1e-3);
    // yaw=90度で前方は+X方向。
    let mut p = Player::new();
    p.yaw = std::f32::consts::FRAC_PI_2;
    for _ in 0..120 {
        p.move_with_input(input(false, false, true, false, false, false), dt);
    }
    assert!(p.velocity.x > WALK_SPEED - 1e-3);
    assert!((p.position.z).abs() < 1e-6);
}

#[test]
fn horizontal_movement_starts_and_stops_at_full_speed_immediately() {
    let dt = 1.0 / 60.0;
    let mut player = Player::new();
    player.move_with_input(input(false, false, true, false, false, false), dt);
    assert!((player.velocity.length() - WALK_SPEED).abs() < 1e-3);

    player.move_with_input(input(false, false, false, false, false, false), dt);
    assert_eq!(player.velocity.x, 0.0);
    assert_eq!(player.velocity.z, 0.0);
}

#[test]
fn dash_speed_and_visuals_transition_continuously() {
    let mut player = Player::new();
    let dt = 1.0 / 60.0;
    player.move_with_input(input(false, false, true, false, true, false), dt);
    assert!(player.dash_progress() > 0.0 && player.dash_progress() < 1.0);
    assert!((player.velocity.length() - DASH_SPEED).abs() < 1e-3);
    assert!(player.dash_weapon_offset() < 0.0);

    for _ in 0..120 {
        player.move_with_input(input(false, false, true, false, true, false), dt);
    }
    assert!((player.velocity.length() - DASH_SPEED).abs() < 1e-3);

    player.move_with_input(input(false, false, false, false, false, false), dt);
    assert!(player.dash_progress() < 1.0);
}

#[test]
fn player_can_walk_from_spawn_while_touching_the_arena_floor() {
    let mut player = Player::new();
    let colliders = world::arena_colliders_scaled(1.0);

    for _ in 0..30 {
        player.move_with_collision(
            &colliders,
            input(false, false, true, false, false, false),
            1.0 / 60.0,
        );
    }

    assert!(player.position.z < -1.0);
    assert_eq!(player.position.y, EYE_HEIGHT);
    assert!(player.on_ground);
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
fn can_jump_again_after_landing() {
    let dt = 1.0 / 60.0;
    let colliders = world::arena_colliders();
    let mut p = Player::new();
    // 1回目のジャンプ。
    p.move_with_collision(
        &colliders,
        input(false, false, false, false, false, true),
        dt,
    );
    assert!(!p.on_ground);
    // 着地するまで進める。
    let mut ticks = 0;
    while !p.on_ground && ticks < 600 {
        p.move_with_collision(
            &colliders,
            input(false, false, false, false, false, false),
            dt,
        );
        ticks += 1;
    }
    assert!(ticks < 600);
    assert!(p.on_ground);
    // 着地後の再ジャンプでも空中へ飛べる。
    p.move_with_collision(
        &colliders,
        input(false, false, false, false, false, true),
        dt,
    );
    assert!(!p.on_ground, "着地後の再ジャンプができない");
    assert!(p.velocity.y > 0.0);
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
    assert!(colliders[1..5].iter().all(|wall| wall.max.y == 12.0));
    assert!(colliders[5..].iter().all(|cover| cover.max.y == 5.0));
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

#[test]
fn grid_shade_is_thin_and_identifies_surface() {
    use glam::Vec3;
    // 水平面（法線+Y）: x/zの格子。
    assert_eq!(
        grid_shade(Vec3::new(0.0, 1.5, 0.0), Vec3::Y, 1.0, 0.03),
        0.94
    );
    assert_eq!(
        grid_shade(Vec3::new(1.0, 1.5, 0.3), Vec3::Y, 1.0, 0.03),
        0.94
    );
    assert_eq!(
        grid_shade(Vec3::new(0.5, 1.5, 0.5), Vec3::Y, 1.0, 0.03),
        1.0
    );
    // 垂直面（法線+Z）: 高さyの水平線も描かれる。
    assert_eq!(
        grid_shade(Vec3::new(1.5, 1.0, 1.5), Vec3::Z, 1.0, 0.03),
        0.94
    );
    assert_eq!(
        grid_shade(Vec3::new(1.5, 0.5, 1.5), Vec3::Z, 1.0, 0.03),
        1.0
    );
    // 負の座標でも同じ格子パターンになる。
    assert_eq!(
        grid_shade(Vec3::new(-1.0, 1.5, 0.3), Vec3::Y, 1.0, 0.03),
        0.94
    );
    assert_eq!(
        grid_shade(Vec3::new(-0.5, 1.5, 0.5), Vec3::Y, 1.0, 0.03),
        1.0
    );
    // 線幅が0なら常に明るい（模様なし）。
    assert_eq!(grid_shade(Vec3::new(0.0, 0.0, 0.0), Vec3::Y, 1.0, 0.0), 1.0);
}
