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
    assert_eq!(WeaponKind::Sniper.fov(1.0), 20.0);
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
            if kind == WeaponKind::Sniper && progress == 1.0 {
                assert!(
                    layout.muzzle_screen.y < -0.05,
                    "sniper barrel must stay below the centered scope axis"
                );
                assert!(layout.sight_screen.y.abs() < 1e-6);
            }
            if kind != WeaponKind::Sniper && progress == 1.0 {
                assert!(layout.muzzle_screen.y.abs() < 1e-6);
            }
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
            mesh.vertices.len() <= 130,
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
fn unscoped_sniper_reticle_matches_the_assault_rifle_shape() {
    let mut loadout = Loadout::default();
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let reticle_positions = |kind, loadout: &mut Loadout| {
        loadout.switch(kind);
        loadout.advance(0.32, false);
        liteweight_fps::ui::hud_mesh(loadout.active(), &combat, 16.0 / 9.0, 0.0)
            .vertices
            .into_iter()
            .filter(|vertex| {
                vertex.color == [0.9, 0.9, 0.9, 1.0]
                    && vertex.position[0].abs() < 0.05
                    && vertex.position[1].abs() < 0.05
            })
            .map(|vertex| [vertex.position[0], vertex.position[1]])
            .collect::<Vec<_>>()
    };

    let ar = reticle_positions(WeaponKind::AssaultRifle, &mut loadout);
    let sniper = reticle_positions(WeaponKind::Sniper, &mut loadout);
    assert_eq!(sniper, ar, "hip-fire sniper reticle should match the AR");
}

#[test]
fn sniper_scope_model_does_not_cover_the_optical_center() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    loadout.active_mut().ads_progress = 1.0;
    let layout = loadout.active().view_model_layout(0.0);
    let mesh = liteweight_fps::ui::hud_mesh(
        loadout.active(),
        &liteweight_fps::simulation::combat::Combat::default(),
        16.0 / 9.0,
        0.0,
    );
    let center = [layout.sight_screen.x / (16.0 / 9.0), layout.sight_screen.y];
    let scope_part_color =
        |color| color == [0.10, 0.12, 0.14, 1.0] || color == [0.18, 0.21, 0.23, 1.0];
    let covered_by_dark_weapon_face = mesh.indices.as_chunks::<3>().0.iter().any(|triangle| {
        let vertices =
            [triangle[0], triangle[1], triangle[2]].map(|index| mesh.vertices[index as usize]);
        if !vertices.iter().all(|vertex| scope_part_color(vertex.color)) {
            return false;
        }
        let points = vertices.map(|vertex| [vertex.position[0], vertex.position[1]]);
        let side = |a: [f32; 2], b: [f32; 2], p: [f32; 2]| {
            (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])
        };
        let sides = [
            side(points[0], points[1], center),
            side(points[1], points[2], center),
            side(points[2], points[0], center),
        ];
        sides.iter().all(|value| *value >= 0.0) || sides.iter().all(|value| *value <= 0.0)
    });
    assert!(
        !covered_by_dark_weapon_face,
        "the scope housing must leave its sight axis open"
    );
}

#[test]
fn sniper_scope_housing_fades_out_as_the_scope_view_appears() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let rim_alpha = |progress: f32, loadout: &mut Loadout| {
        loadout.active_mut().ads_progress = progress;
        liteweight_fps::ui::hud_mesh(loadout.active(), &combat, 16.0 / 9.0, 0.0)
            .vertices
            .into_iter()
            .filter(|vertex| vertex.color[..3] == [0.18, 0.21, 0.23])
            .map(|vertex| vertex.color[3])
            .fold(0.0, f32::max)
    };
    let hip_alpha = rim_alpha(0.0, &mut loadout);
    let partial_alpha = rim_alpha(0.5, &mut loadout);
    let aimed_alpha = rim_alpha(1.0, &mut loadout);
    assert!(hip_alpha > partial_alpha);
    assert!(partial_alpha > aimed_alpha);
    assert_eq!(aimed_alpha, 0.0);
}

#[test]
fn sniper_ads_reticle_stays_compact_and_thin() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    loadout.active_mut().ads_progress = 1.0;
    let aspect = 16.0 / 9.0;
    let mesh = liteweight_fps::ui::hud_mesh(
        loadout.active(),
        &liteweight_fps::simulation::combat::Combat::default(),
        aspect,
        0.0,
    );
    let reticle: Vec<[f32; 2]> = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.color == [0.14, 0.16, 0.18, 1.0])
        .map(|vertex| [vertex.position[0] * aspect, vertex.position[1]])
        .collect();
    let max_extent = reticle
        .iter()
        .map(|point| point[0].abs().max(point[1].abs()))
        .fold(0.0_f32, f32::max);
    assert!(max_extent <= 0.16, "ADS reticle extent is {max_extent}");
    let y_levels: Vec<f32> = reticle.iter().map(|point| point[1]).collect();
    let min_gap = y_levels
        .iter()
        .flat_map(|a| y_levels.iter().map(move |b| (a - b).abs()))
        .filter(|gap| *gap > 1e-6)
        .fold(f32::INFINITY, f32::min);
    assert!(
        min_gap <= 0.004,
        "ADS reticle stroke is too thick: {min_gap}"
    );
}

#[test]
fn sniper_scope_reticle_appears_only_near_the_end_of_ads() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let reticle_alpha = |progress: f32, loadout: &mut Loadout| {
        loadout.active_mut().ads_progress = progress;
        liteweight_fps::ui::hud_mesh(loadout.active(), &combat, 16.0 / 9.0, 0.0)
            .vertices
            .into_iter()
            .filter(|vertex| vertex.color[..3] == [0.14, 0.16, 0.18])
            .map(|vertex| vertex.color[3])
            .fold(0.0_f32, f32::max)
    };

    assert_eq!(reticle_alpha(0.5, &mut loadout), 0.0);
    let emerging = reticle_alpha(0.9, &mut loadout);
    assert!(emerging > 0.0 && emerging < 1.0);
    assert_eq!(reticle_alpha(1.0, &mut loadout), 1.0);
}

#[test]
fn sniper_scope_glass_reflection_fades_in_and_out_during_ads() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let glass_opacity = |progress: f32, loadout: &mut Loadout| {
        loadout.active_mut().ads_progress = progress;
        liteweight_fps::ui::hud_mesh(loadout.active(), &combat, 16.0 / 9.0, 0.0)
            .vertices
            .into_iter()
            .filter(|vertex| {
                vertex.color[..3] == [0.98, 0.96, 0.88] || vertex.color[..3] == [0.68, 0.82, 0.90]
            })
            .map(|vertex| vertex.color[3])
            .fold(0.0, f32::max)
    };
    let beginning = glass_opacity(0.0, &mut loadout);
    let early = glass_opacity(0.08, &mut loadout);
    let middle = glass_opacity(0.3, &mut loadout);
    let late = glass_opacity(0.55, &mut loadout);
    let clear = glass_opacity(0.82, &mut loadout);
    let fully_aimed = glass_opacity(1.0, &mut loadout);
    assert_eq!(beginning, 0.0);
    assert!(
        early >= 0.8,
        "initial glass reflection must obscure the scope view: {early}"
    );
    assert!(middle < early, "reflection should clear as ADS progresses");
    assert!(late > 0.0, "reflection should remain visible for longer");
    assert_eq!(clear, 0.0);
    assert_eq!(fully_aimed, 0.0);

    loadout.active_mut().ads_progress = 0.08;
    let aspect = 16.0 / 9.0;
    let layout = loadout.active().view_model_layout(0.0);
    let lens_vertices: Vec<_> =
        liteweight_fps::ui::hud_mesh(loadout.active(), &combat, aspect, 0.0)
            .vertices
            .into_iter()
            .filter(|vertex| {
                vertex.color[..3] == [0.98, 0.96, 0.88] || vertex.color[..3] == [0.68, 0.82, 0.90]
            })
            .collect();
    assert!(lens_vertices.iter().all(|vertex| vertex.color[3] >= 0.99));
    assert!(lens_vertices
        .iter()
        .any(|vertex| vertex.color[..3] == [0.98, 0.96, 0.88]));
    assert!(lens_vertices
        .iter()
        .any(|vertex| vertex.color[..3] == [0.68, 0.82, 0.90]));
    let lens_radius = lens_vertices
        .iter()
        .map(|vertex| {
            (vertex.position[0] * aspect - layout.sight_screen.x)
                .hypot(vertex.position[1] - layout.sight_screen.y)
        })
        .fold(0.0_f32, f32::max);
    assert!(
        lens_radius >= 0.05,
        "reflection must cover the physical scope lens"
    );
}

#[test]
fn sniper_rifle_model_fades_out_during_ads() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let body_alpha = |progress: f32, loadout: &mut Loadout| {
        loadout.active_mut().ads_progress = progress;
        liteweight_fps::ui::hud_mesh(loadout.active(), &combat, 16.0 / 9.0, 0.0)
            .vertices
            .into_iter()
            .filter(|vertex| vertex.color[..3] == [0.19, 0.22, 0.24])
            .map(|vertex| vertex.color[3])
            .fold(0.0, f32::max)
    };
    let hip_alpha = body_alpha(0.0, &mut loadout);
    let partial_alpha = body_alpha(0.5, &mut loadout);
    let aimed_alpha = body_alpha(1.0, &mut loadout);
    assert!(hip_alpha > partial_alpha);
    assert!(partial_alpha > aimed_alpha);
    assert_eq!(aimed_alpha, 0.0);
}

#[test]
fn sniper_scope_rim_is_a_hollow_circle_when_unscoped() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let layout = loadout.active().view_model_layout(0.0);
    let aspect = 16.0 / 9.0;
    let mesh = liteweight_fps::ui::hud_mesh(
        loadout.active(),
        &liteweight_fps::simulation::combat::Combat::default(),
        aspect,
        0.0,
    );
    let radii: Vec<f32> = mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.color == [0.18, 0.21, 0.23, 1.0])
        .map(|vertex| {
            let x = vertex.position[0] * aspect - layout.sight_screen.x;
            let y = vertex.position[1] - layout.sight_screen.y;
            x.hypot(y)
        })
        .collect();
    assert_eq!(radii.len(), 48, "12 low-poly ring segments expected");
    assert!(
        radii
            .iter()
            .all(|radius| { (radius - 0.05).abs() < 1e-5 || (radius - 0.035).abs() < 1e-5 }),
        "scope rim vertices must lie on circular inner and outer edges"
    );
}

#[test]
fn sniper_scope_lens_moves_to_eye_and_grows_during_ads() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let combat = liteweight_fps::simulation::combat::Combat::default();
    let aspect = 16.0 / 9.0;
    let measure_scope = |progress: f32, loadout: &mut Loadout| {
        loadout.active_mut().ads_progress = progress;
        let mesh = liteweight_fps::ui::hud_mesh(loadout.active(), &combat, aspect, 0.0);
        let vertices: Vec<_> = mesh
            .vertices
            .into_iter()
            .filter(|vertex| vertex.color[..3] == [0.18, 0.21, 0.23] && vertex.color[3] > 0.0)
            .collect();
        let center = vertices.iter().fold([0.0, 0.0], |sum, vertex| {
            [
                sum[0] + vertex.position[0] * aspect / vertices.len() as f32,
                sum[1] + vertex.position[1] / vertices.len() as f32,
            ]
        });
        let radius = vertices
            .iter()
            .map(|vertex| {
                (vertex.position[0] * aspect - center[0]).hypot(vertex.position[1] - center[1])
            })
            .fold(0.0_f32, f32::max);
        (center, radius)
    };
    let (hip_center, hip_radius) = measure_scope(0.0, &mut loadout);
    let (raised_center, raised_radius) = measure_scope(0.5, &mut loadout);
    assert!(raised_center[0].hypot(raised_center[1]) < hip_center[0].hypot(hip_center[1]));
    assert!(raised_radius > hip_radius * 5.0);
}

#[test]
fn sniper_ads_fov_change_does_not_move_or_pitch_the_player_camera() {
    let mut player = liteweight_fps::simulation::Player::new();
    player.look(0.3, 0.2);
    let wide = player.view_matrix_with(16.0 / 9.0, 70.0);
    let scoped = player.view_matrix_with(16.0 / 9.0, 20.0);
    let world_ray = |matrix: glam::Mat4| {
        (matrix.inverse().project_point3(Vec3::new(0.0, 0.0, 0.5)) - player.position).normalize()
    };
    assert!(world_ray(wide).dot(world_ray(scoped)) > 0.99999);
    assert_eq!(player.position, Vec3::new(0.0, 1.6, 0.0));
}

#[test]
fn hip_fire_direction_has_no_random_shot_to_shot_spread() {
    let mut loadout = Loadout::default();
    let view = -Vec3::Z;
    let ar = loadout.active();
    assert_eq!(ar.shot_direction(view), view);

    loadout.switch(WeaponKind::Sniper);
    let sniper = loadout.active();
    let sniper_direction = sniper.shot_direction(view);
    assert_eq!(sniper_direction, sniper.shot_direction(view));
    assert_ne!(sniper_direction, view);

    loadout.switch(WeaponKind::Pistol);
    let pistol = loadout.active();
    assert_eq!(pistol.shot_direction(view), view);
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
fn sniper_ads_progress_moves_in_and_out_smoothly() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    loadout.advance(0.25, true);
    assert!((loadout.active().ads_progress - 0.5).abs() < 1e-5);
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
    loadout.advance(0.25, true);
    let halfway = loadout.active().shot_direction(view);
    assert!(loadout.fire(true, true));
    loadout.advance(0.25, true);
    let fully_aimed = loadout.active().shot_direction(view);

    assert!((hip - view).length() > 0.01);
    assert!((halfway - view).length() < (hip - view).length());
    assert!((fully_aimed - view).length() < 1e-6);
}

#[test]
fn sniper_quick_shot_alignment_converges_earlier_than_ads_progress() {
    use glam::Vec3;

    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    let view = Vec3::new(0.0, 0.0, -1.0);
    loadout.advance(0.25, true);
    let halfway = loadout.active().shot_direction(view);
    let deviation_degrees = halfway.dot(view).clamp(-1.0, 1.0).acos().to_degrees();
    assert!(
        deviation_degrees <= 1.1,
        "halfway ADS shot is still misaligned by {deviation_degrees} degrees"
    );
}

#[test]
fn sniper_quick_shot_alignment_is_usable_before_half_ads() {
    use glam::Vec3;

    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    loadout.advance(0.2, true);
    let view = Vec3::new(0.0, 0.0, -1.0);
    let shot = loadout.active().shot_direction(view);
    let deviation_degrees = shot.dot(view).clamp(-1.0, 1.0).acos().to_degrees();
    assert!(
        deviation_degrees <= 1.2,
        "quick-shot alignment at 40% ADS is {deviation_degrees} degrees"
    );
}

#[test]
fn sniper_scope_fov_moves_smoothly_from_70_to_20_degrees_over_500ms() {
    let mut loadout = Loadout::default();
    loadout.switch(WeaponKind::Sniper);
    loadout.advance(0.32, false);
    assert_eq!(loadout.fov(), 70.);
    loadout.advance(0.425, true);
    assert!((loadout.fov() - 27.5).abs() < 1e-3);
    loadout.advance(0.075, true);
    assert!((loadout.fov() - 20.).abs() < 1e-5);
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
    assert!(fully_aimed.vertices.len() < partial.vertices.len());
    let muzzle = loadout.active().view_model_layout(0.0).muzzle_screen;
    assert!(fully_aimed.vertices.iter().any(|vertex| {
        (vertex.position[0] - muzzle.x / (16. / 9.)).abs() < 1e-6
            && (vertex.position[1] - muzzle.y).abs() < 1e-6
    }));
}
