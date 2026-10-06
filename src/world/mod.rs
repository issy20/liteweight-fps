use crate::physics::Aabb;
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}
pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}
impl Scene {
    pub fn cube(&mut self, center: Vec3, size: Vec3, color: [f32; 3]) {
        let corners = [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ];
        for (face, shade) in [
            ([0, 1, 2, 3], 0.65),
            ([5, 4, 7, 6], 0.85),
            ([4, 0, 3, 7], 0.7),
            ([1, 5, 6, 2], 0.8),
            ([3, 2, 6, 7], 1.0),
            ([4, 5, 1, 0], 0.5),
        ] {
            let base = self.vertices.len() as u16;
            for i in face {
                self.vertices.push(Vertex {
                    position: (center + Vec3::from(corners[i]) * size * 0.5).to_array(),
                    color: [color[0] * shade, color[1] * shade, color[2] * shade, 1.0],
                });
            }
            self.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}
/// 静的な50m角のアリーナ。移動・衝突・AIには依存しない。
pub fn arena() -> Scene {
    arena_scaled(1.0)
}

/// BOT数1〜10に合わせて、50m角を34m〜70mへ水平スケールする。
pub fn arena_scale_for_bot_count(count: u32) -> f32 {
    0.68 + (count.clamp(1, 10) - 1) as f32 * 0.08
}

pub fn arena_scaled(scale: f32) -> Scene {
    let scale = if scale.is_finite() {
        scale.clamp(0.68, 1.4)
    } else {
        1.0
    };
    let horizontal = |v: Vec3| Vec3::new(v.x * scale, v.y, v.z * scale);
    let mut s = Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    s.cube(
        horizontal(Vec3::new(0., -0.25, 0.)),
        horizontal(Vec3::new(50., 0.5, 50.)),
        [0.35, 0.4, 0.45],
    );
    for x in [-25., 25.] {
        s.cube(
            horizontal(Vec3::new(x, 2., 0.)),
            horizontal(Vec3::new(0.5, 4., 50.)),
            [0.5, 0.55, 0.6],
        );
        s.cube(
            horizontal(Vec3::new(0., 2., x)),
            horizontal(Vec3::new(50., 4., 0.5)),
            [0.5, 0.55, 0.6],
        );
    }
    for x in [-12., 0., 12.] {
        for z in [-8., 8.] {
            s.cube(
                horizontal(Vec3::new(x, 1., z)),
                horizontal(Vec3::new(3., 2., 3.)),
                [0.3, 0.5, 0.55],
            );
        }
    }
    s
}

/// M0で固定した負荷比較用シーン。
pub fn scene(dummies: u32) -> Scene {
    let mut s = arena();
    for i in 0..dummies {
        let x = (i % 5) as f32 * 7. - 14.;
        let z = (i / 5) as f32 * 5. - 10.;
        s.cube(
            Vec3::new(x, 0.8, z),
            Vec3::new(0.7, 1.6, 0.5),
            [0.9, 0.25, 0.12],
        );
        s.cube(Vec3::new(x, 1.85, z), Vec3::splat(0.5), [1., 0.6, 0.2]);
    }
    s
}

/// アリーナの床・壁・遮蔽物をAABBコライダーとして返す。床は範囲外落下防止のための平面。
pub fn arena_colliders() -> Vec<Aabb> {
    arena_colliders_scaled(1.0)
}

pub fn arena_colliders_scaled(scale: f32) -> Vec<Aabb> {
    let scale = if scale.is_finite() {
        scale.clamp(0.68, 1.4)
    } else {
        1.0
    };
    let mut colliders = Vec::new();
    colliders.push(Aabb::new(
        Vec3::new(-25. * scale, -0.5, -25. * scale),
        Vec3::new(25. * scale, 0., 25. * scale),
    ));
    for x in [-25., 25.] {
        colliders.push(Aabb::new(
            Vec3::new(x * scale - 0.25 * scale, 0., -25. * scale),
            Vec3::new(x * scale + 0.25 * scale, 4., 25. * scale),
        ));
        colliders.push(Aabb::new(
            Vec3::new(-25. * scale, 0., x * scale - 0.25 * scale),
            Vec3::new(25. * scale, 4., x * scale + 0.25 * scale),
        ));
    }
    for x in [-12., 0., 12.] {
        for z in [-8., 8.] {
            colliders.push(Aabb::new(
                Vec3::new((x - 1.5) * scale, 0., (z - 1.5) * scale),
                Vec3::new((x + 1.5) * scale, 2., (z + 1.5) * scale),
            ));
        }
    }
    colliders
}
