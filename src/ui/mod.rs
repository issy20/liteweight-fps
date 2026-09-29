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
            color,
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
pub fn hud_mesh(weapon: &Weapon, combat: &Combat, aspect: f32) -> Scene {
    let mut m = Scene {
        vertices: Vec::with_capacity(400),
        indices: Vec::with_capacity(600),
    };
    // 武器ごとに形状と大きさを変える。スナイパーADSはメインカメラFOVに重ねる2Dリング。
    let scoped = weapon.kind == WeaponKind::Sniper && weapon.ads > 0.05;
    if scoped {
        let radius = 0.52;
        for i in 0..48 {
            let angle = i as f32 * std::f32::consts::TAU / 48.;
            rect(
                &mut m,
                angle.cos() * radius - 0.004,
                angle.sin() * radius - 0.004,
                0.008,
                0.008,
                [0.12, 0.15, 0.17],
                aspect,
            );
        }
        rect(
            &mut m,
            -0.001,
            -0.38,
            0.002,
            0.76,
            [0.14, 0.16, 0.18],
            aspect,
        );
        rect(
            &mut m,
            -0.38,
            -0.001,
            0.76,
            0.002,
            [0.14, 0.16, 0.18],
            aspect,
        );
    } else {
        let (body_w, body_h, color) = match weapon.kind {
            WeaponKind::Pistol => (0.11, 0.28, [0.2, 0.22, 0.25]),
            WeaponKind::AssaultRifle => (0.17, 0.4, [0.13, 0.16, 0.19]),
            WeaponKind::Sniper => (0.12, 0.48, [0.16, 0.19, 0.22]),
        };
        let x = 0.36 * (1. - weapon.ads);
        let y = -0.7
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
        rect(
            &mut m,
            x - 0.018,
            y + body_h + 0.06,
            0.036,
            0.11,
            [0.15, 0.18, 0.21],
            aspect,
        );
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
