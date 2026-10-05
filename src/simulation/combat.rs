use crate::{
    physics::Aabb,
    simulation::bot::{horizontal_distance, line_of_sight_blocked, player_eye_position},
};
use glam::Vec3;

pub use crate::simulation::bot::{Bot, BotBehavior, BOT_MIN_RESPAWN_DISTANCE, BOT_RESPAWN_SECONDS};
pub type Target = Bot;

const RESPAWN_POINTS: [Vec3; 12] = [
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
        let count = count.clamp(1, 10) as usize;
        self.targets = (0..count)
            .map(|index| {
                let position = INITIAL_POSITIONS
                    .get(index)
                    .copied()
                    .unwrap_or(RESPAWN_POINTS[index % RESPAWN_POINTS.len()]);
                let mut bot = Bot::with_behavior(
                    position,
                    behavior.unwrap_or(mixed[index % mixed.len()]),
                    index,
                );
                bot.hp = hp.clamp(50, 200);
                bot.speed_multiplier = speed_multiplier.clamp(0.5, 2.0);
                bot.respawn_seconds = respawn_seconds.clamp(0.5, 10.0);
                bot
            })
            .collect();
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
            .map(|offset| (self.spawn_cursor + offset) % RESPAWN_POINTS.len())
            .filter_map(|index| {
                let position = RESPAWN_POINTS[index];
                (horizontal_distance(position, player_position) >= BOT_MIN_RESPAWN_DISTANCE
                    && self.targets[bot_index].can_spawn_at(position, bot_index, walls, positions))
                .then_some((index, position))
            })
            .min_by_key(|(index, position)| {
                let visible = !line_of_sight_blocked(eye, *position + Vec3::Y, walls);
                (
                    visible,
                    (horizontal_distance(*position, player_position) - 14.0).abs() as u32,
                    *index,
                )
            })
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
            for (part, bounds) in bot.boxes().iter().enumerate() {
                if let Some(d) = bounds.ray_distance(origin, direction) {
                    if d < distance {
                        distance = d;
                        impact = true;
                        target = Some((i, part == 1));
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
}
