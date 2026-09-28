use crate::physics::Aabb;
use glam::{Mat4, Vec3};

/// 視点位置を基準とする一人称プレイヤー。衝突はカプセル相当をAABBで近似する。
pub const EYE_HEIGHT: f32 = 1.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_RADIUS: f32 = 0.3;
pub const WALK_SPEED: f32 = 4.0;
pub const DASH_SPEED: f32 = 6.5;
pub const JUMP_VELOCITY: f32 = 4.2;
pub const GRAVITY: f32 = 9.8;
/// 衝突判定の丸め誤差許容。接触距離の拡大ではなく、f32精度の境界での誤接触を防ぐ。
const COLLISION_EPSILON: f32 = 1e-4;

/// 1フレームの入力状態。
#[derive(Debug, Default, Clone, Copy)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub back: bool,
    pub dash: bool,
    pub jump: bool,
}

pub struct Player {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: Vec3,
    pub on_ground: bool,
}
impl Default for Player {
    fn default() -> Self {
        Self {
            position: Vec3::new(0., EYE_HEIGHT, 0.),
            yaw: 0.,
            pitch: 0.,
            velocity: Vec3::ZERO,
            on_ground: true,
        }
    }
}
impl Player {
    pub fn new() -> Self {
        Self::default()
    }
    /// 足元の高さ。接地時は0。視点位置と視点高から算出する。
    pub fn foot_height(&self) -> f32 {
        self.position.y - EYE_HEIGHT
    }
    /// 衝突判定で使う身体中心。足元から身体高さの半分の位置。
    pub fn body_center(&self) -> Vec3 {
        Vec3::new(
            self.position.x,
            self.foot_height() + PLAYER_HEIGHT * 0.5,
            self.position.z,
        )
    }
    /// 相対角度を視点へ反映する。感度倍率は呼び出し側で掛ける。
    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw += dx;
        self.pitch = (self.pitch - dy).clamp(-1.55, 1.55);
    }
    /// 視点位置とyaw/pitchから投影×ビュー行列を生成する。
    pub fn view_matrix(&self, aspect: f32) -> Mat4 {
        let eye = self.position;
        let target = eye
            + Vec3::new(
                self.yaw.sin() * self.pitch.cos(),
                self.pitch.sin(),
                -self.yaw.cos() * self.pitch.cos(),
            );
        let projection = Mat4::perspective_rh(70_f32.to_radians(), aspect, 0.1, 100.);
        projection * Mat4::look_at_rh(eye, target, Vec3::Y)
    }
    /// 入力から水平速度を決めて位置を進める。前方はyawの向き、斜め移動は正規化する。
    pub fn move_with_input(&mut self, input: Input, dt: f32) {
        let horizontal = self.horizontal(input);
        self.velocity.x = horizontal.x;
        self.velocity.z = horizontal.z;
        if input.jump && self.on_ground {
            self.velocity.y = JUMP_VELOCITY;
            self.on_ground = false;
        }
        if !self.on_ground {
            self.velocity.y -= GRAVITY * dt;
        }
        self.position += self.velocity * dt;
        if self.position.y <= EYE_HEIGHT {
            self.position.y = EYE_HEIGHT;
            self.velocity.y = 0.;
            self.on_ground = true;
        }
    }
    fn horizontal(&self, input: Input) -> Vec3 {
        let speed = if input.dash { DASH_SPEED } else { WALK_SPEED };
        let mut input_dir = Vec3::ZERO;
        if input.forward {
            input_dir.z -= 1.;
        }
        if input.back {
            input_dir.z += 1.;
        }
        if input.right && !input.left {
            input_dir.x += 1.;
        }
        if input.left && !input.right {
            input_dir.x -= 1.;
        }
        let input_dir = if input_dir.length_squared() > 0. {
            input_dir.normalize()
        } else {
            input_dir
        };
        let forward = Vec3::new(self.yaw.sin(), 0., -self.yaw.cos());
        let right_vec = Vec3::new(self.yaw.cos(), 0., self.yaw.sin());
        let move_dir = forward * -input_dir.z + right_vec * input_dir.x;
        Vec3::new(move_dir.x * speed, 0., move_dir.z * speed)
    }
    /// 入力とAABB衝突を考慮して移動する。水平方向は軸ごとにスライディングする。
    pub fn move_with_collision(&mut self, colliders: &[Aabb], input: Input, dt: f32) {
        let horizontal = self.horizontal(input);
        if input.jump && self.on_ground {
            self.velocity.y = JUMP_VELOCITY;
            self.on_ground = false;
        }
        if !self.on_ground {
            self.velocity.y -= GRAVITY * dt;
        }
        // 水平は軸ごとに移動して衝突を解決し、残った方向へ滑る。
        for axis in [0, 2] {
            self.position[axis] += horizontal[axis] * dt;
            self.velocity[axis] = horizontal[axis];
            if Self::collides(colliders, self.body_center()) {
                self.position[axis] -= horizontal[axis] * dt;
                self.velocity[axis] = 0.;
            }
        }
        self.position.y += self.velocity.y * dt;
        if Self::collides(colliders, self.body_center()) {
            self.position.y -= self.velocity.y * dt;
            // 落下中の衝突は着地。上昇中の衝突は天井に当たった場合。
            if self.velocity.y < 0. {
                self.on_ground = true;
            }
            self.velocity.y = 0.;
        }
        if self.position.y <= EYE_HEIGHT {
            self.position.y = EYE_HEIGHT;
            self.velocity.y = 0.;
            self.on_ground = true;
        }
    }
    fn collides(colliders: &[Aabb], center: Vec3) -> bool {
        let half = PLAYER_HEIGHT * 0.5;
        colliders.iter().any(|c| {
            let cy_min = center.y - half;
            let cy_max = center.y + half;
            if cy_max < c.min.y || cy_min > c.max.y {
                return false;
            }
            let cx = center.x.clamp(c.min.x, c.max.x);
            let cz = center.z.clamp(c.min.z, c.max.z);
            let dx = center.x - cx;
            let dz = center.z - cz;
            // f32の丸め誤差（壁面からちょうど半径の距離）で接触扱いになるのを防ぐ。
            let r = PLAYER_RADIUS - COLLISION_EPSILON;
            dx * dx + dz * dz < r * r
        })
    }
}
