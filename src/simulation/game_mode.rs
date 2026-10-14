//! Gameplay rules that vary by training mode.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionMode {
    FreeTraining,
    TrackingAim,
}

impl SessionMode {
    pub fn recoil_enabled(self) -> bool {
        self != Self::TrackingAim
    }

    pub fn hip_fire_spread_enabled(self) -> bool {
        self != Self::TrackingAim
    }

    pub fn infinite_ammo_enabled(self, setting_enabled: bool) -> bool {
        self == Self::TrackingAim || (self == Self::FreeTraining && setting_enabled)
    }
}
