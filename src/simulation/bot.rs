use crate::{physics::Aabb, simulation::player::EYE_HEIGHT};
use glam::Vec3;

pub const BOT_RESPAWN_SECONDS: f32 = 2.0;
pub const BOT_MIN_RESPAWN_DISTANCE: f32 = 8.0;
const BOT_RADIUS: f32 = 0.36;
const BOT_HEIGHT: f32 = 2.05;
const BOT_AI_INTERVAL: f32 = 1.0 / 20.0;
const BOT_STRAFE_SPEED: f32 = 2.2;
const BOT_CHASE_SPEED: f32 = 1.8;
const BOT_CHASE_STOP_DISTANCE: f32 = 4.5;
const BOT_STRAFE_ANGULAR_SPEED: f32 = 0.9;
const TRACKING_STRAFE_MIN_STEPS: u16 = 45;
const TRACKING_STRAFE_MAX_STEPS: u16 = 120;
const TRACKING_STRAFE_HALF_WIDTH: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotBehavior {
    Static,
    Strafe,
    TrackingStrafe,
    Chase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotHitbox {
    pub bounds: Aabb,
    pub headshot: bool,
    shape: HitboxShape,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum HitboxShape {
    Ellipsoid { center: Vec3, radii: Vec3 },
    Capsule { start: Vec3, end: Vec3, radius: f32 },
    LoftedTorso { position: Vec3 },
}

impl BotHitbox {
    pub fn ray_distance(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        match self.shape {
            HitboxShape::Ellipsoid { center, radii } => {
                ray_ellipsoid_distance(origin, direction, center, radii)
            }
            HitboxShape::Capsule { start, end, radius } => {
                ray_capsule_distance(origin, direction, start, end, radius)
            }
            HitboxShape::LoftedTorso { position } => {
                ray_torso_distance(origin, direction, position)
            }
        }
    }
}

pub struct Bot {
    pub position: Vec3,
    pub hp: u32,
    pub flash: f32,
    pub alive: bool,
    pub behavior: BotBehavior,
    pub speed_multiplier: f32,
    pub respawn_seconds: f32,
    pub respawn_remaining: f32,
    spawn_slot: usize,
    ai_accumulator: f32,
    strafe_phase: f32,
    tracking_origin_x: f32,
    tracking_direction: f32,
    tracking_steps_remaining: u16,
    tracking_rng: u32,
    chase_waypoint: usize,
    move_direction: Vec3,
    pub arena_scale: f32,
}

impl Bot {
    pub fn new(position: Vec3) -> Self {
        Self::with_behavior(position, BotBehavior::Static, 0)
    }

    pub fn with_behavior(position: Vec3, behavior: BotBehavior, spawn_slot: usize) -> Self {
        let strafe_phase = if spawn_slot.is_multiple_of(2) {
            0.0
        } else {
            std::f32::consts::PI
        };
        let mut tracking_rng = (spawn_slot as u32)
            .wrapping_add(1)
            .wrapping_mul(0x9E37_79B9)
            .max(1);
        let tracking_steps_remaining = if behavior == BotBehavior::TrackingStrafe {
            random_tracking_stride(&mut tracking_rng)
        } else {
            0
        };
        let tracking_direction = if spawn_slot.is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        Self {
            position,
            hp: 100,
            flash: 0.0,
            alive: true,
            behavior,
            speed_multiplier: 1.0,
            respawn_seconds: BOT_RESPAWN_SECONDS,
            respawn_remaining: 0.0,
            spawn_slot,
            ai_accumulator: 0.0,
            strafe_phase,
            tracking_origin_x: position.x,
            tracking_direction,
            tracking_steps_remaining,
            tracking_rng,
            chase_waypoint: 0,
            move_direction: match behavior {
                BotBehavior::Strafe => Vec3::X * (BOT_STRAFE_SPEED * strafe_phase.cos()),
                BotBehavior::TrackingStrafe => Vec3::X * (BOT_STRAFE_SPEED * tracking_direction),
                BotBehavior::Static | BotBehavior::Chase => Vec3::ZERO,
            },
            arena_scale: 1.0,
        }
    }

    pub fn hitboxes(&self) -> [BotHitbox; 8] {
        let capsule = |min: Vec3, max: Vec3, start: Vec3, end: Vec3, radius| BotHitbox {
            bounds: Aabb::new(self.position + min, self.position + max),
            headshot: false,
            shape: HitboxShape::Capsule {
                start: self.position + start,
                end: self.position + end,
                radius,
            },
        };
        let ellipsoid = |center: Vec3, radii: Vec3, headshot| BotHitbox {
            bounds: Aabb::new(
                self.position + center - radii,
                self.position + center + radii,
            ),
            headshot,
            shape: HitboxShape::Ellipsoid {
                center: self.position + center,
                radii,
            },
        };
        [
            BotHitbox {
                bounds: Aabb::new(
                    self.position + Vec3::new(-0.24, 0.96, -0.15),
                    self.position + Vec3::new(0.24, 1.67, 0.15),
                ),
                headshot: false,
                shape: HitboxShape::LoftedTorso {
                    position: self.position,
                },
            },
            capsule(
                Vec3::new(-0.42, 0.67, -0.12),
                Vec3::new(-0.12, 1.60, 0.12),
                Vec3::new(-0.20, 1.52, 0.0),
                Vec3::new(-0.34, 0.77, 0.04),
                0.08,
            ),
            capsule(
                Vec3::new(0.12, 0.67, -0.12),
                Vec3::new(0.42, 1.60, 0.12),
                Vec3::new(0.20, 1.52, 0.0),
                Vec3::new(0.34, 0.77, 0.04),
                0.08,
            ),
            capsule(
                Vec3::new(-0.24, 0.0, -0.18),
                Vec3::new(-0.03, 1.02, 0.18),
                Vec3::new(-0.105, 0.96, 0.0),
                Vec3::new(-0.14, 0.12, 0.015),
                0.11,
            ),
            capsule(
                Vec3::new(0.03, 0.0, -0.18),
                Vec3::new(0.24, 1.02, 0.18),
                Vec3::new(0.105, 0.96, 0.0),
                Vec3::new(0.14, 0.12, 0.015),
                0.11,
            ),
            ellipsoid(
                Vec3::new(0.0, 1.90, 0.0),
                Vec3::new(0.085, 0.13, 0.082),
                true,
            ),
            ellipsoid(
                Vec3::new(-0.14, 0.045, 0.085),
                Vec3::new(0.075, 0.045, 0.17),
                false,
            ),
            ellipsoid(
                Vec3::new(0.14, 0.045, 0.085),
                Vec3::new(0.075, 0.045, 0.17),
                false,
            ),
        ]
    }

    pub fn apply_damage(&mut self, damage: u32) -> bool {
        if !self.alive {
            return false;
        }
        self.hp = self.hp.saturating_sub(damage);
        self.flash = 0.15;
        if self.hp == 0 {
            self.alive = false;
            self.respawn_remaining = self.respawn_seconds;
            self.move_direction = Vec3::ZERO;
            true
        } else {
            false
        }
    }

    pub fn advance_timers(&mut self, dt: f32) {
        self.flash = (self.flash - dt).max(0.0);
        if !self.alive {
            self.respawn_remaining = (self.respawn_remaining - dt).max(0.0);
        }
    }

    pub fn advance_motion(
        &mut self,
        index: usize,
        dt: f32,
        player_position: Vec3,
        walls: &[Aabb],
        other_positions: &[Option<Vec3>],
    ) {
        if !self.alive {
            return;
        }
        self.ai_accumulator += dt;
        while self.ai_accumulator >= BOT_AI_INTERVAL {
            self.ai_accumulator -= BOT_AI_INTERVAL;
            self.think(player_position);
        }
        if self.behavior == BotBehavior::Strafe {
            self.strafe_phase = (self.strafe_phase + BOT_STRAFE_ANGULAR_SPEED * dt)
                .rem_euclid(std::f32::consts::TAU);
            self.move_direction = Vec3::X * (BOT_STRAFE_SPEED * self.strafe_phase.cos());
        }
        if self.behavior == BotBehavior::TrackingStrafe {
            if self.tracking_steps_remaining == 0 {
                self.tracking_direction = -self.tracking_direction;
                self.tracking_steps_remaining = random_tracking_stride(&mut self.tracking_rng);
            }
            self.tracking_steps_remaining -= 1;
            let next_x = self.position.x
                + BOT_STRAFE_SPEED * self.tracking_direction * dt * self.speed_multiplier;
            let min_x = self.tracking_origin_x - TRACKING_STRAFE_HALF_WIDTH;
            let max_x = self.tracking_origin_x + TRACKING_STRAFE_HALF_WIDTH;
            if next_x < min_x || next_x > max_x {
                self.tracking_direction = -self.tracking_direction;
                self.tracking_steps_remaining = random_tracking_stride(&mut self.tracking_rng);
            }
            self.move_direction = Vec3::X * (BOT_STRAFE_SPEED * self.tracking_direction);
        }
        let displacement = self.move_direction * (dt * self.speed_multiplier);
        for axis in [0, 2] {
            let mut candidate = self.position;
            candidate[axis] += displacement[axis];
            if self.behavior == BotBehavior::TrackingStrafe && axis == 0 {
                candidate.x = candidate.x.clamp(
                    self.tracking_origin_x - TRACKING_STRAFE_HALF_WIDTH,
                    self.tracking_origin_x + TRACKING_STRAFE_HALF_WIDTH,
                );
            }
            if !self.collides(candidate, index, walls, other_positions) {
                self.position = candidate;
            }
        }
    }

    fn think(&mut self, player_position: Vec3) {
        match self.behavior {
            BotBehavior::Static => self.move_direction = Vec3::ZERO,
            BotBehavior::Strafe => {}
            BotBehavior::TrackingStrafe => {}
            BotBehavior::Chase => {
                let mut target = self.next_waypoint();
                if let Some(waypoint) = target {
                    if horizontal_distance(self.position, waypoint) < 1.0 {
                        self.chase_waypoint += 1;
                        target = self.next_waypoint();
                    }
                }
                let target = target.unwrap_or(player_position);
                let offset = horizontal(target - self.position);
                let distance = offset.length();
                if target == player_position && distance <= BOT_CHASE_STOP_DISTANCE {
                    self.move_direction = Vec3::ZERO;
                } else {
                    self.move_direction = offset.normalize_or_zero() * BOT_CHASE_SPEED;
                }
            }
        }
    }

    fn next_waypoint(&self) -> Option<Vec3> {
        match (self.spawn_slot, self.chase_waypoint) {
            (2, 0) => Some(Vec3::new(
                16.0 * self.arena_scale,
                0.0,
                -10.0 * self.arena_scale,
            )),
            (2, 1) => Some(Vec3::new(16.0 * self.arena_scale, 0.0, 0.0)),
            (4, 0) => Some(Vec3::new(18.0 * self.arena_scale, 0.0, 0.0)),
            (4, 1) => Some(Vec3::new(8.0 * self.arena_scale, 0.0, 0.0)),
            _ => None,
        }
    }

    fn collides(
        &self,
        position: Vec3,
        index: usize,
        walls: &[Aabb],
        other_positions: &[Option<Vec3>],
    ) -> bool {
        let center = position + Vec3::Y * (BOT_HEIGHT * 0.5);
        if walls
            .iter()
            .any(|wall| wall.max.y > 0.0 && wall.hits_cylinder(center, BOT_RADIUS, BOT_HEIGHT))
        {
            return true;
        }
        let minimum_distance_sq = (BOT_RADIUS * 2.0).powi(2);
        other_positions
            .iter()
            .enumerate()
            .any(|(other_index, other)| {
                other_index != index
                    && other.is_some_and(|other| {
                        horizontal_distance_sq(position, other) < minimum_distance_sq
                    })
            })
    }

    pub fn can_spawn_at(
        &self,
        position: Vec3,
        index: usize,
        walls: &[Aabb],
        other_positions: &[Option<Vec3>],
    ) -> bool {
        !self.collides(position, index, walls, other_positions)
    }

    pub fn respawn(&mut self, position: Vec3) {
        self.position = position;
        self.hp = 100;
        self.flash = 0.0;
        self.alive = true;
        self.respawn_remaining = 0.0;
        self.ai_accumulator = 0.0;
        self.strafe_phase = if self.spawn_slot.is_multiple_of(2) {
            0.0
        } else {
            std::f32::consts::PI
        };
        self.tracking_origin_x = position.x;
        self.tracking_steps_remaining = random_tracking_stride(&mut self.tracking_rng);
        self.chase_waypoint = 0;
        self.move_direction = Vec3::ZERO;
    }
}

fn random_tracking_stride(state: &mut u32) -> u16 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    TRACKING_STRAFE_MIN_STEPS
        + (*state as u16 % (TRACKING_STRAFE_MAX_STEPS - TRACKING_STRAFE_MIN_STEPS + 1))
}

const TORSO_PROFILE: [(f32, f32, f32); 6] = [
    (0.96, 0.16, 0.12),
    (1.06, 0.19, 0.135),
    (1.17, 0.14, 0.112),
    (1.38, 0.21, 0.15),
    (1.50, 0.24, 0.14),
    (1.67, 0.16, 0.10),
];
const TORSO_RING_POINTS: [Vec3; 6] = [
    Vec3::new(1.0, 0.0, 0.0),
    Vec3::new(0.5, 0.0, 0.866_025_4),
    Vec3::new(-0.5, 0.0, 0.866_025_4),
    Vec3::new(-1.0, 0.0, 0.0),
    Vec3::new(-0.5, 0.0, -0.866_025_4),
    Vec3::new(0.5, 0.0, -0.866_025_4),
];

fn ray_torso_distance(origin: Vec3, direction: Vec3, position: Vec3) -> Option<f32> {
    if !origin.is_finite() || !direction.is_finite() || direction.length_squared() < 1e-12 {
        return None;
    }
    let direction = direction.normalize();
    let vertex = |ring: usize, side: usize| {
        let (y, radius_x, radius_z) = TORSO_PROFILE[ring];
        let point = TORSO_RING_POINTS[side];
        position + Vec3::new(point.x * radius_x, y, point.z * radius_z)
    };
    let bottom = position + Vec3::new(0.0, TORSO_PROFILE[0].0, 0.0);
    let top = position + Vec3::new(0.0, TORSO_PROFILE[TORSO_PROFILE.len() - 1].0, 0.0);
    let mut closest = None;

    for ring in 0..(TORSO_PROFILE.len() - 1) {
        for side in 0..TORSO_RING_POINTS.len() {
            let next = (side + 1) % TORSO_RING_POINTS.len();
            let lower = vertex(ring, side);
            let lower_next = vertex(ring, next);
            let upper = vertex(ring + 1, side);
            let upper_next = vertex(ring + 1, next);
            for distance in [
                ray_triangle_distance(origin, direction, lower, lower_next, upper_next),
                ray_triangle_distance(origin, direction, lower, upper_next, upper),
            ]
            .into_iter()
            .flatten()
            {
                closest = Some(closest.map_or(distance, |current: f32| current.min(distance)));
            }
        }
    }

    for side in 0..TORSO_RING_POINTS.len() {
        let next = (side + 1) % TORSO_RING_POINTS.len();
        for distance in [
            ray_triangle_distance(origin, direction, bottom, vertex(0, next), vertex(0, side)),
            ray_triangle_distance(
                origin,
                direction,
                top,
                vertex(TORSO_PROFILE.len() - 1, side),
                vertex(TORSO_PROFILE.len() - 1, next),
            ),
        ]
        .into_iter()
        .flatten()
        {
            closest = Some(closest.map_or(distance, |current: f32| current.min(distance)));
        }
    }
    closest
}

fn ray_triangle_distance(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let edge_ab = b - a;
    let edge_ac = c - a;
    let p = direction.cross(edge_ac);
    let determinant = edge_ab.dot(p);
    if determinant.abs() < 1e-8 {
        return None;
    }
    let inverse_determinant = determinant.recip();
    let from_a = origin - a;
    let u = from_a.dot(p) * inverse_determinant;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = from_a.cross(edge_ab);
    let v = direction.dot(q) * inverse_determinant;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge_ac.dot(q) * inverse_determinant;
    (distance >= 0.0).then_some(distance)
}

fn ray_ellipsoid_distance(origin: Vec3, direction: Vec3, center: Vec3, radii: Vec3) -> Option<f32> {
    if !origin.is_finite()
        || !direction.is_finite()
        || direction.length_squared() < 1e-12
        || radii.min_element() <= 0.0
    {
        return None;
    }
    let direction = direction.normalize();
    let relative_origin = (origin - center) / radii;
    let scaled_direction = direction / radii;
    let a = scaled_direction.length_squared();
    let b = relative_origin.dot(scaled_direction);
    let c = relative_origin.length_squared() - 1.0;
    if c <= 0.0 {
        return Some(0.0);
    }
    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = (-b - root) / a;
    if near >= 0.0 {
        Some(near)
    } else {
        let far = (-b + root) / a;
        (far >= 0.0).then_some(far)
    }
}

fn ray_capsule_distance(
    origin: Vec3,
    direction: Vec3,
    start: Vec3,
    end: Vec3,
    radius: f32,
) -> Option<f32> {
    if !origin.is_finite()
        || !direction.is_finite()
        || direction.length_squared() < 1e-12
        || !radius.is_finite()
        || radius <= 0.0
    {
        return None;
    }
    let direction = direction.normalize();
    let axis = end - start;
    let axis_length_sq = axis.length_squared();
    if axis_length_sq < 1e-12 {
        return ray_ellipsoid_distance(origin, direction, start, Vec3::splat(radius));
    }

    let from_start = origin - start;
    let axis_dot_direction = axis.dot(direction);
    let axis_dot_origin = axis.dot(from_start);
    let direction_dot_origin = direction.dot(from_start);
    let origin_length_sq = from_start.length_squared();
    let quadratic_a = axis_length_sq - axis_dot_direction * axis_dot_direction;
    let quadratic_b = axis_length_sq * direction_dot_origin - axis_dot_origin * axis_dot_direction;
    let quadratic_c = axis_length_sq * origin_length_sq
        - axis_dot_origin * axis_dot_origin
        - radius * radius * axis_length_sq;
    let discriminant = quadratic_b * quadratic_b - quadratic_a * quadratic_c;

    let cylinder_hit = if quadratic_a > 1e-12 && discriminant >= 0.0 {
        let root = discriminant.sqrt();
        [
            (-quadratic_b - root) / quadratic_a,
            (-quadratic_b + root) / quadratic_a,
        ]
        .into_iter()
        .filter(|distance| *distance >= 0.0)
        .filter(|distance| {
            let along_axis = axis_dot_origin + *distance * axis_dot_direction;
            (0.0..=axis_length_sq).contains(&along_axis)
        })
        .min_by(f32::total_cmp)
    } else {
        None
    };
    [
        cylinder_hit,
        ray_ellipsoid_distance(origin, direction, start, Vec3::splat(radius)),
        ray_ellipsoid_distance(origin, direction, end, Vec3::splat(radius)),
    ]
    .into_iter()
    .flatten()
    .min_by(f32::total_cmp)
}

pub fn horizontal_distance(a: Vec3, b: Vec3) -> f32 {
    horizontal(a - b).length()
}

fn horizontal(vector: Vec3) -> Vec3 {
    Vec3::new(vector.x, 0.0, vector.z)
}

fn horizontal_distance_sq(a: Vec3, b: Vec3) -> f32 {
    let delta = horizontal(a - b);
    delta.length_squared()
}

pub fn line_of_sight_blocked(origin: Vec3, target: Vec3, walls: &[Aabb]) -> bool {
    let delta = target - origin;
    let distance = delta.length();
    let direction = delta.normalize_or_zero();
    walls.iter().any(|wall| {
        wall.max.y > 0.0
            && wall
                .ray_distance(origin, direction)
                .is_some_and(|hit| hit < distance - 0.5)
    })
}

pub fn player_eye_position(position: Vec3) -> Vec3 {
    Vec3::new(position.x, EYE_HEIGHT, position.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_strafe_uses_random_step_counts_within_configured_bounds() {
        let mut rng = 0x9E37_79B9;
        let observed: Vec<_> = (0..8).map(|_| random_tracking_stride(&mut rng)).collect();
        assert!(observed
            .iter()
            .all(|steps| (TRACKING_STRAFE_MIN_STEPS..=TRACKING_STRAFE_MAX_STEPS).contains(steps)));
        assert!(observed.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn tracking_strafe_reverses_after_countdown_and_stays_in_lane() {
        let mut bot = Bot::with_behavior(Vec3::ZERO, BotBehavior::TrackingStrafe, 0);
        let initial_direction = bot.tracking_direction;
        let initial_steps = bot.tracking_steps_remaining;
        for _ in 0..=initial_steps {
            bot.advance_motion(0, 1.0 / 60.0, Vec3::ZERO, &[], &[]);
        }
        assert_ne!(bot.tracking_direction, initial_direction);

        for _ in 0..60 * 120 {
            bot.advance_motion(0, 1.0 / 60.0, Vec3::ZERO, &[], &[]);
            assert!(bot.position.x.abs() <= TRACKING_STRAFE_HALF_WIDTH + 1e-4);
        }
    }
}
