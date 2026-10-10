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
                bot.hp = hp.clamp(50, 200);
                bot.speed_multiplier = speed_multiplier.clamp(0.5, 2.0);
                bot.respawn_seconds = respawn_seconds.clamp(0.5, 10.0);
                bot
            })
            .collect();
    }

    pub fn configure_tracking_target(
        &mut self,
        hp: u32,
        speed_multiplier: f32,
        respawn_seconds: f32,
    ) {
        self.arena_scale = 1.0;
        let mut target =
            Bot::with_behavior(Vec3::new(0.0, 0.0, -12.0), BotBehavior::TrackingStrafe, 1);
        target.hp = hp.clamp(50, 200);
        target.speed_multiplier = speed_multiplier.clamp(0.5, 2.0);
        target.respawn_seconds = respawn_seconds.clamp(0.5, 10.0);
        self.targets = vec![target];
        self.last_shot = Shot::default();
        self.feedback = 0.0;
        self.shots = 0;
        self.hits = 0;
        self.headshots = 0;
        self.kills = 0;
        self.spawn_cursor = 0;
    }

    pub fn advance(&mut self, dt: f32, player_position: Vec3, walls: &[Aabb]) {
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
            bot.advance_motion(index, dt, player_position, walls, &positions);
            positions[index] = bot.alive.then_some(bot.position);
        }

        for index in 0..self.targets.len() {
            if self.targets[index].alive || self.targets[index].respawn_remaining > 0.0 {
                continue;
            }
            if let Some((spawn_index, position)) =
                self.choose_respawn(index, player_position, walls, &positions)
            {
                self.targets[index].respawn(position);
                positions[index] = Some(position);
                self.spawn_cursor = (spawn_index + 1) % RESPAWN_POINTS.len();
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
