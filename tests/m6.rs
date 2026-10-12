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
        GameSettings::from_json(r#"{"bot_count":99,"bot_behavior":9,"volume":2.0}"#);
    assert_eq!(invalid_bot_config, GameSettings::default());
    assert_eq!(GameSettings::from_json(r#"{"bot_count":20}"#).bot_count, 20);
}

#[test]
fn missing_control_and_display_settings_are_adjustable_and_persisted() {
    use liteweight_fps::persistence::{BindingAction, BindingKey};

    let mut settings = GameSettings::default();
    assert!(!settings.invert_y);
    assert_eq!(settings.look_y_sign(), 1.0);
    assert!(settings.vsync);
    assert_eq!(settings.resolution_size(), (960, 540));
    assert_eq!(settings.render_scale(), 0.67);
    assert_eq!(settings.fps_limit(), None);
    assert_eq!(
        settings.key_bindings.get(BindingAction::Forward),
        BindingKey::W
    );

    settings.toggle_setting(2);
    assert!(settings.invert_y);
    assert_eq!(settings.look_y_sign(), -1.0);
    settings.toggle_setting(3);
    assert!(!settings.vsync);
    settings.adjust_setting(9, 1.0);
    settings.adjust_setting(10, 1.0);
    settings.adjust_setting(11, -1.0);
    assert_eq!(settings.resolution_size(), (1280, 720));
    assert_eq!(settings.render_scale(), 0.75);
    assert_eq!(settings.fps_limit(), Some(240));
    assert!(settings
        .key_bindings
        .set(BindingAction::Forward, BindingKey::E));
    assert!(!settings
        .key_bindings
        .set(BindingAction::Backward, BindingKey::E));

    let restored = GameSettings::from_json(&serde_json::to_string(&settings).unwrap());
    assert_eq!(restored, settings);
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

    settings.bot_count = 20;
    settings.adjust_setting(3, 1.0);
    settings.volume = 1.0;
    settings.adjust_setting(5, 1.0);
    assert_eq!(settings.bot_count, 20);
    assert_eq!(settings.volume, 1.0);
}

#[test]
fn twenty_bots_can_be_configured_without_overlapping_initial_spawns() {
    let mut combat = liteweight_fps::simulation::combat::Combat::default();
    combat.configure_bots_with_arena_scale(20, 3, 100, 1.0, 2.0, 2.2);
    assert_eq!(combat.targets.len(), 20);
    assert!(combat.targets.iter().all(|bot| bot.arena_scale == 2.2));
    for (index, bot) in combat.targets.iter().enumerate() {
        for other in &combat.targets[index + 1..] {
            assert!(bot.position.distance(other.position) > 1.0);
        }
    }
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
    assert_eq!(combat.targets.len(), 20);
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
    for viewport in [[1280, 720], [960, 540], [640, 360]] {
        for selected in [0, 9, 18] {
            let mesh = liteweight_fps::ui::settings_text_with_bindings(
                &GameSettings::default(),
                selected,
                viewport,
                false,
                5,
                true,
            );
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
    }
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
                    key_bindings: GameSettings::default().key_bindings,
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
fn infinite_ammo_hud_renders_infinity_and_hides_magazine_digits() {
    use liteweight_fps::{
        engine::text::FontAtlas,
        simulation::{
            combat::Combat,
            game_mode::SessionMode,
            weapon::{Loadout, WeaponKind},
        },
        ui::{game_hud_text, GameHudInfo},
    };

    let infinity = FontAtlas::shared().layout("∞", [0.0, 0.0], 24.0, [1280, 720], [1.0; 4]);
    assert_eq!(infinity.mesh.vertices.len(), 4);

    let info = |infinite_ammo| GameHudInfo {
        mode: SessionMode::FreeTraining,
        remaining: None,
        weapon: WeaponKind::AssaultRifle,
        ammo: 30,
        magazine: 30,
        kills: 0,
        infinite_ammo,
        key_bindings: GameSettings::default().key_bindings,
    };
    let finite_text = game_hud_text(info(false), [1280, 720]);
    let infinite_text = game_hud_text(info(true), [1280, 720]);
    assert_eq!(
        finite_text.vertices.len(),
        infinite_text.vertices.len() + 16
    );

    let weapon = Loadout::default();
    let combat = Combat::default();
    let finite = liteweight_fps::ui::hud_mesh_with_menu_and_ammo(
        weapon.active(),
        &combat,
        16.0 / 9.0,
        0.0,
        None,
        false,
    );
    let infinite = liteweight_fps::ui::hud_mesh_with_menu_and_ammo(
        weapon.active(),
        &combat,
        16.0 / 9.0,
        0.0,
        None,
        true,
    );
    assert!(infinite.vertices.len() < finite.vertices.len());
}

#[test]
fn hud_mesh_builder_reuses_cpu_buffer_capacity() {
    use liteweight_fps::simulation::{combat::Combat, weapon::Loadout};

    let loadout = Loadout::default();
    let combat = Combat::default();
    let mut scratch = liteweight_fps::Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    liteweight_fps::ui::hud_mesh_with_menu_and_ammo_into(
        &mut scratch,
        loadout.active(),
        &combat,
        16.0 / 9.0,
        0.0,
        None,
        false,
    );
    let vertex_capacity = scratch.vertices.capacity();
    let index_capacity = scratch.indices.capacity();
    let expected = liteweight_fps::ui::hud_mesh_with_menu_and_ammo(
        loadout.active(),
        &combat,
        16.0 / 9.0,
        0.0,
        None,
        false,
    );
    liteweight_fps::ui::hud_mesh_with_menu_and_ammo_into(
        &mut scratch,
        loadout.active(),
        &combat,
        16.0 / 9.0,
        0.0,
        None,
        false,
    );

    assert_eq!(scratch.vertices, expected.vertices);
    assert_eq!(scratch.indices, expected.indices);
    assert_eq!(scratch.vertices.capacity(), vertex_capacity);
    assert_eq!(scratch.indices.capacity(), index_capacity);
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
    assert_eq!(menu.toggle_setting_index(), Some(2));
    menu.move_down();
    assert_eq!(menu.selected_index(), 5);
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(9));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(10));
    menu.move_down();
    assert_eq!(menu.toggle_setting_index(), Some(1));
    menu.move_down();
    assert_eq!(menu.toggle_setting_index(), Some(3));
    menu.move_down();
    assert_eq!(menu.setting_index(), Some(11));
    menu.move_down();
    assert_eq!(menu.toggle_setting_index(), Some(0));
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
    assert_eq!(menu.activate(), Some(PauseMenuAction::ChangeMode));
    menu.move_down();
    assert_eq!(menu.activate(), Some(PauseMenuAction::Exit));
    menu.move_down();
    assert_eq!(menu.activate(), Some(PauseMenuAction::Resume));
}

#[test]
fn fullscreen_setting_is_a_toggle_menu_item_and_is_shown_in_settings() {
    let mut menu = PauseSelection::default();
    for _ in 0..8 {
        menu.move_down();
    }
    assert_eq!(menu.toggle_setting_index(), Some(1));
    assert_eq!(menu.selected_index(), 8);

    let settings = GameSettings {
        fullscreen: true,
        ..GameSettings::default()
    };
    let text = liteweight_fps::ui::settings_text(&settings, 8, [1280, 720]);
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
fn saving_after_corrupt_result_history_quarantines_it_and_recovers() {
    let path = std::env::temp_dir().join(format!(
        "liteweight-fps-corrupt-results-{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let original = b"[{ broken result history";
    std::fs::write(&path, original).unwrap();

    assert!(liteweight_fps::persistence::load_results_from(&path)
        .unwrap()
        .is_empty());
    let record = liteweight_fps::persistence::ResultRecord {
        recorded_at_unix_seconds: 123,
        stats: Default::default(),
    };
    liteweight_fps::persistence::save_result_to(&path, record).unwrap();
    assert_eq!(
        liteweight_fps::persistence::load_results_from(&path)
            .unwrap()
            .as_slice(),
        &[record]
    );

    let file_name = path.file_name().unwrap().to_string_lossy();
    let backup = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|candidate| {
            candidate.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with(&format!("{file_name}.corrupt."))
            })
        })
        .expect("corrupt history should be preserved beside the new history");
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(backup).unwrap();
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
    assert!(SessionMode::TrackingAim.infinite_ammo_enabled(false));
    let mut tracking = TrainingSession::new(SessionMode::TrackingAim);
    assert_eq!(tracking.remaining_seconds(), None);
    assert!(!tracking.advance(10_000.0));
    assert!(!tracking.is_complete());
}

#[test]
fn title_menu_selects_both_training_modes_settings_and_quit() {
    let mut menu = TitleSelection::default();
    assert_eq!(menu.activate(), TitleMenuAction::FreeTraining);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::FiveMinuteChallenge);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::TrackingAim);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::Settings);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::Quit);
    menu.move_down();
    assert_eq!(menu.activate(), TitleMenuAction::FreeTraining);
}

#[test]
fn tracking_mode_uses_enclosed_cover_free_range_with_multiple_moving_targets() {
    let scene = liteweight_fps::world::tracking_range();
    assert_eq!(scene.vertices.len(), 120);
    assert_eq!(
        scene
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::MIN, f32::max),
        8.0
    );
    assert!(scene
        .vertices
        .iter()
        .all(|vertex| { vertex.position[0].abs() <= 20.0 && vertex.position[2].abs() <= 12.0 }));
    assert_eq!(liteweight_fps::world::tracking_colliders().len(), 5);

    let mut combat = liteweight_fps::simulation::combat::Combat::default();
    let player_position = glam::Vec3::new(2.0, 1.65, 1.0);
    combat.configure_tracking_bots(5, 100, 1.0, 2.0, player_position);
    assert_eq!(combat.targets.len(), 5);
    assert!(combat.targets.iter().all(|bot| {
        bot.behavior == liteweight_fps::simulation::bot::BotBehavior::TrackingStrafe
    }));
    assert!(combat.targets.iter().all(|bot| {
        let distance =
            liteweight_fps::simulation::bot::horizontal_distance(bot.position, player_position);
        (5.0..=11.5).contains(&distance)
    }));
    let starts: Vec<_> = combat.targets.iter().map(|bot| bot.position).collect();
    combat.advance(
        1.0,
        player_position,
        &liteweight_fps::world::tracking_colliders(),
    );
    assert!(combat
        .targets
        .iter()
        .zip(starts)
        .all(|(bot, start)| bot.position != start));
    assert!(combat
        .targets
        .iter()
        .all(|bot| { bot.position.x.abs() < 20.0 && bot.position.z.abs() < 12.0 }));

    combat.configure_tracking_bots(20, 100, 1.0, 2.0, player_position);
    assert_eq!(combat.targets.len(), 5);
    assert!(combat
        .targets
        .iter()
        .all(|bot| { bot.position.x.abs() < 20.0 && bot.position.z.abs() < 12.0 }));
    assert!(combat.targets[0].apply_damage(300));
    combat.advance(
        2.1,
        player_position,
        &liteweight_fps::world::tracking_colliders(),
    );
    assert!(combat.targets[0].alive);
    assert!(combat.targets[0].position.x.abs() < 20.0);
    assert!(combat.targets[0].position.z.abs() < 12.0);
    let respawn_distance = liteweight_fps::simulation::bot::horizontal_distance(
        combat.targets[0].position,
        player_position,
    );
    assert!((5.0..=11.5).contains(&respawn_distance));

    let edge_player = glam::Vec3::new(18.0, 1.65, 10.0);
    let mut edge_combat = liteweight_fps::simulation::combat::Combat::default();
    edge_combat.configure_tracking_bots(5, 100, 1.0, 2.0, edge_player);
    assert_eq!(edge_combat.targets.len(), 5);
    assert!(edge_combat.targets.iter().all(|bot| {
        let distance =
            liteweight_fps::simulation::bot::horizontal_distance(bot.position, edge_player);
        (5.0..=11.5).contains(&distance) && bot.position.x.abs() <= 16.5 && bot.position.z <= 10.0
    }));
}

#[test]
fn tracking_aim_moves_horizontally_from_view_and_bots_face_the_player() {
    let mut combat = liteweight_fps::simulation::combat::Combat::default();
    let player_position = glam::Vec3::ZERO;
    combat.configure_tracking_bots(5, 100, 1.0, 2.0, player_position);
    let starts: Vec<_> = combat.targets.iter().map(|bot| bot.position).collect();
    let view_right = glam::Vec3::Z;

    combat.advance_with_view(
        1.0 / 60.0,
        player_position,
        view_right,
        &liteweight_fps::world::tracking_colliders(),
    );

    for (bot, start) in combat.targets.iter().zip(starts) {
        let movement = bot.position - start;
        assert!(movement.z.abs() > movement.x.abs());
        let toward_player = (player_position - bot.position).normalize();
        let bot_forward = glam::Vec3::new(-bot.yaw.sin(), 0.0, -bot.yaw.cos());
        assert!(bot_forward.dot(toward_player) > 0.999);
    }

    let before_turn: Vec<_> = combat.targets.iter().map(|bot| bot.position).collect();
    combat.advance_with_view(
        1.0 / 60.0,
        player_position,
        glam::Vec3::X,
        &liteweight_fps::world::tracking_colliders(),
    );
    for (bot, start) in combat.targets.iter().zip(before_turn) {
        let movement = bot.position - start;
        assert!(movement.x.abs() > movement.z.abs());
    }

    combat.targets[0].apply_damage(300);
    combat.advance_with_view(
        2.1,
        player_position,
        view_right,
        &liteweight_fps::world::tracking_colliders(),
    );
    let respawned = &combat.targets[0];
    let toward_player = (player_position - respawned.position).normalize();
    let bot_forward = glam::Vec3::new(-respawned.yaw.sin(), 0.0, -respawned.yaw.cos());
    assert!(bot_forward.dot(toward_player) > 0.999);
}

#[test]
fn tracking_respawns_keep_the_bot_group_spread_out() {
    let player_position = glam::Vec3::ZERO;
    let positions = [
        glam::Vec3::new(0.0, 0.0, -8.5),
        glam::Vec3::new(5.5, 0.0, 0.0),
        glam::Vec3::new(0.0, 0.0, 8.5),
        glam::Vec3::new(-8.5, 0.0, 0.0),
        glam::Vec3::new(11.0, 0.0, 0.0),
    ];
    let bots = positions
        .into_iter()
        .enumerate()
        .map(|(index, position)| {
            liteweight_fps::simulation::bot::Bot::with_behavior(
                position,
                liteweight_fps::simulation::bot::BotBehavior::TrackingStrafe,
                index,
            )
        })
        .collect();
    let mut combat = liteweight_fps::simulation::combat::Combat::new(bots);
    for bot in &mut combat.targets {
        bot.speed_multiplier = 0.0;
        bot.respawn_seconds = 0.5;
    }

    for index in 0..combat.targets.len() {
        combat.targets[index].apply_damage(100);
        combat.targets[index].respawn_remaining = 0.0;
        combat.advance(
            0.0,
            player_position,
            &liteweight_fps::world::tracking_colliders(),
        );

        assert!(combat.targets[index].alive);
        for (bot_index, bot) in combat.targets.iter().enumerate() {
            for other in combat.targets.iter().skip(bot_index + 1) {
                let distance = liteweight_fps::simulation::bot::horizontal_distance(
                    bot.position,
                    other.position,
                );
                assert!(
                    distance >= 2.5,
                    "respawned bots are only {distance:.2}m apart"
                );
            }
        }
    }
}

#[test]
fn tracking_aim_triples_configured_bot_hp_and_restores_it_on_respawn() {
    let mut combat = liteweight_fps::simulation::combat::Combat::default();
    let player_position = glam::Vec3::ZERO;
    combat.configure_tracking_bots(5, 100, 1.0, 0.5, player_position);
    assert!(combat.targets.iter().all(|bot| bot.hp == 300));

    combat.configure_tracking_bots(5, 120, 1.0, 0.5, player_position);
    assert!(combat.targets.iter().all(|bot| bot.hp == 360));
    combat.targets[0].apply_damage(360);
    combat.advance(
        0.6,
        player_position,
        &liteweight_fps::world::tracking_colliders(),
    );

    assert!(combat.targets[0].alive);
    assert_eq!(combat.targets[0].hp, 360);
}

#[test]
fn tracking_aim_disables_camera_and_weapon_recoil() {
    use liteweight_fps::simulation::game_mode::SessionMode;

    assert!(!SessionMode::TrackingAim.recoil_enabled());
    assert!(SessionMode::FreeTraining.recoil_enabled());
    assert!(!SessionMode::TrackingAim.hip_fire_spread_enabled());
    assert!(SessionMode::FreeTraining.hip_fire_spread_enabled());
    assert!(SessionMode::FiveMinuteChallenge.hip_fire_spread_enabled());
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
