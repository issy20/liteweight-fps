use liteweight_fps::engine::text::FontAtlas;
use liteweight_fps::persistence::GameSettings;
use liteweight_fps::simulation::{
    game_mode::{SessionMode, TrainingSession, CHALLENGE_SECONDS},
    weapon::WeaponKind,
};
use liteweight_fps::ui::menu::{PauseMenuAction, PauseSelection, TitleMenuAction, TitleSelection};

#[test]
fn settings_restore_defaults_for_corrupt_or_out_of_range_values() {
    let corrupt = GameSettings::from_json("not json");
    assert_eq!(corrupt, GameSettings::default());

    let out_of_range =
        GameSettings::from_json(r#"{"mouse_sensitivity":99.0,"ads_sensitivity":-1.0,"fov":200.0}"#);
    assert_eq!(out_of_range, GameSettings::default());
    let invalid_bot_config =
        GameSettings::from_json(r#"{"bot_count":20,"bot_behavior":9,"volume":2.0}"#);
    assert_eq!(invalid_bot_config, GameSettings::default());
}

#[test]
fn settings_round_trip_and_partial_files_keep_safe_defaults() {
    let settings = GameSettings {
        mouse_sensitivity: 1.4,
        ads_sensitivity: 0.5,
        fov: 95.0,
        infinite_ammo: true,
        fullscreen: true,
        ..GameSettings::default()
    };
    let json = serde_json::to_string(&settings).unwrap();
    assert_eq!(GameSettings::from_json(&json), settings);

    let partial = GameSettings::from_json(r#"{"mouse_sensitivity":1.25}"#);
    assert_eq!(partial.mouse_sensitivity, 1.25);
    assert_eq!(
        partial.ads_sensitivity,
        GameSettings::default().ads_sensitivity
    );
    assert_eq!(partial.fov, GameSettings::default().fov);
    assert!(!GameSettings::from_json(r#"{"mouse_sensitivity":1.25}"#).fullscreen);
}

#[test]
fn look_sensitivity_blends_from_normal_to_ads_multiplier() {
    let settings = GameSettings {
        mouse_sensitivity: 1.5,
        ads_sensitivity: 0.4,
        ..GameSettings::default()
    };
    assert!((settings.look_multiplier(0.0) - 1.5).abs() < f32::EPSILON);
    assert!((settings.look_multiplier(1.0) - 0.6).abs() < f32::EPSILON);
    assert!((settings.look_multiplier(0.5) - 1.05).abs() < f32::EPSILON);
    assert_eq!(settings.look_multiplier(2.0), settings.look_multiplier(1.0));
}

#[test]
fn sensitivity_adjustment_changes_selected_value_and_respects_limits() {
    let mut settings = GameSettings::default();
    settings.adjust_sensitivity(0, 1.0);
    settings.adjust_sensitivity(1, -1.0);
    assert!((settings.mouse_sensitivity - 1.1).abs() < f32::EPSILON);
    assert!((settings.ads_sensitivity - 0.6).abs() < f32::EPSILON);

    settings.mouse_sensitivity = 5.0;
    settings.ads_sensitivity = 0.1;
    settings.adjust_sensitivity(0, 1.0);
    settings.adjust_sensitivity(1, -1.0);
    assert_eq!(settings.mouse_sensitivity, 5.0);
    assert_eq!(settings.ads_sensitivity, 0.1);
    settings.ads_sensitivity = 1.0;
    settings.adjust_sensitivity(1, 1.0);
    assert_eq!(settings.ads_sensitivity, 1.0);
}

#[test]
fn fov_setting_adjusts_by_one_degree_and_stays_within_supported_range() {
    let mut settings = GameSettings::default();
    settings.adjust_setting(2, 1.0);
    assert_eq!(settings.fov, 71.0);
    settings.fov = 120.0;
    settings.adjust_setting(2, 1.0);
    assert_eq!(settings.fov, 120.0);
    settings.fov = 60.0;
    settings.adjust_setting(2, -1.0);
    assert_eq!(settings.fov, 60.0);
}

#[test]
fn bot_count_behavior_and_volume_settings_are_bounded_and_adjustable() {
    let mut settings = GameSettings::default();
    assert_eq!(settings.bot_count, 5);
    assert_eq!(settings.bot_behavior, 3);
    assert_eq!(settings.volume, 0.7);
    settings.adjust_setting(3, 1.0);
    settings.adjust_setting(4, -1.0);
    settings.adjust_setting(5, -1.0);
    assert_eq!(settings.bot_count, 6);
    assert_eq!(settings.bot_behavior, 2);
    assert!((settings.volume - 0.65).abs() < f32::EPSILON);

    settings.bot_count = 10;
    settings.adjust_setting(3, 1.0);
    settings.volume = 1.0;
    settings.adjust_setting(5, 1.0);
    assert_eq!(settings.bot_count, 10);
    assert_eq!(settings.volume, 1.0);
}

#[test]
fn bot_hp_speed_and_respawn_settings_adjust_with_safe_limits() {
    let mut settings = GameSettings::default();
    settings.adjust_setting(6, 1.0);
    settings.adjust_setting(7, 1.0);
    settings.adjust_setting(8, -1.0);
    assert_eq!(settings.bot_hp, 110);
    assert!((settings.bot_speed - 1.1).abs() < f32::EPSILON);
    assert_eq!(settings.bot_respawn_seconds, 1.75);
    settings.bot_hp = 200;
    settings.bot_speed = 2.0;
    settings.bot_respawn_seconds = 0.5;
    settings.adjust_setting(6, 1.0);
    settings.adjust_setting(7, 1.0);
    settings.adjust_setting(8, -1.0);
    assert_eq!(settings.bot_hp, 200);
    assert_eq!(settings.bot_speed, 2.0);
    assert_eq!(settings.bot_respawn_seconds, 0.5);
}

#[test]
fn bot_configuration_applies_count_and_selected_behavior_pattern() {
    use liteweight_fps::simulation::bot::BotBehavior;
    use liteweight_fps::simulation::combat::Combat;
    let mut combat = Combat::default();
    combat.configure_bots_with(8, 1, 125, 1.4, 3.5);
    assert_eq!(combat.targets.len(), 8);
    assert!(combat
        .targets
        .iter()
        .all(|bot| bot.behavior == BotBehavior::Strafe));
    assert!(combat
        .targets
        .iter()
        .all(|bot| { bot.hp == 125 && bot.speed_multiplier == 1.4 && bot.respawn_seconds == 3.5 }));
    combat.configure_bots(99, 2);
    assert_eq!(combat.targets.len(), 10);
    assert!(combat
        .targets
        .iter()
        .all(|bot| bot.behavior == BotBehavior::Chase));
}

#[test]
fn configured_fov_applies_to_hip_view_and_blends_into_weapon_ads_fov() {
    let settings = GameSettings {
        fov: 90.0,
        ..GameSettings::default()
    };
    let loadout = liteweight_fps::simulation::weapon::Loadout::default();
    assert_eq!(loadout.active_kind().fov_with_base(settings.fov, 0.0), 90.0);
    assert_eq!(loadout.active_kind().fov_with_base(settings.fov, 1.0), 65.0);
    assert_eq!(loadout.active_kind().fov_with_base(settings.fov, 0.5), 77.5);
}

#[test]
fn font_text_layout_uses_proportional_glyphs_and_valid_quad_indices() {
    let font = FontAtlas::embedded();
    let narrow = font.layout("III", [20.0, 20.0], 32.0, [1280, 720], [1.0; 4]);
    let wide = font.layout("WWW", [20.0, 20.0], 32.0, [1280, 720], [1.0; 4]);
    assert!(narrow.width_px < wide.width_px);
    assert_eq!(narrow.mesh.vertices.len(), 12);
    assert_eq!(narrow.mesh.indices.len(), 18);
    assert!(narrow
        .mesh
        .indices
        .iter()
        .all(|index| usize::from(*index) < 12));
    assert!(font.pixels().iter().any(|coverage| *coverage > 0));
    assert!(font
        .pixels()
        .iter()
        .any(|coverage| *coverage > 0 && *coverage < 255));
}

#[test]
fn settings_text_uses_smooth_font_mesh_inside_clip_bounds() {
    let mesh = liteweight_fps::ui::settings_text(&GameSettings::default(), 0, [1280, 720]);
    assert!(!mesh.vertices.is_empty());
    assert!(mesh.vertices.len() < 4096);
    assert!(mesh.indices.len() < 6144);
    assert!(mesh.vertices.iter().all(|vertex| vertex
        .position
        .iter()
        .all(|coordinate| (-1.0..=1.0).contains(coordinate))));
    assert!(mesh
        .indices
        .iter()
        .all(|index| usize::from(*index) < mesh.vertices.len()));
}

#[test]
fn gameplay_hud_text_fits_common_internal_resolutions_for_both_modes() {
    use liteweight_fps::simulation::game_mode::SessionMode;
    use liteweight_fps::simulation::weapon::WeaponKind;
    for viewport in [[1280, 720], [1920, 1080], [960, 540]] {
        for mode in [SessionMode::FreeTraining, SessionMode::FiveMinuteChallenge] {
            let mesh = liteweight_fps::ui::game_hud_text(
                liteweight_fps::ui::GameHudInfo {
                    mode,
                    remaining: Some(299.0),
                    weapon: WeaponKind::AssaultRifle,
                    ammo: 30,
                    magazine: 30,
                    kills: 12,
                    infinite_ammo: false,
                },
                viewport,
            );
            assert!(!mesh.vertices.is_empty());
            assert!(mesh.vertices.iter().all(|vertex| vertex
                .position
                .iter()
                .all(|coordinate| (-1.0..=1.0).contains(coordinate))));
        }
    }
}

#[test]
fn pause_menu_selects_resume_sensitivity_and_exit_actions() {
    let mut menu = PauseSelection::default();
    assert_eq!(menu.activate(), Some(PauseMenuAction::Resume));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(0));
    assert_eq!(menu.activate(), None);
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(1));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(2));
    menu.move_down();
    assert_eq!(menu.toggle_setting_index(), Some(0));
    menu.move_down();
    assert_eq!(menu.toggle_setting_index(), Some(1));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(5));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(3));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(4));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(6));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(7));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(8));
    menu.move_down();
    assert_eq!(menu.activate(), Some(PauseMenuAction::Exit));
    menu.move_down();
    assert_eq!(menu.activate(), Some(PauseMenuAction::Resume));
}

#[test]
fn fullscreen_setting_is_a_toggle_menu_item_and_is_shown_in_settings() {
    let mut menu = PauseSelection::default();
    for _ in 0..5 {
        menu.move_down();
    }
    assert_eq!(menu.toggle_setting_index(), Some(1));
    assert_eq!(menu.selected_index(), 5);

    let settings = GameSettings {
        fullscreen: true,
        ..GameSettings::default()
    };
    let text = liteweight_fps::ui::settings_text(&settings, 5, [1280, 720]);
    assert!(!text.vertices.is_empty());
}

#[test]
fn challenge_timer_clamps_at_zero_and_completes_only_once() {
    let mut session = TrainingSession::new(SessionMode::FiveMinuteChallenge);
    assert_eq!(session.remaining_seconds(), Some(CHALLENGE_SECONDS));
    assert!(!session.advance(299.5));
    assert_eq!(session.remaining_seconds(), Some(0.5));
    assert!(session.advance(2.0));
    assert_eq!(session.remaining_seconds(), Some(0.0));
    assert!(!session.advance(1.0));
}

#[test]
fn session_results_track_rates_and_weapon_breakdown_with_zero_safe_rates() {
    let mut session = TrainingSession::new(SessionMode::FiveMinuteChallenge);
    assert_eq!(session.stats().accuracy(), 0.0);
    assert_eq!(session.stats().headshot_rate(), 0.0);

    session.record_shot(WeaponKind::AssaultRifle, true, false, false);
    session.record_shot(WeaponKind::AssaultRifle, true, true, true);
    session.record_shot(WeaponKind::Sniper, false, false, false);

    let stats = session.stats();
    assert_eq!(
        (stats.shots, stats.hits, stats.headshots, stats.kills),
        (3, 2, 1, 1)
    );
    assert_eq!(stats.accuracy(), 2.0 / 3.0);
    assert_eq!(stats.headshot_rate(), 0.5);
    assert_eq!(stats.for_weapon(WeaponKind::AssaultRifle).shots, 2);
    assert_eq!(stats.for_weapon(WeaponKind::Sniper).shots, 1);
    assert_eq!(stats.for_weapon(WeaponKind::Pistol).shots, 0);
}

#[test]
fn challenge_result_history_round_trips_and_keeps_newest_entries() {
    let path = std::env::temp_dir().join(format!(
        "liteweight-fps-results-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    for timestamp in 1..=55 {
        liteweight_fps::persistence::save_result_to(
            &path,
            liteweight_fps::persistence::ResultRecord {
                recorded_at_unix_seconds: timestamp,
                stats: Default::default(),
            },
        )
        .unwrap();
    }
    let records = liteweight_fps::persistence::load_results_from(&path).unwrap();
    assert_eq!(records.len(), 50);
    assert_eq!(records.first().unwrap().recorded_at_unix_seconds, 6);
    assert_eq!(records.last().unwrap().recorded_at_unix_seconds, 55);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn free_training_has_no_countdown_or_automatic_completion() {
    let mut session = TrainingSession::new(SessionMode::FreeTraining);
    assert_eq!(session.remaining_seconds(), None);
    assert!(!session.advance(10_000.0));
    assert!(!session.is_complete());
}

#[test]
fn infinite_ammo_setting_is_ignored_during_the_five_minute_challenge() {
    use liteweight_fps::simulation::game_mode::SessionMode;
    assert!(SessionMode::FreeTraining.infinite_ammo_enabled(true));
    assert!(!SessionMode::FreeTraining.infinite_ammo_enabled(false));
    assert!(!SessionMode::FiveMinuteChallenge.infinite_ammo_enabled(true));
}

#[test]
fn title_menu_selects_both_training_modes_settings_and_quit() {
    let mut menu = TitleSelection::default();
    assert_eq!(menu.activate(), TitleMenuAction::FreeTraining);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::FiveMinuteChallenge);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::Settings);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::Quit);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::FreeTraining);
}

#[test]
fn free_training_infinite_ammo_keeps_magazine_full_without_bypassing_fire_cadence() {
    let mut loadout = liteweight_fps::simulation::weapon::Loadout::default();
    let magazine = loadout.active().data.magazine;
    assert!(loadout.fire_with_infinite_ammo(true, false, true));
    assert_eq!(loadout.active().ammo, magazine);
    assert!(!loadout.fire_with_infinite_ammo(true, false, true));

    let mut fired = 1;
    for _ in 0..40 {
        loadout.advance(0.1, false);
        fired += u32::from(loadout.fire_with_infinite_ammo(true, false, true));
    }
    assert_eq!(fired, 41);
    assert_eq!(loadout.active().ammo, magazine);
}

#[test]
fn enemy_hit_marker_is_red_and_more_prominent_than_the_reticle() {
    let weapon = liteweight_fps::simulation::weapon::Weapon::default();
    let mut combat = liteweight_fps::simulation::combat::Combat::new(Vec::new());
    combat.feedback = 0.2;
    combat.last_shot = liteweight_fps::simulation::combat::Shot {
        hit: true,
        ..Default::default()
    };
    let aspect = 16.0 / 9.0;
    let mesh = liteweight_fps::ui::hud_mesh(&weapon, &combat, aspect, 0.0);
    let markers = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.color == [1.0, 0.3, 0.24, 1.0])
        .collect::<Vec<_>>();
    assert_eq!(markers.len(), 16, "four red strokes use four vertices each");
    let min_x = markers
        .iter()
        .map(|vertex| vertex.position[0] * aspect)
        .fold(f32::INFINITY, f32::min);
    let max_x = markers
        .iter()
        .map(|vertex| vertex.position[0] * aspect)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((max_x - min_x - 0.0685).abs() < 1e-4);
    let (strokes, remainder) = markers.as_slice().as_chunks::<4>();
    assert!(remainder.is_empty());
    for stroke in strokes {
        let dx = (stroke[1].position[0] - stroke[0].position[0]) * aspect;
        let dy = stroke[1].position[1] - stroke[0].position[1];
        assert!(
            (dx.abs() - dy.abs()).abs() < 1e-5,
            "hit mark should use diagonal lines"
        );
    }
}

#[test]
fn kill_hit_marker_keeps_the_larger_heavier_red_strokes() {
    let weapon = liteweight_fps::simulation::weapon::Weapon::default();
    let mut combat = liteweight_fps::simulation::combat::Combat::new(Vec::new());
    combat.feedback = 0.2;
    combat.last_shot = liteweight_fps::simulation::combat::Shot {
        hit: true,
        killed: true,
        ..Default::default()
    };
    let aspect = 16.0 / 9.0;
    let mesh = liteweight_fps::ui::hud_mesh(&weapon, &combat, aspect, 0.0);
    let markers = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.color == [1.0, 0.05, 0.06, 1.0])
        .collect::<Vec<_>>();
    assert_eq!(markers.len(), 16);
    let min_x = markers
        .iter()
        .map(|vertex| vertex.position[0] * aspect)
        .fold(f32::INFINITY, f32::min);
    let max_x = markers
        .iter()
        .map(|vertex| vertex.position[0] * aspect)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!((max_x - min_x - 0.097).abs() < 1e-4);
}
