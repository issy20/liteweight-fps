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
fn recoil_accumulates_recovers_and_ads_reduces_spread() {
    let mut w = Weapon::default();
    let hip = w.spread(false);
    assert!(w.spread(true) > hip);
    w.fire();
    let first = w.recoil;
    assert!(first > 0.0);
    w.advance(0.1, true);
    w.fire();
    assert!(w.recoil > first);
    w.advance(2.0, true);
    assert_eq!(w.recoil, 0.0);
    assert_eq!(w.visual_recoil, 0.0);
    assert!(w.spread(false) < hip);
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
    let mut c = Combat {
        targets: vec![
            Target::new(Vec3::new(0., 0., -5.)),
            Target::new(Vec3::new(0., 0., -8.)),
        ],
        ..Combat::default()
    };
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
    assert_eq!(c.targets[0].hp, 100); // M3測定用標的は即時リセット
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
