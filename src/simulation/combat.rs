use crate::{
    physics::Aabb,
    simulation::bot::{horizontal_distance, line_of_sight_blocked, player_eye_position},
};
use glam::Vec3;

pub use crate::simulation::bot::{Bot, BotBehavior, BOT_MIN_RESPAWN_DISTANCE, BOT_RESPAWN_SECONDS};
pub type Target = Bot;

const RESPAWN_POINTS: [Vec3; 36] = [
    Vec3::new(-20.0, 0.0, -20.0),
    Vec3::new(0.0, 0.0, -20.0),
    Vec3::new(20.0, 0.0, -20.0),
    Vec3::new(-20.0, 0.0, 20.0),
    Vec3::new(20.0, 0.0, 20.0),
    Vec3::new(0.0, 0.0, 20.0),
    Vec3::new(-20.0, 0.0, 0.0),
    Vec3::new(20.0, 0.0, 0.0),
    Vec3::new(-20.0, 0.0, -12.0),
    Vec3::new(20.0, 0.0, -12.0),
    Vec3::new(-20.0, 0.0, 12.0),
    Vec3::new(20.0, 0.0, 12.0),
    Vec3::new(-12.0, 0.0, -20.0),
    Vec3::new(12.0, 0.0, -20.0),
    Vec3::new(-12.0, 0.0, 20.0),
    Vec3::new(12.0, 0.0, 20.0),
    Vec3::new(-12.0, 0.0, -12.0),
    Vec3::new(12.0, 0.0, -12.0),
    Vec3::new(-12.0, 0.0, 12.0),
    Vec3::new(12.0, 0.0, 12.0),
    Vec3::new(0.0, 0.0, -12.0),
    Vec3::new(0.0, 0.0, 12.0),
    Vec3::new(-12.0, 0.0, 0.0),
    Vec3::new(12.0, 0.0, 0.0),
    Vec3::new(-22.0, 0.0, -22.0),
    Vec3::new(0.0, 0.0, -22.0),
    Vec3::new(22.0, 0.0, -22.0),
    Vec3::new(-22.0, 0.0, 22.0),
    Vec3::new(0.0, 0.0, 22.0),
    Vec3::new(22.0, 0.0, 22.0),
    Vec3::new(-22.0, 0.0, -11.0),
    Vec3::new(22.0, 0.0, -11.0),
    Vec3::new(-22.0, 0.0, 11.0),
    Vec3::new(22.0, 0.0, 11.0),
    Vec3::new(-11.0, 0.0, -22.0),
    Vec3::new(11.0, 0.0, -22.0),
];
const TRACKING_SPAWN_RADII: [f32; 3] = [5.5, 8.5, 11.0];
const TRACKING_SPAWN_MIN_X: f32 = -16.5;
const TRACKING_SPAWN_MAX_X: f32 = 16.5;
const TRACKING_SPAWN_MIN_Z: f32 = -10.0;
const TRACKING_SPAWN_MAX_Z: f32 = 10.0;
const TRACKING_SPAWN_MIN_SEPARATION: f32 = 2.5;

fn tracking_spawn_candidates(player_position: Vec3) -> Vec<Vec3> {
    let mut candidates = Vec::with_capacity(TRACKING_SPAWN_RADII.len() * 24);
    for radius in TRACKING_SPAWN_RADII {
        for step in 0..24 {
            let angle = std::f32::consts::TAU * step as f32 / 24.0;
            let position = Vec3::new(
                player_position.x + angle.cos() * radius,
                0.0,
                player_position.z + angle.sin() * radius,
            );
            let inside_range = (TRACKING_SPAWN_MIN_X..=TRACKING_SPAWN_MAX_X).contains(&position.x)
                && (TRACKING_SPAWN_MIN_Z..=TRACKING_SPAWN_MAX_Z).contains(&position.z);
            if inside_range
                && candidates
                    .iter()
                    .all(|candidate: &Vec3| horizontal_distance(*candidate, position) > 1.0)
            {
                candidates.push(position);
            }
        }
    }
    candidates
}

fn tracking_spawn_positions(player_position: Vec3, count: usize) -> Vec<Vec3> {
    let candidates = tracking_spawn_candidates(player_position);
    let mut selected = Vec::with_capacity(count);
    while selected.len() < count {
        let best = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                selected.iter().all(|existing| {
                    horizontal_distance(*existing, **candidate) >= TRACKING_SPAWN_MIN_SEPARATION
                })
            })
            .max_by(|(_, left), (_, right)| {
                let separation = |candidate: &Vec3| {
                    selected
                        .iter()
                        .map(|existing| horizontal_distance(*existing, *candidate))
                        .fold(f32::INFINITY, f32::min)
                };
                separation(left).total_cmp(&separation(right))
            })
            .map(|(_, candidate)| *candidate)
            .or_else(|| {
                candidates.iter().copied().find(|candidate| {
                    selected
                        .iter()
                        .all(|existing| horizontal_distance(*existing, *candidate) >= 1.0)
                })
            });
        let Some(position) = best else {
            break;
        };
        selected.push(position);
    }
    selected
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Shot {
    pub position: Vec3,
    pub impact: bool,
    pub hit: bool,
    pub headshot: bool,
    pub killed: bool,
}

pub struct Combat {
    pub targets: Vec<Bot>,
    pub last_shot: Shot,
    pub feedback: f32,
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    pub kills: u32,
    pub spawn_cursor: usize,
    pub arena_scale: f32,
}

impl Default for Combat {
    fn default() -> Self {
        Self::new(vec![
            Bot::with_behavior(Vec3::new(0.0, 0.0, -6.0), BotBehavior::Static, 0),
            Bot::with_behavior(Vec3::new(-7.0, 0.0, -11.0), BotBehavior::Strafe, 1),
            Bot::with_behavior(Vec3::new(7.0, 0.0, -14.0), BotBehavior::Chase, 2),
            Bot::with_behavior(Vec3::new(-18.0, 0.0, 14.0), BotBehavior::Strafe, 3),
            Bot::with_behavior(Vec3::new(18.0, 0.0, 14.0), BotBehavior::Chase, 4),
        ])
    }
}

impl Combat {
    pub fn new(targets: Vec<Bot>) -> Self {
        Self {
            targets,
            last_shot: Shot::default(),
            feedback: 0.0,
            shots: 0,
            hits: 0,
            headshots: 0,
            kills: 0,
            spawn_cursor: 0,
            arena_scale: 1.0,
        }
    }

    pub fn configure_bots(&mut self, count: u32, behavior_setting: u8) {
        self.configure_bots_with(count, behavior_setting, 100, 1.0, BOT_RESPAWN_SECONDS);
    }

    pub fn configure_bots_with(
        &mut self,
        count: u32,
        behavior_setting: u8,
        hp: u32,
        speed_multiplier: f32,
        respawn_seconds: f32,
    ) {
        self.configure_bots_with_arena_scale(
            count,
            behavior_setting,
            hp,
            speed_multiplier,
            respawn_seconds,
            1.0,
        );
    }

    pub fn configure_bots_with_arena_scale(
        &mut self,
        count: u32,
        behavior_setting: u8,
        hp: u32,
        speed_multiplier: f32,
        respawn_seconds: f32,
        arena_scale: f32,
    ) {
        self.arena_scale = if arena_scale.is_finite() {
            arena_scale.clamp(0.68, 2.2)
        } else {
            1.0
        };
        const INITIAL_POSITIONS: [Vec3; 5] = [
            Vec3::new(0.0, 0.0, -6.0),
            Vec3::new(-7.0, 0.0, -11.0),
            Vec3::new(7.0, 0.0, -14.0),
            Vec3::new(-18.0, 0.0, 14.0),
            Vec3::new(18.0, 0.0, 14.0),
        ];
        let mixed = [
            BotBehavior::Static,
            BotBehavior::Strafe,
            BotBehavior::Chase,
            BotBehavior::Strafe,
            BotBehavior::Chase,
        ];
        let behavior = match behavior_setting {
            0 => Some(BotBehavior::Static),
            1 => Some(BotBehavior::Strafe),
            2 => Some(BotBehavior::Chase),
            _ => None,
        };
        let count = count.clamp(1, 20) as usize;
        self.targets = (0..count)
            .map(|index| {
                let position = INITIAL_POSITIONS
                    .get(index)
                    .copied()
                    .unwrap_or(RESPAWN_POINTS[index % RESPAWN_POINTS.len()]);
                let position = Vec3::new(
                    position.x * self.arena_scale,
                    position.y,
                    position.z * self.arena_scale,
                );
                let mut bot = Bot::with_behavior(
                    position,
                    behavior.unwrap_or(mixed[index % mixed.len()]),
                    index,
                );
                bot.arena_scale = self.arena_scale;
                bot.set_max_hp(hp.clamp(50, 200));
                bot.speed_multiplier = speed_multiplier.clamp(0.5, 2.0);
                bot.respawn_seconds = respawn_seconds.clamp(0.5, 10.0);
                bot
            })
            .collect();
    }

    pub fn configure_tracking_bots(
        &mut self,
        count: u32,
        hp: u32,
        speed_multiplier: f32,
        respawn_seconds: f32,
        player_position: Vec3,
    ) {
        let tracking_spawn_count = count.clamp(1, 5) as usize;
        self.arena_scale = 1.0;
        self.targets = tracking_spawn_positions(player_position, tracking_spawn_count)
            .into_iter()
            .enumerate()
            .map(|(index, position)| {
                let mut bot = Bot::with_behavior(position, BotBehavior::TrackingStrafe, index);
                bot.set_max_hp(hp.clamp(50, 200).saturating_mul(3));
                bot.speed_multiplier = speed_multiplier.clamp(0.5, 2.0);
                bot.respawn_seconds = respawn_seconds.clamp(0.5, 10.0);
                bot.face_toward_player(player_position);
                bot
            })
            .collect();
        self.last_shot = Shot::default();
        self.feedback = 0.0;
        self.shots = 0;
        self.hits = 0;
        self.headshots = 0;
        self.kills = 0;
        self.spawn_cursor = 0;
    }

    pub fn advance(&mut self, dt: f32, player_position: Vec3, walls: &[Aabb]) {
        self.advance_with_view(dt, player_position, Vec3::X, walls);
    }

    pub fn advance_with_view(
        &mut self,
        dt: f32,
        player_position: Vec3,
        view_right: Vec3,
        walls: &[Aabb],
    ) {
        let tracking_aim = !self.targets.is_empty()
            && self
                .targets
                .iter()
                .all(|bot| bot.behavior == BotBehavior::TrackingStrafe);
        self.feedback = (self.feedback - dt).max(0.0);
        for bot in &mut self.targets {
            bot.advance_timers(dt);
        }

        let mut positions = self
            .targets
            .iter()
            .map(|bot| bot.alive.then_some(bot.position))
            .collect::<Vec<_>>();
        for (index, bot) in self.targets.iter_mut().enumerate() {
            if tracking_aim && bot.alive {
                bot.align_tracking_to_player(player_position, view_right);
            }
            bot.advance_motion(index, dt, player_position, walls, &positions);
            positions[index] = bot.alive.then_some(bot.position);
        }

        for index in 0..self.targets.len() {
            if self.targets[index].alive || self.targets[index].respawn_remaining > 0.0 {
                continue;
            }
            let respawn = if tracking_aim {
                self.choose_tracking_respawn(index, player_position, walls, &positions)
            } else {
                self.choose_respawn(index, player_position, walls, &positions)
            };
            if let Some((spawn_index, position)) = respawn {
                self.targets[index].respawn(position);
                if tracking_aim {
                    self.targets[index].align_tracking_to_player(player_position, view_right);
                }
                positions[index] = Some(position);
                self.spawn_cursor = if tracking_aim {
                    spawn_index + 1
                } else {
                    (spawn_index + 1) % RESPAWN_POINTS.len()
                };
            }
        }
    }

    fn choose_respawn(
        &self,
        bot_index: usize,
        player_position: Vec3,
        walls: &[Aabb],
        positions: &[Option<Vec3>],
    ) -> Option<(usize, Vec3)> {
        let eye = player_eye_position(player_position);
        (0..RESPAWN_POINTS.len())
            .map(|offset| ((self.spawn_cursor + offset) % RESPAWN_POINTS.len(), offset))
            .filter_map(|index| {
                let (index, offset) = index;
                let point = RESPAWN_POINTS[index];
                let position = Vec3::new(
                    point.x * self.arena_scale,
                    point.y,
                    point.z * self.arena_scale,
                );
                (horizontal_distance(position, player_position) >= BOT_MIN_RESPAWN_DISTANCE
                    && self.targets[bot_index].can_spawn_at(position, bot_index, walls, positions))
                .then_some((index, offset, position))
            })
            .min_by_key(|(_, offset, position)| {
                let visible = !line_of_sight_blocked(eye, *position + Vec3::Y, walls);
                (visible, *offset)
            })
            .map(|(index, _, position)| (index, position))
    }

    fn choose_tracking_respawn(
        &self,
        bot_index: usize,
        player_position: Vec3,
        walls: &[Aabb],
        positions: &[Option<Vec3>],
    ) -> Option<(usize, Vec3)> {
        let candidates = tracking_spawn_candidates(player_position);
        if candidates.is_empty() {
            return None;
        }
        let mut best_spaced = None;
        let mut best_fallback = None;
        for offset in 0..candidates.len() {
            let index = (self.spawn_cursor + offset) % candidates.len();
            let position = candidates[index];
            if !self.targets[bot_index].can_spawn_at(position, bot_index, walls, positions) {
                continue;
            }
            let nearest_bot = positions
                .iter()
                .enumerate()
                .filter(|(other_index, _)| *other_index != bot_index)
                .filter_map(|(_, other)| *other)
                .map(|other| horizontal_distance(position, other))
                .fold(f32::INFINITY, f32::min);
            let candidate = (index, position, nearest_bot);
            if best_fallback.is_none_or(|(_, _, best_distance)| nearest_bot > best_distance) {
                best_fallback = Some(candidate);
            }
            if nearest_bot >= TRACKING_SPAWN_MIN_SEPARATION
                && best_spaced.is_none_or(|(_, _, best_distance)| nearest_bot > best_distance)
            {
                best_spaced = Some(candidate);
            }
        }
        best_spaced
            .or(best_fallback)
            .map(|(index, position, _)| (index, position))
    }

    pub fn shoot(&mut self, origin: Vec3, direction: Vec3, walls: &[Aabb], damage: u32) -> Shot {
        let direction = direction.normalize_or_zero();
        let mut distance = 100.0;
        let mut impact = false;
        for wall in walls {
            if let Some(d) = wall.ray_distance(origin, direction) {
                if d < distance {
                    distance = d;
                    impact = true;
                }
            }
        }
        let mut target = None;
        for (i, bot) in self.targets.iter().enumerate() {
            if !bot.alive {
                continue;
            }
            for hitbox in bot.hitboxes() {
                if let Some(d) = hitbox.ray_distance(origin, direction) {
                    if d < distance {
                        distance = d;
                        impact = true;
                        target = Some((i, hitbox.headshot));
                    }
                }
            }
        }
        let mut shot = Shot {
            position: origin + direction * distance,
            impact,
            ..Shot::default()
        };
        if let Some((index, head)) = target {
            let bot = &mut self.targets[index];
            shot.hit = true;
            shot.headshot = head;
            shot.killed = bot.apply_damage(damage * if head { 2 } else { 1 });
            self.hits += 1;
            self.headshots += u32::from(head);
            self.kills += u32::from(shot.killed);
        }
        self.shots += 1;
        self.last_shot = shot;
        self.feedback = 0.12;
        shot
    }

    /// カメラの照準点へ銃口から射線を引き、銃口視差を補正しつつ遮蔽物を維持する。
    pub fn shoot_from_muzzle(
        &mut self,
        camera_origin: Vec3,
        muzzle_origin: Vec3,
        aim_direction: Vec3,
        walls: &[Aabb],
        damage: u32,
    ) -> Shot {
        self.shoot_from_muzzle_with_aim(
            camera_origin,
            muzzle_origin,
            aim_direction,
            aim_direction,
            walls,
            damage,
        )
    }

    /// Only damages a target selected by the reticle ray and physically intersected by the shot.
    pub fn shoot_from_muzzle_with_aim(
        &mut self,
        camera_origin: Vec3,
        muzzle_origin: Vec3,
        aim_direction: Vec3,
        shot_direction: Vec3,
        walls: &[Aabb],
        damage: u32,
    ) -> Shot {
        let aim_direction = aim_direction.normalize_or_zero();
        let shot_direction = shot_direction.normalize_or_zero();
        let mut aim_distance: f32 = 100.0;
        let mut aimed_target = None;
        let mut aim_blocked_by_wall = false;
        for wall in walls {
            if let Some(distance) = wall.ray_distance(camera_origin, aim_direction) {
                if distance < aim_distance {
                    aim_distance = distance;
                    aimed_target = None;
                    aim_blocked_by_wall = true;
                }
            }
        }
        for (index, bot) in self.targets.iter().enumerate() {
            if bot.alive {
                for hitbox in bot.hitboxes() {
                    if let Some(distance) = hitbox.ray_distance(camera_origin, aim_direction) {
                        if distance < aim_distance {
                            aim_distance = distance;
                            aimed_target = Some(index);
                            aim_blocked_by_wall = false;
                        }
                    }
                }
            }
        }
        let endpoint_direction = if aim_blocked_by_wall {
            aim_direction
        } else {
            shot_direction
        };
        let aim_point = camera_origin + endpoint_direction * aim_distance;
        let muzzle_delta = aim_point - muzzle_origin;
        let muzzle_distance = muzzle_delta.length();
        let muzzle_direction = muzzle_delta.normalize_or_zero();

        let wall_hit = walls
            .iter()
            .filter_map(|wall| wall.ray_distance(muzzle_origin, muzzle_direction))
            .filter(|distance| *distance < muzzle_distance - 1e-3)
            .min_by(f32::total_cmp);
        let physical_target_hit = self
            .targets
            .iter()
            .enumerate()
            .filter(|(_, bot)| bot.alive)
            .flat_map(|(index, bot)| {
                bot.hitboxes().into_iter().filter_map(move |hitbox| {
                    hitbox
                        .ray_distance(muzzle_origin, muzzle_direction)
                        .filter(|distance| *distance <= muzzle_distance + 1e-3)
                        .map(|distance| (distance, index, hitbox.headshot))
                })
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));

        let mut shot = if aim_blocked_by_wall {
            Shot {
                position: aim_point,
                impact: true,
                ..Shot::default()
            }
        } else if let Some((wall_distance, target_hit)) = wall_hit.zip(physical_target_hit) {
            if wall_distance <= target_hit.0 {
                Shot {
                    position: muzzle_origin + muzzle_direction * wall_distance,
                    impact: true,
                    ..Shot::default()
                }
            } else {
                self.finish_muzzle_hit(
                    aimed_target,
                    target_hit,
                    muzzle_origin,
                    muzzle_direction,
                    damage,
                )
            }
        } else if let Some(distance) = wall_hit {
            Shot {
                position: muzzle_origin + muzzle_direction * distance,
                impact: true,
                ..Shot::default()
            }
        } else if let Some(target_hit) = physical_target_hit {
            self.finish_muzzle_hit(
                aimed_target,
                target_hit,
                muzzle_origin,
                muzzle_direction,
                damage,
            )
        } else {
            Shot {
                position: aim_point,
                ..Shot::default()
            }
        };
        if aim_blocked_by_wall {
            shot.position = aim_point;
            shot.impact = true;
        }
        self.shots += 1;
        self.last_shot = shot;
        self.feedback = 0.12;
        shot
    }

    fn finish_muzzle_hit(
        &mut self,
        aimed_target: Option<usize>,
        target_hit: (f32, usize, bool),
        muzzle_origin: Vec3,
        muzzle_direction: Vec3,
        damage: u32,
    ) -> Shot {
        let (distance, index, headshot) = target_hit;
        let selected_by_reticle = aimed_target == Some(index);
        let mut shot = Shot {
            position: muzzle_origin + muzzle_direction * distance,
            impact: selected_by_reticle,
            ..Shot::default()
        };
        if selected_by_reticle {
            shot.hit = true;
            shot.headshot = headshot;
            shot.killed = self.targets[index].apply_damage(damage * if headshot { 2 } else { 1 });
            self.hits += 1;
            self.headshots += u32::from(headshot);
            self.kills += u32::from(shot.killed);
        }
        shot
    }
}
