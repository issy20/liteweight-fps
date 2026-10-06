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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotBehavior {
    Static,
    Strafe,
    Chase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotHitbox {
    pub bounds: Aabb,
    pub headshot: bool,
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
            chase_waypoint: 0,
            move_direction: if behavior == BotBehavior::Strafe {
                Vec3::X * (BOT_STRAFE_SPEED * strafe_phase.cos())
            } else {
                Vec3::ZERO
            },
            arena_scale: 1.0,
        }
    }

    pub fn hitboxes(&self) -> [BotHitbox; 6] {
        let region = |min: Vec3, max: Vec3, headshot| BotHitbox {
            bounds: Aabb::new(self.position + min, self.position + max),
            headshot,
        };
        [
            region(
                Vec3::new(-0.24, 0.96, -0.15),
                Vec3::new(0.24, 1.67, 0.15),
                false,
            ),
            region(
                Vec3::new(-0.42, 0.67, -0.12),
                Vec3::new(-0.12, 1.60, 0.12),
                false,
            ),
            region(
                Vec3::new(0.12, 0.67, -0.12),
                Vec3::new(0.42, 1.60, 0.12),
                false,
            ),
            region(
                Vec3::new(-0.24, 0.0, -0.18),
                Vec3::new(-0.03, 1.02, 0.18),
                false,
            ),
            region(
                Vec3::new(0.03, 0.0, -0.18),
                Vec3::new(0.24, 1.02, 0.18),
                false,
            ),
            region(
                Vec3::new(-0.09, 1.77, -0.09),
                Vec3::new(0.09, 2.03, 0.09),
                true,
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
        let displacement = self.move_direction * (dt * self.speed_multiplier);
        for axis in [0, 2] {
            let mut candidate = self.position;
            candidate[axis] += displacement[axis];
            if !self.collides(candidate, index, walls, other_positions) {
                self.position = candidate;
            }
        }
    }

    fn think(&mut self, player_position: Vec3) {
        match self.behavior {
            BotBehavior::Static => self.move_direction = Vec3::ZERO,
            BotBehavior::Strafe => {}
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
        self.chase_waypoint = 0;
        self.move_direction = Vec3::ZERO;
    }
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
