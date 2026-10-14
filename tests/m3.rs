use liteweight_fps::simulation::weapon::Weapon;

#[test]
fn automatic_fire_obeys_interval_and_magazine() {
    let mut w = Weapon::default();
    assert!(w.fire());
    assert_eq!(w.ammo, 29);
    assert!(!w.fire());
    w.advance(0.099, false);
    assert!(!w.fire());
    w.advance(0.001, false);
    assert!(w.fire());
    for _ in 0..28 {
        w.advance(0.1, false);
        assert!(w.fire());
    }
    w.advance(0.1, false);
    assert_eq!(w.ammo, 0);
    assert!(!w.fire());
}

#[test]
fn reload_blocks_fire_and_only_refills_on_completion() {
    let mut w = Weapon::default();
    assert!(!w.reload());
    assert!(w.fire());
    assert!(w.reload());
    assert!(!w.reload());
    w.advance(1.999, true);
    assert_eq!(w.ammo, 29);
    assert!(!w.fire());
    w.advance(0.001, false);
    assert_eq!(w.ammo, 30);
    assert!(w.fire());
}

#[test]
fn ads_interpolates_allows_fire_and_yields_to_reload() {
    let mut w = Weapon::default();
    w.advance(0.09, true);
    assert!((w.ads_progress - 0.5).abs() < 1e-5);
    assert!(w.fire());
    w.advance(0.09, true);
    assert!((w.ads_progress - 1.0).abs() < 1e-5);
    w.advance(0.045, false);
    assert!((w.ads_progress - 0.75).abs() < 1e-5);
    w.reload();
    w.advance(0.18, true);
    assert_eq!(w.ads_progress, 0.0);
}

#[test]
fn weapon_visual_recoil_returns_while_view_recoil_stays() {
    let mut w = Weapon::default();
    let mut player = liteweight_fps::simulation::Player::new();
    w.fire();
    assert!(w.visual_recoil > 0.0);
    player.apply_recoil(w.data.recoil_kick);
    let pitch_after_shot = player.pitch;
    w.advance(0.1, true);
    w.fire();
    w.advance(2.0, true);
    assert_eq!(player.pitch, pitch_after_shot);
    assert_eq!(w.visual_recoil, 0.0);
}

#[test]
fn weapon_sway_follows_movement_settles_when_still_and_is_damped_while_ads() {
    let mut moving = Weapon::default();
    let hip_base = moving.view_model_layout(0.0);
    let aim_direction = glam::Vec3::new(0.1, 0.2, -1.0).normalize();
    let shot_direction = moving.shot_direction(aim_direction);
    for _ in 0..30 {
        moving.advance_with_movement(1.0 / 60.0, false, 4.0);
    }
    let hip_moving = moving.view_model_layout(0.0);
    assert_ne!(hip_moving.center_x, hip_base.center_x);
    assert_ne!(hip_moving.base_y, hip_base.base_y);
    assert_eq!(moving.shot_direction(aim_direction), shot_direction);

    let mut still = Weapon::default();
    for _ in 0..90 {
        still.advance_with_movement(1.0 / 60.0, false, 0.0);
    }
    let settled = still.view_model_layout(0.0);
    assert!((settled.center_x - hip_base.center_x).abs() < 1e-5);
    assert!((settled.base_y - hip_base.base_y).abs() < 1e-5);

    let mut ads = Weapon::default();
    ads.ads_progress = 1.0;
    let ads_base = ads.view_model_layout(0.0);
    for _ in 0..30 {
        ads.advance_with_movement(1.0 / 60.0, true, 4.0);
    }
    let ads_moving = ads.view_model_layout(0.0);
    assert!(
        (ads_moving.center_x - ads_base.center_x).abs()
            < (hip_moving.center_x - hip_base.center_x).abs() * 0.2
    );
}

#[test]
fn ar_and_sniper_recoil_tuning_is_reduced_while_pistol_stays_unchanged() {
    use liteweight_fps::simulation::weapon::{Loadout, WeaponKind};

    let mut loadout = Loadout::default();
    assert_eq!(loadout.active().data.recoil_kick, 0.010);
    loadout.switch(WeaponKind::Sniper);
    assert_eq!(loadout.active().data.recoil_kick, 0.075);
    loadout.switch(WeaponKind::Pistol);
    assert_eq!(loadout.active().data.recoil_kick, 0.045);
}

#[test]
fn sustained_infinite_ammo_fire_preserves_cadence_after_many_shots() {
    let mut loadout = liteweight_fps::simulation::weapon::Loadout::default();
    assert!(loadout.fire_with_infinite_ammo(true, false, true));
    let kick = loadout.active().data.recoil_kick;
    assert!(kick > 0.0);

    for _ in 1..80 {
        for _ in 0..6 {
            loadout.advance(1.0 / 60.0, false);
        }
        assert!(loadout.fire_with_infinite_ammo(true, false, true));
    }
    assert!(loadout.active().visual_recoil > 0.0);
}

#[test]
fn ray_box_handles_parallel_inside_behind_and_distance() {
    use glam::Vec3;
    use liteweight_fps::physics::Aabb;
    let b = Aabb::new(Vec3::new(-1., -1., -6.), Vec3::new(1., 1., -4.));
    assert_eq!(b.ray_distance(Vec3::ZERO, -Vec3::Z), Some(4.));
    assert_eq!(b.ray_distance(Vec3::ZERO, Vec3::Z), None);
    assert_eq!(b.ray_distance(Vec3::X * 2., -Vec3::Z), None);
    assert_eq!(b.ray_distance(-Vec3::Z * 5., Vec3::X), Some(0.));
    assert_eq!(b.ray_distance(Vec3::ZERO, Vec3::ZERO), None);
}

#[test]
fn hitscan_stops_at_wall_and_applies_single_target_damage() {
    use glam::Vec3;
    use liteweight_fps::{
        physics::Aabb,
        simulation::combat::{Combat, Target},
    };
    let mut c = Combat::default();
    c.targets = vec![
        Target::new(Vec3::new(0., 0., -5.)),
        Target::new(Vec3::new(0., 0., -8.)),
    ];
    let origin = Vec3::new(0., 1., 0.);
    let wall = Aabb::new(Vec3::new(-2., 0., -3.), Vec3::new(2., 3., -2.));
    assert!(!c.shoot(origin, -Vec3::Z, &[wall], 25).hit);
    assert_eq!(c.targets[0].hp, 100);
    assert!(c.shoot(origin, -Vec3::Z, &[], 25).hit);
    assert_eq!(c.targets[0].hp, 75);
    assert_eq!(c.targets[1].hp, 100);
    let shot = c.shoot(Vec3::new(0., 1.8, 0.), -Vec3::Z, &[], 25);
    assert!(shot.headshot);
    assert_eq!(c.targets[0].hp, 25);
    assert!(c.shoot(origin, -Vec3::Z, &[], 25).killed);
    assert_eq!(c.targets[0].hp, 0);
    assert!(!c.targets[0].alive);
}

#[test]
fn short_click_is_latched_and_pause_clears_buttons() {
    use liteweight_fps::simulation::weapon::FireInput;
    let mut input = FireInput::default();
    input.set_trigger(true);
    input.set_trigger(false);
    assert!(input.take_trigger());
    assert!(!input.take_trigger());
    input.set_trigger(true);
    assert!(input.take_trigger());
    assert!(input.take_trigger());
    input.clear();
    assert!(!input.take_trigger());
}

#[test]
fn reload_completes_in_exactly_120_fixed_steps() {
    let mut w = Weapon::default();
    w.fire();
    w.reload();
    for _ in 0..120 {
        w.advance(1. / 60., false);
    }
    assert_eq!(w.ammo, 30);
    assert_eq!(w.reload_remaining, 0.);
}
