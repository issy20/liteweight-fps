use glam::Vec3;
use liteweight_fps::{
    physics::Aabb,
    simulation::combat::{Bot, BotBehavior, Combat, BOT_MIN_RESPAWN_DISTANCE, BOT_RESPAWN_SECONDS},
};

#[test]
fn training_starts_with_five_bots_and_all_three_behaviors() {
    let combat = Combat::default();
    assert_eq!(combat.targets.len(), 5);
    assert!(combat.targets.iter().all(|bot| bot.alive && bot.hp == 100));
    assert!(combat
        .targets
        .iter()
        .any(|bot| bot.behavior == BotBehavior::Static));
    assert!(combat
        .targets
        .iter()
        .any(|bot| bot.behavior == BotBehavior::Strafe));
    assert!(combat
        .targets
        .iter()
        .any(|bot| bot.behavior == BotBehavior::Chase));
}

#[test]
fn static_strafe_and_chase_behaviors_have_distinct_motion() {
    let bots = vec![
        Bot::with_behavior(Vec3::new(-8.0, 0.0, 0.0), BotBehavior::Static, 0),
        Bot::with_behavior(Vec3::new(0.0, 0.0, -8.0), BotBehavior::Strafe, 1),
        Bot::with_behavior(Vec3::new(8.0, 0.0, 0.0), BotBehavior::Chase, 99),
    ];
    let mut combat = Combat::new(bots);
    let initial = combat
        .targets
        .iter()
        .map(|bot| bot.position)
        .collect::<Vec<_>>();
    let player = Vec3::ZERO;
    for _ in 0..180 {
        combat.advance(1.0 / 60.0, player, &[]);
    }

    assert_eq!(combat.targets[0].position, initial[0]);
    assert_ne!(combat.targets[1].position, initial[1]);
    assert!(combat.targets[2].position.distance(player) < initial[2].distance(player));
}

#[test]
fn strafing_bot_reverses_direction_smoothly_for_tracking_practice() {
    let mut bot = Bot::with_behavior(Vec3::ZERO, BotBehavior::Strafe, 0);
    bot.advance_motion(0, 1.0 / 60.0, Vec3::new(0.0, 0.0, -10.0), &[], &[]);
    let mut previous_velocity = (bot.position.x) * 60.0;
    let mut peak_velocity = 0.0_f32;
    for _ in 0..180 {
        let before = bot.position.x;
        bot.advance_motion(0, 1.0 / 60.0, Vec3::new(0.0, 0.0, -10.0), &[], &[]);
        let velocity = (bot.position.x - before) * 60.0;
        assert!((velocity - previous_velocity).abs() < 0.08);
        peak_velocity = peak_velocity.max(velocity.abs());
        previous_velocity = velocity;
    }
    assert!(
        peak_velocity > 1.5,
        "sweeps should remain wide enough to track"
    );
}

#[test]
fn killed_bot_stays_dead_then_respawns_without_resetting_score() {
    let bot = Bot::new(Vec3::new(0.0, 0.0, -5.0));
    let mut combat = Combat::new(vec![bot]);
    let hit = combat.shoot(Vec3::new(0.0, 1.0, 0.0), -Vec3::Z, &[], 100);
    assert!(hit.killed);
    assert_eq!(combat.kills, 1);
    assert!(!combat.targets[0].alive);
    assert_eq!(combat.targets[0].hp, 0);
    assert!(
        !combat
            .shoot(Vec3::new(0.0, 1.0, 0.0), -Vec3::Z, &[], 100)
            .hit
    );

    combat.advance(BOT_RESPAWN_SECONDS - 0.01, Vec3::ZERO, &[]);
    assert!(!combat.targets[0].alive);
    combat.advance(0.02, Vec3::ZERO, &[]);
    assert!(combat.targets[0].alive);
    assert_eq!(combat.targets[0].hp, 100);
    assert_eq!(combat.kills, 1);
    assert!(combat.targets[0].position.distance(Vec3::ZERO) >= BOT_MIN_RESPAWN_DISTANCE);
}

#[test]
fn respawn_does_not_overlap_other_live_bots_and_dead_bot_is_not_rendered() {
    let mut combat = Combat::default();
    let single_bot_vertices =
        liteweight_fps::ui::world_mesh(&Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]))
            .vertices
            .len();
    let vertices_before = liteweight_fps::ui::world_mesh(&combat).vertices.len();
    let hit = combat.shoot(Vec3::new(0.0, 1.0, 0.0), -Vec3::Z, &[], 100);
    assert!(hit.killed);
    combat.feedback = 0.0;
    let vertices_after_death = liteweight_fps::ui::world_mesh(&combat).vertices.len();
    assert_eq!(vertices_before - vertices_after_death, single_bot_vertices);

    combat.advance(BOT_RESPAWN_SECONDS + 0.01, Vec3::ZERO, &[]);
    assert!(combat.targets[0].alive);
    for i in 0..combat.targets.len() {
        for j in (i + 1)..combat.targets.len() {
            assert!(
                combat.targets[i]
                    .position
                    .distance(combat.targets[j].position)
                    >= 0.72
            );
        }
    }
}

#[test]
fn repeated_respawns_are_randomized_and_remain_valid_across_the_arena() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
    combat.set_respawn_seed(0x1234_5678);
    let walls = liteweight_fps::world::arena_colliders();
    let mut positions = Vec::new();
    for _ in 0..8 {
        combat.targets[0].apply_damage(100);
        combat.advance(BOT_RESPAWN_SECONDS + 0.01, Vec3::ZERO, &walls);
        let position = combat.targets[0].position;
        assert!(combat.targets[0].can_spawn_at(position, 0, &walls, &[Some(position)]));
        positions.push(position);
    }

    assert!(positions
        .iter()
        .all(|position| { position.distance(Vec3::ZERO) >= BOT_MIN_RESPAWN_DISTANCE }));
    assert!(positions.windows(2).all(|pair| pair[0] != pair[1]));

    let span = |component: fn(Vec3) -> f32| {
        let (min, max) = positions
            .iter()
            .map(|position| component(*position))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                (min.min(value), max.max(value))
            });
        max - min
    };
    assert!(span(|position| position.x) >= 20.0);
    assert!(span(|position| position.z) >= 20.0);
}

#[test]
fn free_training_respawn_sequence_is_reproducible_per_seed() {
    let sample = |seed| {
        let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
        combat.set_respawn_seed(seed);
        let mut positions = Vec::new();
        for _ in 0..8 {
            combat.targets[0].apply_damage(100);
            combat.advance(BOT_RESPAWN_SECONDS + 0.01, Vec3::ZERO, &[]);
            positions.push(combat.targets[0].position);
        }
        positions
    };

    assert_eq!(sample(0x1234_5678), sample(0x1234_5678));
    assert_ne!(sample(0x1234_5678), sample(0x8765_4321));
}

#[test]
fn free_training_respawn_does_not_overlap_another_live_bot() {
    let other_position = Vec3::new(-20.0, 0.0, -20.0);
    let mut combat = Combat::new(vec![
        Bot::new(Vec3::new(0.0, 0.0, -5.0)),
        Bot::new(other_position),
    ]);
    combat.set_respawn_seed(0x1234_5678);
    combat.targets[0].apply_damage(100);
    combat.advance(BOT_RESPAWN_SECONDS + 0.01, Vec3::ZERO, &[]);

    assert!(combat.targets[0].position.distance(other_position) >= 0.72);
    assert!(combat.targets[0].position.distance(Vec3::ZERO) >= BOT_MIN_RESPAWN_DISTANCE);
}

#[test]
fn bot_render_mesh_is_a_faceless_mannequin_with_clear_head_accent() {
    let mut bot = Bot::new(Vec3::new(0.0, 0.0, -5.0));
    bot.yaw = 0.0;
    let combat = Combat::new(vec![bot]);
    let mesh = liteweight_fps::ui::world_mesh(&combat);
    assert!(
        mesh.vertices.len() > 48,
        "BOT should use a multi-part silhouette"
    );
    assert!(
        mesh.vertices.len() <= 200,
        "low-poly mannequin should use at most 200 vertices per BOT, got {}",
        mesh.vertices.len()
    );
    let width_at_height = |height: f32| {
        mesh.vertices
            .iter()
            .filter(|vertex| (vertex.position[1] - height).abs() < 0.002)
            .map(|vertex| vertex.position[0])
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
                (min.min(x), max.max(x))
            })
    };
    let waist_width = width_at_height(1.17);
    let chest_width = width_at_height(1.38);
    let shoulder_width = width_at_height(1.50);
    let hip_width = width_at_height(0.96);
    let chest_depth = mesh
        .vertices
        .iter()
        .filter(|vertex| (vertex.position[1] - 1.38).abs() < 0.002)
        .map(|vertex| vertex.position[2])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), z| {
            (min.min(z), max.max(z))
        });
    assert!(
        chest_width.1 - chest_width.0 > waist_width.1 - waist_width.0 + 0.1,
        "chest should be visibly broader than the waist"
    );
    assert!(
        chest_depth.1 - chest_depth.0 >= (chest_width.1 - chest_width.0) * 0.60,
        "chest should retain enough depth when viewed from the side"
    );
    assert!(
        shoulder_width.1 - shoulder_width.0 > chest_width.1 - chest_width.0 + 0.02,
        "shoulders should be broader than the chest"
    );
    assert!(
        hip_width.1 - hip_width.0 >= 0.30,
        "hips should support the longer legs"
    );
    let head_poles = mesh
        .vertices
        .iter()
        .filter(|vertex| {
            vertex.position[0].abs() < 0.001 && (vertex.position[2] + 5.0).abs() < 0.001
        })
        .filter(|vertex| vertex.position[1] > 1.7)
        .map(|vertex| vertex.position[1])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
            (min.min(y), max.max(y))
        });
    assert!(
        head_poles.0 >= 1.76,
        "head should be about one eighth of total height"
    );
    assert!(
        (head_poles.1 - 2.03).abs() < 0.001,
        "overall height should remain unchanged"
    );
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == [1.0, 0.48, 0.08, 1.0]));
    assert!(mesh
        .indices
        .iter()
        .all(|index| (*index as usize) < mesh.vertices.len()));
    assert!(mesh.vertices.iter().all(|vertex| vertex
        .position
        .iter()
        .all(|coordinate| coordinate.is_finite())));
    let model_width = mesh
        .vertices
        .iter()
        .map(|vertex| vertex.position[0])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    assert!(
        model_width.1 - model_width.0 <= 0.95,
        "BOT should have a slender human silhouette"
    );
    let accent_width = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.color == [1.0, 0.48, 0.08, 1.0])
        .map(|vertex| vertex.position[0])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    assert!(
        accent_width.1 - accent_width.0 <= 0.28,
        "head accent should stay inside the compact head silhouette"
    );
    let accent_depth = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.color == [1.0, 0.48, 0.08, 1.0])
        .map(|vertex| vertex.position[2])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), z| {
            (min.min(z), max.max(z))
        });
    assert!(
        accent_depth.1 - accent_depth.0 <= 0.04,
        "head accent should be a small frontal marker, not a full ring"
    );
    let head_width = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.position[1] >= 1.77)
        .map(|vertex| vertex.position[0])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    assert!(
        head_width.1 - head_width.0 <= 0.28,
        "head should be smaller than the current mannequin head: {}",
        head_width.1 - head_width.0
    );
    let height = mesh
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), y| {
            (min.min(y), max.max(y))
        });
    assert!((height.0 - 0.0).abs() < 0.001);
    assert!(
        (height.1 - 2.03).abs() < 0.001,
        "BOT height should remain unchanged"
    );

    // Torso and head hitbox bounds follow the visible mannequin proportions.
    assert_eq!(
        combat.targets[0].hitboxes()[0].bounds,
        Aabb::new(Vec3::new(-0.24, 0.96, -5.15), Vec3::new(0.24, 1.67, -4.85))
    );
    assert_eq!(
        combat.targets[0].hitboxes()[5].bounds,
        Aabb::new(
            Vec3::new(-0.085, 1.77, -5.082),
            Vec3::new(0.085, 2.03, -4.918)
        )
    );
}

#[test]
fn bots_have_varied_horizontal_facing_on_spawn_and_turn_on_respawn() {
    use liteweight_fps::simulation::bot::{Bot, BotBehavior};

    let mut bots: Vec<_> = (0..5)
        .map(|slot| Bot::with_behavior(Vec3::ZERO, BotBehavior::Static, slot))
        .collect();
    assert!(bots
        .iter()
        .all(|bot| (0.0..std::f32::consts::TAU).contains(&bot.yaw)));
    assert!(bots
        .windows(2)
        .all(|pair| (pair[0].yaw - pair[1].yaw).abs() > 0.1));

    let previous_yaw = bots[0].yaw;
    bots[0].respawn(Vec3::new(1.0, 0.0, 0.0));
    assert_ne!(bots[0].yaw, previous_yaw);
}

#[test]
fn rotated_bot_hitboxes_follow_the_mannequin_orientation() {
    let mut bot = Bot::new(Vec3::new(0.0, 0.0, 0.0));
    bot.yaw = std::f32::consts::FRAC_PI_2;
    let mut combat = Combat::new(vec![bot]);
    let shot = combat.shoot(Vec3::new(-2.0, 1.38, 0.0), Vec3::X, &[], 25);

    assert!(
        shot.hit,
        "the rotated chest should be hittable from its front"
    );
    assert_eq!(combat.targets[0].hp, 75);
}

#[test]
fn moving_bots_use_individual_horizontal_lanes() {
    for behavior in [BotBehavior::Strafe, BotBehavior::TrackingStrafe] {
        let mut bots: Vec<_> = (0..5)
            .map(|slot| Bot::with_behavior(Vec3::new(slot as f32 * 7.0, 0.0, 0.0), behavior, slot))
            .collect();
        let starts: Vec<_> = bots.iter().map(|bot| bot.position).collect();
        for _ in 0..20 {
            for (index, bot) in bots.iter_mut().enumerate() {
                bot.advance_motion(index, 1.0 / 60.0, Vec3::ZERO, &[], &[]);
            }
        }
        let directions: Vec<_> = bots
            .iter()
            .zip(starts)
            .map(|(bot, start)| (bot.position - start).normalize_or_zero())
            .collect();

        assert!(directions.iter().all(|direction| direction.is_finite()));
        assert!(directions.iter().enumerate().any(|(index, direction)| {
            directions[index + 1..]
                .iter()
                .any(|other| direction.dot(*other).abs() < 0.95)
        }));
    }
}

#[test]
fn mannequin_uses_short_neck_and_longer_legs() {
    let mut bot = Bot::new(Vec3::new(0.0, 0.0, -5.0));
    bot.yaw = 0.0;
    let mesh = liteweight_fps::ui::world_mesh(&Combat::new(vec![bot]));
    let horizontal_span_at = |height: f32| {
        let xs = mesh
            .vertices
            .iter()
            .filter(|vertex| (vertex.position[1] - height).abs() < 0.002)
            .map(|vertex| vertex.position[0])
            .collect::<Vec<_>>();
        let min = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let max = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        max - min
    };

    assert!(
        horizontal_span_at(0.96) >= 0.30,
        "raised hip/upper-leg junction should make the legs read longer"
    );
    assert!(
        horizontal_span_at(1.67) >= 0.30,
        "raised shoulder line should visually shorten the exposed neck"
    );
    let neck_width = mesh
        .vertices
        .iter()
        .filter(|vertex| (vertex.position[1] - 1.78).abs() < 0.025)
        .map(|vertex| vertex.position[0])
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    assert!(
        neck_width.1 - neck_width.0 <= 0.07,
        "neck should stay narrow between the shoulders and head"
    );
}

#[test]
fn twenty_bot_render_mesh_stays_within_renderer_buffer_limits() {
    let bots = (0..20)
        .map(|slot| Bot::new(Vec3::new(slot as f32 * 3.0, 0.0, -5.0 - slot as f32)))
        .collect();
    let mesh = liteweight_fps::ui::world_mesh(&Combat::new(bots));
    assert!(mesh.vertices.len() <= 8192);
    assert!(
        mesh.indices.len() > 4096,
        "test should exercise a multi-BOT mesh"
    );
    assert!(mesh.indices.len() <= 32768);
}

#[test]
fn world_mesh_builder_reuses_cpu_buffer_capacity() {
    let combat = Combat::new(
        (0..20)
            .map(|index| Bot::new(Vec3::new(index as f32, 0.0, -5.0)))
            .collect(),
    );
    let mut scratch = liteweight_fps::Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    liteweight_fps::ui::world_mesh_into(&mut scratch, &combat);
    let vertex_capacity = scratch.vertices.capacity();
    let index_capacity = scratch.indices.capacity();
    let expected = liteweight_fps::ui::world_mesh(&combat);
    liteweight_fps::ui::world_mesh_into(&mut scratch, &combat);

    assert_eq!(scratch.vertices, expected.vertices);
    assert_eq!(scratch.indices, expected.indices);
    assert_eq!(scratch.vertices.capacity(), vertex_capacity);
    assert_eq!(scratch.indices.capacity(), index_capacity);
}

#[test]
fn body_and_head_hits_report_damage_and_headshot_status() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
    let body = combat.shoot(Vec3::new(0.0, 1.0, 0.0), -Vec3::Z, &[], 25);
    assert!(body.hit && !body.headshot);
    assert!(!body.killed);
    assert_eq!(combat.targets[0].hp, 75);

    let head = combat.shoot(Vec3::new(0.0, 1.8, 0.0), -Vec3::Z, &[], 25);
    assert!(head.hit && head.headshot);
    assert!(!head.killed);
    assert_eq!(combat.targets[0].hp, 25);
    assert_eq!(combat.kills, 0);
}

#[test]
fn hitscan_does_not_hit_in_the_small_neck_gap_between_torso_and_head() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
    let shot = combat.shoot(Vec3::new(0.0, 1.72, 0.0), -Vec3::Z, &[], 25);
    assert!(
        !shot.hit,
        "the small neck gap must not be covered by oversized boxes"
    );
}

#[test]
fn hitscan_does_not_hit_empty_space_outside_the_rendered_shoulder() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
    let shot = combat.shoot(
        Vec3::new(0.0, 1.58, 0.0),
        Vec3::new(-0.08, 0.0, -1.0),
        &[],
        25,
    );

    assert!(!shot.hit, "the ray passes beside the visible shoulder");
    assert_eq!(combat.targets[0].hp, 100);
}

#[test]
fn hitscan_does_not_hit_outside_the_narrow_torso_waist() {
    let mut bot = Bot::new(Vec3::new(0.0, 0.0, -5.0));
    bot.yaw = 0.0;
    let mut combat = Combat::new(vec![bot]);
    let direction = Vec3::new(0.15 / 4.85, 0.0, -1.0);
    let shot = combat.shoot(Vec3::new(0.0, 1.17, 0.0), direction, &[], 25);

    assert!(
        !shot.hit,
        "the ray passes outside the visible waist contour"
    );
    assert_eq!(combat.targets[0].hp, 100);
}

#[test]
fn muzzle_shot_converges_on_camera_aim_and_still_respects_cover() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
    let corrected = combat.shoot_from_muzzle(
        Vec3::new(0.0, 1.4, 0.0),
        Vec3::new(0.3, 1.2, 0.4),
        -Vec3::Z,
        &[],
        25,
    );
    assert!(
        corrected.hit,
        "muzzle parallax should be corrected toward the reticle"
    );

    let wall = Aabb::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::new(2.0, 3.0, -1.5));
    let blocked = combat.shoot_from_muzzle(
        Vec3::new(0.0, 1.4, 0.0),
        Vec3::new(0.3, 1.2, 0.4),
        -Vec3::Z,
        &[wall],
        25,
    );
    assert!(
        !blocked.hit,
        "the camera aim point must not let muzzle shots pass cover"
    );
}

#[test]
fn muzzle_parallax_cannot_damage_a_bot_outside_the_reticle_ray() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.5, 0.0, -5.0))]);
    let shot = combat.shoot_from_muzzle(
        Vec3::new(0.0, 1.4, 0.0),
        Vec3::new(0.3, 1.2, 0.4),
        -Vec3::Z,
        &[],
        25,
    );

    assert!(!shot.hit && !shot.impact, "the reticle ray misses this BOT");
    assert_eq!(combat.targets[0].hp, 100);
}

#[test]
fn shot_direction_cannot_select_a_bot_that_the_reticle_misses() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.5, 0.0, -5.0))]);
    let shot = combat.shoot_from_muzzle_with_aim(
        Vec3::new(0.0, 1.4, 0.0),
        Vec3::new(0.3, 1.2, 0.4),
        -Vec3::Z,
        Vec3::new(0.03, 0.0, -1.0),
        &[],
        25,
    );

    assert!(
        !shot.hit && !shot.impact,
        "the shot must not hit a BOT outside the reticle direction"
    );
    assert_eq!(combat.targets[0].hp, 100);
}

#[test]
fn bot_population_configuration_scales_spawns_and_respawns_with_arena() {
    let mut small = Combat::new(Vec::new());
    small.configure_bots_with_arena_scale(5, 1, 100, 1.0, 2.0, 0.68);
    let mut large = Combat::new(Vec::new());
    large.configure_bots_with_arena_scale(5, 1, 100, 1.0, 2.0, 1.4);
    assert!(large.targets[3].position.x.abs() > small.targets[3].position.x.abs());
    assert_eq!(large.arena_scale, 1.4);
    assert!(large.targets.iter().all(|bot| bot.arena_scale == 1.4));
}

#[test]
fn chase_bot_does_not_pass_through_a_wall() {
    let wall = Aabb::new(Vec3::new(-20.0, 0.0, -5.0), Vec3::new(20.0, 3.0, -4.5));
    let mut combat = Combat::new(vec![Bot::with_behavior(
        Vec3::new(0.0, 0.0, -10.0),
        BotBehavior::Chase,
        99,
    )]);
    for _ in 0..600 {
        combat.advance(1.0 / 60.0, Vec3::ZERO, &[wall]);
    }
    assert!(combat.targets[0].position.z <= wall.min.z - 0.3);
}
