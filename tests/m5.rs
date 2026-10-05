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
fn bot_render_mesh_is_a_faceless_mannequin_with_clear_head_accent() {
    let combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
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

    // Rendering detail must not expand or replace the torso/head hitboxes.
    assert_eq!(
        combat.targets[0].boxes()[0],
        Aabb::new(Vec3::new(-0.35, 0.0, -5.25), Vec3::new(0.35, 1.55, -4.75))
    );
    assert_eq!(
        combat.targets[0].boxes()[1],
        Aabb::new(Vec3::new(-0.25, 1.55, -5.25), Vec3::new(0.25, 2.05, -4.75))
    );
}

#[test]
fn mannequin_uses_short_neck_and_longer_legs() {
    let mesh =
        liteweight_fps::ui::world_mesh(&Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]));
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
fn ten_bot_render_mesh_stays_within_renderer_buffer_limits() {
    let bots = (0..10)
        .map(|slot| Bot::new(Vec3::new(slot as f32 * 3.0, 0.0, -5.0 - slot as f32)))
        .collect();
    let mesh = liteweight_fps::ui::world_mesh(&Combat::new(bots));
    assert!(mesh.vertices.len() <= 4096);
    assert!(
        mesh.indices.len() > 4096,
        "test should exercise a multi-BOT mesh"
    );
    assert!(mesh.indices.len() <= 16384);
}

#[test]
fn headshots_and_body_hits_update_independent_score_counts() {
    let mut combat = Combat::new(vec![Bot::new(Vec3::new(0.0, 0.0, -5.0))]);
    let body = combat.shoot(Vec3::new(0.0, 1.0, 0.0), -Vec3::Z, &[], 25);
    assert!(body.hit && !body.headshot);
    assert_eq!(combat.targets[0].hp, 75);

    let head = combat.shoot(Vec3::new(0.0, 1.8, 0.0), -Vec3::Z, &[], 25);
    assert!(head.hit && head.headshot);
    assert_eq!(combat.targets[0].hp, 25);
    assert_eq!(combat.hits, 2);
    assert_eq!(combat.headshots, 1);
    assert_eq!(combat.kills, 0);
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
