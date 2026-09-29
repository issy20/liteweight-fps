use liteweight_fps::simulation::weapon::{FireInput, Loadout, WeaponKind};

#[test]
fn loadout_starts_with_ar_and_keeps_magazines_per_weapon() {
    let mut loadout = Loadout::default();
    assert_eq!(loadout.active_kind(), WeaponKind::AssaultRifle);
    assert!(loadout.fire(true, true));
    assert_eq!(loadout.active().ammo, 29);
    assert!(loadout.switch(WeaponKind::Pistol));
    assert_eq!(loadout.active().ammo, 7);
    assert!(loadout.fire(true, true));
    assert_eq!(loadout.active().ammo, 6);
    loadout.switch(WeaponKind::AssaultRifle);
    assert_eq!(loadout.active().ammo, 29);
}

#[test]
fn pistol_is_semiautomatic_even_while_trigger_is_held() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Pistol);
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
    assert!(loadout.active().ads > 0.0);
    assert!(loadout.active_mut().reload());
    loadout.advance(0.5, false);
    loadout.switch(WeaponKind::Pistol);
    assert_eq!(loadout.active().reload_remaining, 0.0);
    assert_eq!(loadout.active().ammo, 7);
    assert_eq!(loadout.active().ads, 0.0);
    loadout.switch(WeaponKind::AssaultRifle);
    assert_eq!(loadout.active().ammo, 29);
}

#[test]
fn sniper_reaches_high_precision_at_170ms_and_aims_at_200ms() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    let hip = loadout.active().spread(false);
    loadout.advance(0.17, true);
    assert!((loadout.active().ads - 0.85).abs() < 1e-5);
    let quick_scope = loadout.active().spread(false);
    assert!(quick_scope < hip * 0.1);
    loadout.advance(0.03, true);
    assert!((loadout.active().ads - 1.0).abs() < 1e-5);
    assert!(loadout.active().spread(false) <= quick_scope);
}

#[test]
fn sniper_scope_fov_moves_smoothly_from_70_to_32_degrees() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    assert_eq!(loadout.fov(), 70.);
    loadout.advance(0.17, true);
    assert!((loadout.fov() - 37.7).abs() < 1e-3);
    loadout.advance(0.03, true);
    assert!((loadout.fov() - 32.).abs() < 1e-5);
}

#[test]
fn sniper_bolt_blocks_next_shot_and_keeps_progress_when_switched_away() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
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
