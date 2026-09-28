use crate::physics::Aabb;
use glam::Vec3;

pub struct Target {
    pub position: Vec3,
    pub hp: u32,
    pub flash: f32,
}
impl Target {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            hp: 100,
            flash: 0.0,
        }
    }
    pub fn boxes(&self) -> [Aabb; 2] {
        [
            Aabb::new(
                self.position + Vec3::new(-0.35, 0., -0.25),
                self.position + Vec3::new(0.35, 1.55, 0.25),
            ),
            Aabb::new(
                self.position + Vec3::new(-0.25, 1.55, -0.25),
                self.position + Vec3::new(0.25, 2.05, 0.25),
            ),
        ]
    }
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
    pub targets: Vec<Target>,
    pub last_shot: Shot,
    pub feedback: f32,
    pub shots: u32,
    pub hits: u32,
}
impl Default for Combat {
    fn default() -> Self {
        Self {
            targets: vec![
                Target::new(Vec3::new(0., 0., -4.)),
                Target::new(Vec3::new(-6., 0., -12.)),
                Target::new(Vec3::new(6., 0., -20.)),
            ],
            last_shot: Shot::default(),
            feedback: 0.0,
            shots: 0,
            hits: 0,
        }
    }
}
impl Combat {
    pub fn advance(&mut self, dt: f32) {
        self.feedback = (self.feedback - dt).max(0.0);
        for t in &mut self.targets {
            t.flash = (t.flash - dt).max(0.0);
        }
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
        for (i, t) in self.targets.iter().enumerate() {
            for (part, b) in t.boxes().iter().enumerate() {
                if let Some(d) = b.ray_distance(origin, direction) {
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
        if let Some((i, head)) = target {
            let t = &mut self.targets[i];
            t.hp = t.hp.saturating_sub(damage * if head { 2 } else { 1 });
            shot.hit = true;
            shot.headshot = head;
            shot.killed = t.hp == 0;
            t.flash = 0.15;
            if shot.killed {
                t.hp = 100;
            }
            self.hits += 1;
        }
        self.shots += 1;
        self.last_shot = shot;
        self.feedback = 0.12;
        shot
    }
}
