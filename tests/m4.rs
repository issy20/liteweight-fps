use glam::Vec3;
use liteweight_fps::simulation::weapon::{FireInput, Loadout, WeaponKind};

#[test]
fn loadout_starts_with_ar_and_keeps_magazines_per_weapon() {
    let mut loadout = Loadout::default();
    assert_eq!(loadout.active_kind(), WeaponKind::AssaultRifle);
    assert!(loadout.fire(true, true));
    assert_eq!(loadout.active().ammo, 29);
    assert!(loadout.switch(WeaponKind::Pistol));
    loadout.advance(0.32, false);
    assert_eq!(loadout.active().ammo, 7);
    assert!(loadout.fire(true, true));
    assert_eq!(loadout.active().ammo, 6);
    loadout.switch(WeaponKind::AssaultRifle);
    assert_eq!(loadout.active().ammo, 29);
}

#[test]
fn ar_and_pistol_ads_use_a_less_magnified_view_than_the_initial_tuning() {
    assert_eq!(WeaponKind::AssaultRifle.fov(1.0), 65.0);
    assert_eq!(WeaponKind::Pistol.fov(1.0), 67.0);
    assert_eq!(WeaponKind::Sniper.fov(1.0), 32.0);
}

#[test]
fn ballistic_muzzle_projects_to_the_visible_weapon_muzzle_in_hip_and_ads() {
    let aspect = 16.0 / 9.0;
    let player = liteweight_fps::simulation::Player::new();
    let view = player.aim_direction();
    let mut loadout = Loadout::default();
    for kind in [
        WeaponKind::AssaultRifle,
        WeaponKind::Pistol,
        WeaponKind::Sniper,
    ] {
        loadout.switch(kind);
        loadout.advance(0.32, false);
        for progress in [0.0, 0.5, 1.0] {
            loadout.active_mut().ads_progress = progress;
            let weapon = loadout.active();
            let fov = kind.fov_with_base(90.0, progress);
            let layout = weapon.view_model_layout(0.0);
            let muzzle = weapon.muzzle_position(player.position, view, aspect, fov, 0.0);
            let projected = player.view_matrix_with(aspect, fov).project_point3(muzzle);
            assert!((projected.x - layout.muzzle_screen.x / aspect).abs() < 1e-5);
            assert!((projected.y - layout.muzzle_screen.y).abs() < 1e-5);
        }
    }
}

#[test]
fn rendered_muzzle_tip_uses_the_same_anchor_as_the_shot_origin() {
    let aspect = 16.0 / 9.0;
    let mut loadout = Loadout::default();
    let combat = liteweight_fps::simulation::combat::Combat::default();
    for kind in [
        WeaponKind::AssaultRifle,
        WeaponKind::Pistol,
        WeaponKind::Sniper,
    ] {
        loadout.switch(kind);
        loadout.advance(0.32, false);
        for progress in [0.0, 0.6, 1.0] {
            loadout.active_mut().ads_progress = progress;
            let weapon = loadout.active();
            let layout = weapon.view_model_layout(0.0);
            let mesh = liteweight_fps::ui::hud_mesh(weapon, &combat, aspect, 0.0);
            assert!(mesh.vertices.iter().any(|vertex| {
                (vertex.position[0] - layout.muzzle_screen.x / aspect).abs() < 1e-6
                    && (vertex.position[1] - layout.muzzle_screen.y).abs() < 1e-6
            }));
        }
    }
}

#[test]
fn weapon_models_use_a_compact_mesh_and_keep_the_muzzle_anchor() {
    let aspect = 16.0 / 9.0;
    let mut loadout = Loadout::default();
    let combat = liteweight_fps::simulation::combat::Combat::default();
    for kind in [
        WeaponKind::AssaultRifle,
        WeaponKind::Pistol,
        WeaponKind::Sniper,
    ] {
        loadout.switch(kind);
        loadout.advance(0.32, false);
        let mesh = liteweight_fps::ui::hud_mesh(loadout.active(), &combat, aspect, 0.0);
        assert!(
            mesh.vertices.len() <= 100,
            "{kind:?} has {} vertices",
            mesh.vertices.len()
        );
        let muzzle = loadout.active().view_model_layout(0.0).muzzle_screen;
        assert!(mesh.vertices.iter().any(|vertex| {
            (vertex.position[0] - muzzle.x / aspect).abs() < 1e-6
                && (vertex.position[1] - muzzle.y).abs() < 1e-6
        }));
        {
            let layout = loadout.active().view_model_layout(0.0);
            let maximum_part_height = if kind == WeaponKind::Pistol {
                0.75
            } else {
                0.51
            };
            let connected_lower_part = mesh.vertices.iter().any(|vertex| {
                vertex.color == [0.10, 0.12, 0.14, 1.0]
                    && (vertex.position[0] - layout.center_x / aspect).abs() < 0.04
                    && vertex.position[1] > layout.base_y + layout.body_height * 0.4
                    && vertex.position[1] < layout.base_y + layout.body_height * maximum_part_height
            });
            match kind {
                WeaponKind::Pistol => {
                    assert!(connected_lower_part, "pistol grip should meet the frame")
                }
                WeaponKind::AssaultRifle => {
                    assert!(connected_lower_part, "AR magazine should meet the receiver")
                }
                WeaponKind::Sniper => assert!(
                    connected_lower_part,
                    "sniper stock should meet the receiver"
                ),
            }
        }
    }
}

#[test]
fn standard_reticle_is_hidden_while_ads_is_active() {
    let mut loadout = Loadout::default();
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let mut has_center_reticle = |progress| {
        loadout.active_mut().ads_progress = progress;
        let mesh = liteweight_fps::ui::hud_mesh(loadout.active(), &combat, 16.0 / 9.0, 0.0);
        mesh.vertices.iter().any(|vertex| {
            vertex.color == [0.9, 0.9, 0.9, 1.0]
                && vertex.position[0].abs() < 0.02
                && vertex.position[1].abs() < 0.02
        })
    };
    assert!(has_center_reticle(0.0));
    assert!(!has_center_reticle(0.5));
    assert!(!has_center_reticle(1.0));
}

#[test]
fn sniper_ads_fov_change_does_not_move_or_pitch_the_player_camera() {
    let mut player = liteweight_fps::simulation::Player::new();
    player.look(0.3, 0.2);
    let wide = player.view_matrix_with(16.0 / 9.0, 70.0);
    let scoped = player.view_matrix_with(16.0 / 9.0, 32.0);
    let world_ray = |matrix: glam::Mat4| {
        (matrix.inverse().project_point3(Vec3::new(0.0, 0.0, 0.5)) - player.position).normalize()
    };
    assert!(world_ray(wide).dot(world_ray(scoped)) > 0.99999);
    assert_eq!(player.position, Vec3::new(0.0, 1.6, 0.0));
}

#[test]
fn hip_fire_has_reproducible_shot_to_shot_spread_while_sniper_stays_deterministic() {
    let mut loadout = Loadout::default();
    let view = -Vec3::Z;
    let ar = loadout.active();
    let first = ar.spread_direction(view, false, 0);
    let second = ar.spread_direction(view, false, 1);
    assert_ne!(first, second);
    assert_eq!(first, ar.spread_direction(view, false, 0));

    loadout.switch(WeaponKind::Sniper);
    let sniper = loadout.active();
    assert_eq!(
        sniper.spread_direction(view, false, 0),
        sniper.spread_direction(view, false, 1)
    );
}

#[test]
fn pistol_is_semiautomatic_even_while_trigger_is_held() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Pistol);
    loadout.advance(0.32, false);
    assert!(loadout.fire(true, true));
    assert!(!loadout.fire(true, false));
    for _ in 0..20 {
        loadout.advance(1. / 60., false);
    }
    assert!(!loadout.fire(true, false));
    assert!(loadout.fire(true, true));
    assert_eq!(loadout.active().ammo, 5);
}

#[test]
fn pistol_obeys_180_rpm_interval_between_clicks() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Pistol);
    loadout.advance(0.32, false);
    assert!(loadout.fire(false, true));
    loadout.advance(1. / 3. - 0.001, false);
    assert!(!loadout.fire(false, true));
    loadout.advance(0.001, false);
    assert!(loadout.fire(false, true));
}

#[test]
fn switching_cancels_reload_but_keeps_loaded_ammo_and_clears_ads() {
    let mut loadout = Loadout::default();
    loadout.fire(true, true);
    loadout.advance(0.2, true);
    assert!(loadout.active().ads_progress > 0.0);
    assert!(loadout.active_mut().reload());
    loadout.advance(0.5, false);
    loadout.switch(WeaponKind::Pistol);
    assert_eq!(loadout.active().reload_remaining, 0.0);
    assert_eq!(loadout.active().ammo, 7);
    assert_eq!(loadout.active().ads_progress, 0.0);
    loadout.switch(WeaponKind::AssaultRifle);
    assert_eq!(loadout.active().ammo, 29);
}

#[test]
fn sniper_ads_progress_moves_in_and_out_smoothly_without_spread_accuracy_curve() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let hip = loadout.active().spread(false);
    loadout.advance(0.175, true);
    assert!((loadout.active().ads_progress - 0.5).abs() < 1e-5);
    assert_eq!(loadout.active().spread(false), hip);
    loadout.advance(0.1, false);
    assert!(loadout.active().ads_progress < 0.5);
    loadout.advance(1.0, false);
    assert_eq!(loadout.active().ads_progress, 0.0);
}

#[test]
fn sniper_shot_direction_converges_to_view_during_ads() {
    use glam::Vec3;

    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let view = Vec3::new(0.0, 0.0, -1.0);
    let hip = loadout.active().shot_direction(view);
    loadout.advance(0.175, true);
    let halfway = loadout.active().shot_direction(view);
    assert!(loadout.fire(true, true));
    loadout.advance(0.175, true);
    let fully_aimed = loadout.active().shot_direction(view);

    assert!((hip - view).length() > 0.01);
    assert!((halfway - view).length() < (hip - view).length());
    assert!((fully_aimed - view).length() < 1e-6);
}

#[test]
fn sniper_scope_fov_moves_smoothly_from_70_to_32_degrees_over_350ms() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    assert_eq!(loadout.fov(), 70.);
    loadout.advance(0.2975, true);
    assert!((loadout.fov() - 37.7).abs() < 1e-3);
    loadout.advance(0.0525, true);
    assert!((loadout.fov() - 32.).abs() < 1e-5);
}

#[test]
fn sniper_bolt_blocks_next_shot_and_keeps_progress_when_switched_away() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    assert!(loadout.fire(true, true));
    assert!(!loadout.fire(true, true));
    loadout.advance(0.6, false);
    let remaining = loadout.active().bolt_remaining;
    assert!(remaining > 0.0);
    loadout.switch(WeaponKind::Pistol);
    loadout.advance(0.3, false);
    loadout.switch(WeaponKind::Sniper);
    assert!(loadout.active().bolt_remaining < remaining);
    assert!(!loadout.fire(true, true));
    loadout.advance(0.4, false);
    assert!(loadout.active().bolt_remaining == 0.0);
    assert!(loadout.fire(true, true));
}

#[test]
fn fire_input_preserves_press_edge_for_semiautomatic_weapon() {
    let mut input = FireInput::default();
    input.set_trigger(true);
    input.set_trigger(false);
    assert_eq!(input.take_state(), (false, true));
    assert_eq!(input.take_state(), (false, false));
}

#[test]
fn switching_has_a_drawn_transition_and_temporarily_blocks_fire() {
    let mut loadout = Loadout::default();
    assert!(loadout.switch(WeaponKind::Pistol));
    assert!(loadout.is_switching());
    assert_eq!(loadout.switch_remaining(), 0.32);
    loadout.advance(0.05, false);
    let (displayed, offset) = loadout.presentation();
    assert_eq!(displayed.kind, WeaponKind::AssaultRifle);
    assert!(offset < 0.0);
    assert!(!loadout.fire(true, true));
    loadout.advance(0.27, false);
    assert!(!loadout.is_switching());
    assert_eq!(loadout.presentation().0.kind, WeaponKind::Pistol);
    assert!(loadout.fire(true, true));
}

#[test]
fn weapon_models_are_distinct_and_scope_view_transitions_without_hiding_the_rifle() {
    use liteweight_fps::{simulation::combat::Combat, ui::hud_mesh};
    let mut loadout = Loadout::default();
    loadout.advance(0.32, false);
    let ar = hud_mesh(loadout.active(), &Combat::default(), 16. / 9., 0.);
    loadout.switch(WeaponKind::Pistol);
    loadout.advance(0.32, false);
    let pistol = hud_mesh(loadout.active(), &Combat::default(), 16. / 9., 0.);
    assert_ne!(ar.vertices[0].position, pistol.vertices[0].position);
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let hip = hud_mesh(loadout.active(), &Combat::default(), 16. / 9., 0.);
    loadout.active_mut().ads_progress = 0.3;
    let partial = hud_mesh(loadout.active(), &Combat::default(), 16. / 9., 0.);
    loadout.active_mut().ads_progress = 0.5;
    let middle = hud_mesh(loadout.active(), &Combat::default(), 16. / 9., 0.);
    loadout.active_mut().ads_progress = 1.0;
    let fully_aimed = hud_mesh(loadout.active(), &Combat::default(), 16. / 9., 0.);
    assert!(partial.vertices.len() > hip.vertices.len());
    assert!(middle.vertices.len() > partial.vertices.len());
    assert_eq!(fully_aimed.vertices.len(), partial.vertices.len());
}
