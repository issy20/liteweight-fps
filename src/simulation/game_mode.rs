//! Match timers and result statistics shared by training modes.

use super::weapon::WeaponKind;
use serde::{Deserialize, Serialize};

pub const CHALLENGE_SECONDS: f32 = 5.0 * 60.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionMode {
    FreeTraining,
    FiveMinuteChallenge,
}

impl SessionMode {
    pub fn infinite_ammo_enabled(self, setting_enabled: bool) -> bool {
        self == Self::FreeTraining && setting_enabled
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WeaponStats {
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    pub kills: u32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MatchStats {
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    pub kills: u32,
    pub pistol: WeaponStats,
    pub assault_rifle: WeaponStats,
    pub sniper: WeaponStats,
}

impl MatchStats {
    pub fn accuracy(self) -> f32 {
        ratio(self.hits, self.shots)
    }

    pub fn headshot_rate(self) -> f32 {
        ratio(self.headshots, self.hits)
    }

    pub fn for_weapon(self, weapon: WeaponKind) -> WeaponStats {
        match weapon {
            WeaponKind::Pistol => self.pistol,
            WeaponKind::AssaultRifle => self.assault_rifle,
            WeaponKind::Sniper => self.sniper,
        }
    }

    fn record_shot(&mut self, weapon: WeaponKind, hit: bool, headshot: bool, killed: bool) {
        self.shots += 1;
        self.hits += u32::from(hit);
        self.headshots += u32::from(headshot);
        self.kills += u32::from(killed);
        let weapon_stats = match weapon {
            WeaponKind::Pistol => &mut self.pistol,
            WeaponKind::AssaultRifle => &mut self.assault_rifle,
            WeaponKind::Sniper => &mut self.sniper,
        };
        weapon_stats.shots += 1;
        weapon_stats.hits += u32::from(hit);
        weapon_stats.headshots += u32::from(headshot);
        weapon_stats.kills += u32::from(killed);
    }
}

fn ratio(numerator: u32, denominator: u32) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

#[derive(Clone, Debug)]
pub struct TrainingSession {
    mode: SessionMode,
    remaining_seconds: Option<f32>,
    complete: bool,
    stats: MatchStats,
}

impl TrainingSession {
    pub fn new(mode: SessionMode) -> Self {
        Self {
            mode,
            remaining_seconds: match mode {
                SessionMode::FreeTraining => None,
                SessionMode::FiveMinuteChallenge => Some(CHALLENGE_SECONDS),
            },
            complete: false,
            stats: MatchStats::default(),
        }
    }

    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    pub fn remaining_seconds(&self) -> Option<f32> {
        self.remaining_seconds
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Advances active match time and returns true only on the completion transition.
    pub fn advance(&mut self, dt: f32) -> bool {
        if self.complete || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        let Some(remaining) = &mut self.remaining_seconds else {
            return false;
        };
        *remaining = (*remaining - dt).max(0.0);
        if *remaining == 0.0 {
            self.complete = true;
            return true;
        }
        false
    }

    pub fn record_shot(&mut self, weapon: WeaponKind, hit: bool, headshot: bool, killed: bool) {
        if !self.complete {
            self.stats.record_shot(weapon, hit, headshot, killed);
        }
    }

    pub fn stats(&self) -> MatchStats {
        self.stats
    }
}
