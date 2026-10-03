//! M4確認用の最小HUD。テクスチャ・フォントの依存は持たない。
use crate::{
    simulation::{
        combat::Combat,
        weapon::{Weapon, WeaponKind},
    },
    Scene, Vertex,
};
use glam::Vec3;

pub fn world_mesh(combat: &Combat) -> Scene {
    let mut mesh = Scene {
        vertices: Vec::with_capacity(168),
        indices: Vec::with_capacity(252),
    };
    for t in &combat.targets {
        for (i, b) in t.boxes().iter().enumerate() {
            let color = if t.flash > 0.0 {
                [1., 0.95, 0.7]
            } else if i == 1 {
                [1., 0.6, 0.2]
            } else {
                [0.9, 0.25, 0.12]
            };
            mesh.cube((b.min + b.max) * 0.5, b.max - b.min, color);
        }
    }
    if combat.feedback > 0.0 && combat.last_shot.impact {
        mesh.cube(combat.last_shot.position, Vec3::splat(0.09), [1., 0.9, 0.4]);
    }
    mesh
}
fn rect(mesh: &mut Scene, x: f32, y: f32, w: f32, h: f32, color: [f32; 3], aspect: f32) {
    let base = mesh.vertices.len() as u16;
    for (dx, dy) in [(0., 0.), (w, 0.), (w, h), (0., h)] {
        mesh.vertices.push(Vertex {
            position: [(x + dx) / aspect, y + dy, 0.],
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}
fn digit(mesh: &mut Scene, n: u32, x: f32, y: f32, aspect: f32) {
    let masks = [
        0b1111110, 0b0110000, 0b1101101, 0b1111001, 0b0110011, 0b1011011, 0b1011111, 0b1110000,
        0b1111111, 0b1111011,
    ];
    let segments = [
        (0., 0.08, 0.04, 0.008),
        (0.032, 0.04, 0.008, 0.04),
        (0.032, 0., 0.008, 0.04),
        (0., 0., 0.04, 0.008),
        (0., 0., 0.008, 0.04),
        (0., 0.04, 0.008, 0.04),
        (0., 0.04, 0.04, 0.008),
    ];
    for (i, (dx, dy, w, h)) in segments.into_iter().enumerate() {
        if masks[n as usize] & (1 << (6 - i)) != 0 {
            rect(mesh, x + dx, y + dy, w, h, [0.9; 3], aspect);
        }
    }
}
const SCOPE_MAX_RADIUS: f32 = 0.56;
const SCOPE_MAX_SHADOW_WIDTH: f32 = 0.8;
const SCOPE_START_OFFSET: f32 = 0.10;
const SCOPE_SHADOW_SEGMENTS: usize = 24;
const SCOPE_RING_SEGMENTS: usize = 32;
const SCOPE_TICK_SIZE_RATIO: f32 = 0.018;
const SCOPE_RETICLE_ARM_RATIO: f32 = 0.68;
const SCOPE_RETICLE_LINE_RATIO: f32 = 0.008;
const PERIPHERAL_DARKEN_START: f32 = 0.3;
const PERIPHERAL_DARKEN_CURVE: f32 = 1.5;
const PERIPHERAL_MAX_OPACITY: f32 = 0.98;
const SCOPE_SHADOW_PEAK_ALPHA: f32 = 0.65;

#[derive(Clone, Copy, Debug)]
struct ScopeVisualState {
    center_x: f32,
    aperture_radius: f32,
    shadow_width: f32,
    peripheral_opacity: f32,
}
impl ScopeVisualState {
    fn from_progress(progress: f32) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        let darken_progress = ((progress - PERIPHERAL_DARKEN_START)
            / (1.0 - PERIPHERAL_DARKEN_START))
            .clamp(0.0, 1.0);
        Self {
            center_x: SCOPE_START_OFFSET * (1.0 - eased),
            aperture_radius: SCOPE_MAX_RADIUS * eased,
            shadow_width: SCOPE_MAX_SHADOW_WIDTH * progress * (1.0 - progress),
            peripheral_opacity: PERIPHERAL_MAX_OPACITY
                * darken_progress.powf(PERIPHERAL_DARKEN_CURVE),
        }
    }
}

fn scope_shadow(mesh: &mut Scene, aspect: f32, state: ScopeVisualState) {
    let inner = state.aperture_radius;
    let shadow_peak_width = SCOPE_MAX_SHADOW_WIDTH * 0.25;
    let inner_opacity =
        SCOPE_SHADOW_PEAK_ALPHA * (state.shadow_width / shadow_peak_width).clamp(0.0, 1.0);
    for i in 0..SCOPE_SHADOW_SEGMENTS {
        let a0 = i as f32 * std::f32::consts::TAU / SCOPE_SHADOW_SEGMENTS as f32;
        let a1 = (i + 1) as f32 * std::f32::consts::TAU / SCOPE_SHADOW_SEGMENTS as f32;
        let point = |angle: f32| {
            let c = angle.cos();
            let s = angle.sin();
            let to_x_edge = if c > 0.0 {
                (aspect - state.center_x) / c
            } else if c < 0.0 {
                (-aspect - state.center_x) / c
            } else {
                f32::INFINITY
            };
            let to_y_edge = if s.abs() > f32::EPSILON {
                1.0 / s.abs()
            } else {
                f32::INFINITY
            };
            let radius = to_x_edge.min(to_y_edge);
            [(state.center_x + c * radius) / aspect, s * radius]
        };
        let shadow_outer = inner + state.shadow_width;
        let at_angle = |angle: f32, radius: f32| {
            [
                (state.center_x + angle.cos() * radius) / aspect,
                angle.sin() * radius,
            ]
        };
        if state.shadow_width > 0.0 {
            push_scope_mask_quad(
                mesh,
                [
                    at_angle(a0, inner),
                    at_angle(a0, shadow_outer),
                    at_angle(a1, shadow_outer),
                    at_angle(a1, inner),
                ],
                [inner_opacity, 0.0, 0.0, inner_opacity],
            );
        }
        if state.peripheral_opacity > 0.0 {
            push_scope_mask_quad(
                mesh,
                [
                    at_angle(a0, shadow_outer),
                    point(a0),
                    point(a1),
                    at_angle(a1, shadow_outer),
                ],
                [state.peripheral_opacity; 4],
            );
        }
    }
}

fn push_scope_mask_quad(mesh: &mut Scene, points: [[f32; 2]; 4], opacity: [f32; 4]) {
    let base = mesh.vertices.len() as u16;
    for (position, alpha) in points.into_iter().zip(opacity) {
        mesh.vertices.push(Vertex {
            position: [position[0], position[1], 0.0],
            color: [0.0, 0.0, 0.0, alpha],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn scope_picture(mesh: &mut Scene, aspect: f32, progress: f32) {
    let state = ScopeVisualState::from_progress(progress);
    if state.aperture_radius == 0.0 {
        return;
    }
    scope_shadow(mesh, aspect, state);
    let radius = state.aperture_radius;
    let tick = radius * SCOPE_TICK_SIZE_RATIO;
    for i in 0..SCOPE_RING_SEGMENTS {
        let angle = i as f32 * std::f32::consts::TAU / SCOPE_RING_SEGMENTS as f32;
        rect(
            mesh,
            state.center_x + angle.cos() * radius - tick * 0.5,
            angle.sin() * radius - tick * 0.5,
            tick,
            tick,
            [0.12, 0.15, 0.17],
            aspect,
        );
    }
    let arm = radius * SCOPE_RETICLE_ARM_RATIO;
    let line = radius * SCOPE_RETICLE_LINE_RATIO;
    rect(
        mesh,
        state.center_x - line * 0.5,
        -arm,
        line,
        arm * 2.0,
        [0.14, 0.16, 0.18],
        aspect,
    );
    rect(
        mesh,
        state.center_x - arm,
        -line * 0.5,
        arm * 2.0,
        line,
        [0.14, 0.16, 0.18],
        aspect,
    );
}

pub fn hud_mesh(weapon: &Weapon, combat: &Combat, aspect: f32, weapon_y_offset: f32) -> Scene {
    let mut m = Scene {
        vertices: Vec::with_capacity(400),
        indices: Vec::with_capacity(600),
    };
    // 武器はADS中も表示し続け、サイトピクチャーを別の軽量2Dメッシュで重ねる。
    {
        let (body_w, body_h, color) = match weapon.kind {
            WeaponKind::Pistol => (0.11, 0.28, [0.2, 0.22, 0.25]),
            WeaponKind::AssaultRifle => (0.17, 0.4, [0.13, 0.16, 0.19]),
            WeaponKind::Sniper => (0.12, 0.48, [0.16, 0.19, 0.22]),
        };
        let x = 0.36 * (1. - weapon.ads_progress);
        let y = -0.7 + weapon_y_offset
            - weapon.visual_recoil * 0.045
            - if weapon.reload_remaining > 0.0 {
                0.22
            } else {
                0.0
            };
        rect(&mut m, x - body_w * 0.5, y, body_w, body_h, color, aspect);
        rect(
            &mut m,
            x - 0.04,
            y + body_h - 0.04,
            0.08,
            0.11,
            [0.28, 0.32, 0.36],
            aspect,
        );
        match weapon.kind {
            WeaponKind::Pistol => {
                rect(
                    &mut m,
                    x - body_w * 0.36,
                    y + body_h * 0.58,
                    body_w * 0.72,
                    body_h * 0.34,
                    [0.38, 0.4, 0.42],
                    aspect,
                ); // slide
                rect(
                    &mut m,
                    x - 0.012,
                    y + body_h * 0.91,
                    0.024,
                    0.12,
                    [0.22, 0.24, 0.26],
                    aspect,
                ); // barrel
                rect(
                    &mut m,
                    x - 0.022,
                    y - 0.09,
                    0.044,
                    0.12,
                    [0.12, 0.14, 0.16],
                    aspect,
                ); // grip
                for rib in 0..4 {
                    rect(
                        &mut m,
                        x - 0.017,
                        y - 0.075 + rib as f32 * 0.022,
                        0.034,
                        0.006,
                        [0.32, 0.34, 0.36],
                        aspect,
                    );
                }
                rect(
                    &mut m,
                    x - 0.008,
                    y + body_h + 0.02,
                    0.016,
                    0.035,
                    [0.7, 0.45, 0.12],
                    aspect,
                ); // front sight
            }
            WeaponKind::AssaultRifle => {
                rect(
                    &mut m,
                    x - 0.105,
                    y + 0.06,
                    0.045,
                    0.15,
                    [0.24, 0.27, 0.3],
                    aspect,
                ); // stock
                rect(
                    &mut m,
                    x - 0.024,
                    y + body_h + 0.07,
                    0.048,
                    0.19,
                    [0.36, 0.38, 0.4],
                    aspect,
                ); // barrel
                rect(
                    &mut m,
                    x - 0.07,
                    y + 0.11,
                    0.14,
                    0.12,
                    [0.28, 0.31, 0.34],
                    aspect,
                ); // handguard
                for rib in 0..5 {
                    rect(
                        &mut m,
                        x - 0.066 + rib as f32 * 0.03,
                        y + 0.12,
                        0.006,
                        0.10,
                        [0.42, 0.44, 0.46],
                        aspect,
                    );
                }
                rect(
                    &mut m,
                    x - 0.036,
                    y - 0.15,
                    0.072,
                    0.18,
                    [0.18, 0.21, 0.23],
                    aspect,
                ); // magazine
                rect(
                    &mut m,
                    x - 0.08,
                    y + body_h + 0.02,
                    0.16,
                    0.035,
                    [0.25, 0.28, 0.31],
                    aspect,
                ); // optic
                rect(
                    &mut m,
                    x - 0.025,
                    y + body_h + 0.055,
                    0.05,
                    0.025,
                    [0.1, 0.14, 0.15],
                    aspect,
                );
            }
            WeaponKind::Sniper => {
                rect(
                    &mut m,
                    x - 0.018,
                    y + body_h * 0.52,
                    0.036,
                    0.31,
                    [0.34, 0.37, 0.39],
                    aspect,
                ); // long barrel
                rect(
                    &mut m,
                    x - 0.07,
                    y + body_h * 0.61,
                    0.14,
                    0.035,
                    [0.22, 0.24, 0.26],
                    aspect,
                ); // receiver
                rect(
                    &mut m,
                    x - 0.045,
                    y + body_h * 0.84,
                    0.09,
                    0.11,
                    [0.12, 0.15, 0.18],
                    aspect,
                ); // scope tube
                rect(
                    &mut m,
                    x - 0.058,
                    y + body_h * 0.92,
                    0.116,
                    0.035,
                    [0.25, 0.29, 0.32],
                    aspect,
                ); // scope rim
                rect(
                    &mut m,
                    x + 0.05,
                    y + body_h * 0.57,
                    0.07,
                    0.018,
                    [0.5, 0.52, 0.54],
                    aspect,
                ); // bolt handle
                rect(
                    &mut m,
                    x - 0.018,
                    y - 0.13,
                    0.036,
                    0.15,
                    [0.12, 0.14, 0.17],
                    aspect,
                ); // magazine
            }
        }
        if combat.feedback > 0.075 {
            rect(
                &mut m,
                x - 0.035,
                y + body_h + 0.18,
                0.07,
                0.055,
                [1., 0.8, 0.2],
                aspect,
            );
        }
    }
    if weapon.kind == WeaponKind::Sniper {
        scope_picture(&mut m, aspect, weapon.ads_progress);
    }
    let gap = 0.008 + weapon.spread(false) * 1.4;
    for sign in [-1., 1.] {
        rect(
            &mut m,
            sign * gap - 0.006,
            -0.0015,
            0.012,
            0.003,
            [0.9; 3],
            aspect,
        );
        rect(
            &mut m,
            -0.0015,
            sign * gap - 0.006,
            0.003,
            0.012,
            [0.9; 3],
            aspect,
        );
    }
    rect(&mut m, -0.002, -0.002, 0.004, 0.004, [1.; 3], aspect);
    if combat.feedback > 0.0 && combat.last_shot.hit {
        let color = if combat.last_shot.killed {
            [1., 0.2, 0.2]
        } else if combat.last_shot.headshot {
            [1., 0.7, 0.1]
        } else {
            [1.; 3]
        };
        for x in [-0.025, 0.021] {
            for y in [-0.025, 0.021] {
                rect(&mut m, x, y, 0.006, 0.006, color, aspect);
            }
        }
    }
    if weapon.bolt_remaining > 0.0 {
        rect(
            &mut m,
            -0.15,
            -0.82,
            0.3 * (1. - weapon.bolt_remaining / weapon.data.bolt_seconds),
            0.012,
            [1., 0.65, 0.25],
            aspect,
        );
    }
    digit(&mut m, weapon.ammo / 10, aspect - 0.19, -0.87, aspect);
    digit(&mut m, weapon.ammo % 10, aspect - 0.13, -0.87, aspect);
    if weapon.reload_remaining > 0.0 {
        rect(
            &mut m,
            -0.15,
            -0.82,
            0.3 * (1. - weapon.reload_remaining / weapon.data.reload_seconds),
            0.015,
            [0.3, 0.8, 1.],
            aspect,
        );
    }
    m
}

#[cfg(test)]
mod tests {
    use super::{scope_shadow, ScopeVisualState, SCOPE_MAX_RADIUS, SCOPE_SHADOW_SEGMENTS};

    #[test]
    fn scope_view_grows_smoothly_while_eye_alignment_shadow_fades() {
        let hip = ScopeVisualState::from_progress(0.0);
        let early = ScopeVisualState::from_progress(0.3);
        let middle = ScopeVisualState::from_progress(0.5);
        let late = ScopeVisualState::from_progress(0.7);
        let aligned = ScopeVisualState::from_progress(1.0);

        assert_eq!(hip.aperture_radius, 0.0);
        assert!(early.aperture_radius < middle.aperture_radius);
        assert!(middle.aperture_radius < late.aperture_radius);
        assert!(late.aperture_radius < aligned.aperture_radius);
        assert!(early.shadow_width > 0.0);
        assert!(middle.shadow_width > early.shadow_width);
        assert!(late.shadow_width < middle.shadow_width);
        assert_eq!(aligned.shadow_width, 0.0);
        assert!(hip.center_x > early.center_x);
        assert!(early.center_x > middle.center_x);
        assert!(middle.center_x > aligned.center_x);
    }

    #[test]
    fn scope_geometry_is_bounded_for_out_of_range_progress() {
        let below = ScopeVisualState::from_progress(-2.0);
        let above = ScopeVisualState::from_progress(2.0);
        assert_eq!(below.aperture_radius, 0.0);
        assert_eq!(
            above.aperture_radius,
            ScopeVisualState::from_progress(1.0).aperture_radius
        );
    }

    #[test]
    fn peripheral_opacity_begins_gradually_and_rises_to_near_black() {
        let levels = [0.0, 0.3, 0.5, 0.7, 0.9, 1.0]
            .map(|progress| ScopeVisualState::from_progress(progress).peripheral_opacity);
        assert_eq!(levels[0], 0.0);
        assert_eq!(levels[1], 0.0);
        assert!(levels[2] > levels[1]);
        assert!(levels[3] > levels[2]);
        assert!(levels[4] > levels[3]);
        assert!(levels[5] > 0.95);
        assert!(levels.windows(2).all(|pair| pair[1] >= pair[0]));
    }

    #[test]
    fn peripheral_mask_keeps_scope_opening_clear_and_darkens_screen_edges() {
        let mut aligned = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        scope_shadow(
            &mut aligned,
            16.0 / 9.0,
            ScopeVisualState::from_progress(1.0),
        );
        assert_eq!(aligned.vertices.len(), SCOPE_SHADOW_SEGMENTS * 4);
        assert!((aligned.vertices[0].position[0] - SCOPE_MAX_RADIUS / (16.0 / 9.0)).abs() < 1e-5);
        assert!(aligned.vertices[0].color[3] > 0.95);

        let mut early = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        scope_shadow(&mut early, 16.0 / 9.0, ScopeVisualState::from_progress(0.3));
        assert_eq!(early.vertices.len(), SCOPE_SHADOW_SEGMENTS * 4);

        let mut partial = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        scope_shadow(
            &mut partial,
            16.0 / 9.0,
            ScopeVisualState::from_progress(0.5),
        );
        assert_eq!(partial.vertices.len(), SCOPE_SHADOW_SEGMENTS * 8);
        assert!(partial.vertices[0].color[3] > partial.vertices[1].color[3]);
        assert!(partial.vertices[4].color[3] > 0.0);
        assert_eq!(partial.vertices[4].color[3], partial.vertices[5].color[3]);
    }
}
