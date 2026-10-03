use glam::Vec3;

/// 発射入力方式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    SemiAutomatic,
    Automatic,
    BoltAction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    Pistol,
    AssaultRifle,
    Sniper,
}
impl WeaponKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pistol => "PISTOL",
            Self::AssaultRifle => "ASSAULT RIFLE",
            Self::Sniper => "SNIPER",
        }
    }
    pub fn fov(self, ads_progress: f32) -> f32 {
        let ads_fov = match self {
            Self::Pistol => 55.,
            Self::AssaultRifle => 50.,
            Self::Sniper => 32.,
        };
        70. + (ads_fov - 70.) * ads_progress
    }
}

/// 描画や入力APIから独立した武器パラメータ。
#[derive(Clone, Copy, Debug)]
pub struct WeaponData {
    pub magazine: u32,
    pub interval: f32,
    pub reload_seconds: f32,
    pub damage: u32,
    pub ads_in_seconds: f32,
    pub ads_out_seconds: f32,
    pub hip_spread: f32,
    pub ads_spread: f32,
    pub recoil_kick: f32,
    pub recoil_recovery: f32,
    pub mode: FireMode,
    pub bolt_seconds: f32,
}
pub const AR: WeaponData = WeaponData {
    magazine: 30,
    interval: 0.1,
    reload_seconds: 2.0,
    damage: 25,
    ads_in_seconds: 0.18,
    ads_out_seconds: 0.18,
    hip_spread: 0.018,
    ads_spread: 0.001,
    recoil_kick: 0.012,
    recoil_recovery: 0.045,
    mode: FireMode::Automatic,
    bolt_seconds: 0.0,
};
pub const PISTOL: WeaponData = WeaponData {
    magazine: 7,
    interval: 60. / 180.,
    reload_seconds: 1.8,
    damage: 50,
    ads_in_seconds: 0.16,
    ads_out_seconds: 0.16,
    hip_spread: 0.035,
    ads_spread: 0.004,
    recoil_kick: 0.045,
    recoil_recovery: 0.035,
    mode: FireMode::SemiAutomatic,
    bolt_seconds: 0.0,
};
pub const SNIPER_ADS_IN_SECONDS: f32 = 0.35;
pub const SNIPER_ADS_OUT_SECONDS: f32 = 0.20;
const SNIPER_HIP_ALIGNMENT_RADIANS: f32 = 4.0_f32.to_radians();
pub const SNIPER: WeaponData = WeaponData {
    magazine: 5,
    interval: 60. / 50.,
    reload_seconds: 2.8,
    damage: 100,
    ads_in_seconds: SNIPER_ADS_IN_SECONDS,
    ads_out_seconds: SNIPER_ADS_OUT_SECONDS,
    hip_spread: 0.0,
    ads_spread: 0.0,
    recoil_kick: 0.09,
    recoil_recovery: 0.025,
    mode: FireMode::BoltAction,
    bolt_seconds: 1.2,
};

pub struct Weapon {
    pub kind: WeaponKind,
    pub data: WeaponData,
    pub ammo: u32,
    pub ads_progress: f32,
    pub recoil: f32,
    pub visual_recoil: f32,
    cooldown: f32,
    pub reload_remaining: f32,
    pub bolt_remaining: f32,
}
impl Default for Weapon {
    fn default() -> Self {
        Self::new(WeaponKind::AssaultRifle, AR)
    }
}
impl Weapon {
    pub fn new(kind: WeaponKind, data: WeaponData) -> Self {
        Self {
            kind,
            data,
            ammo: data.magazine,
            ads_progress: 0.,
            recoil: 0.,
            visual_recoil: 0.,
            cooldown: 0.,
            reload_remaining: 0.,
            bolt_remaining: 0.,
        }
    }
    pub fn advance(&mut self, dt: f32, ads: bool) {
        self.recoil = (self.recoil - self.data.recoil_recovery * dt).max(0.0);
        self.visual_recoil = (self.visual_recoil - dt * 10.0).max(0.0);
        let aiming = ads && self.reload_remaining == 0.0;
        let duration = if aiming {
            self.data.ads_in_seconds
        } else {
            self.data.ads_out_seconds
        };
        let direction = if aiming { 1.0 } else { -1.0 };
        self.ads_progress =
            (self.ads_progress + direction * dt / duration.max(f32::EPSILON)).clamp(0.0, 1.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.bolt_remaining = (self.bolt_remaining - dt).max(0.0);
        if self.reload_remaining > 0.0 {
            self.reload_remaining = (self.reload_remaining - dt).max(0.0);
            if self.reload_remaining < 1e-5 {
                self.reload_remaining = 0.0;
                self.ammo = self.data.magazine;
            }
        }
    }
    pub fn spread(&self, moving: bool) -> f32 {
        let spread = self.data.hip_spread
            + (self.data.ads_spread - self.data.hip_spread) * self.ads_progress;
        spread * if moving { 2.5 } else { 1.0 }
    }
    /// スナイパーは弾のランダム拡散を使わず、腰撃ち時の照準ずれをADSで収束させる。
    pub fn shot_direction(&self, view_direction: Vec3) -> Vec3 {
        let view = view_direction.normalize_or_zero();
        if self.kind != WeaponKind::Sniper || view == Vec3::ZERO {
            return view;
        }
        let right = view.cross(Vec3::Y).normalize_or_zero();
        (view + right * (SNIPER_HIP_ALIGNMENT_RADIANS.tan() * (1.0 - self.ads_progress)))
            .normalize_or_zero()
    }
    pub fn reload(&mut self) -> bool {
        if self.reload_remaining > 0.0
            || self.bolt_remaining > 0.0
            || self.ammo == self.data.magazine
        {
            return false;
        }
        self.reload_remaining = self.data.reload_seconds;
        true
    }
    pub fn fire(&mut self) -> bool {
        if self.reload_remaining > 0.0
            || self.bolt_remaining > 0.0
            || self.cooldown > 1e-6
            || self.ammo == 0
        {
            return false;
        }
        self.ammo -= 1;
        self.recoil = (self.recoil + self.data.recoil_kick).min(0.3);
        self.visual_recoil = 1.0;
        self.cooldown = self.data.interval;
        if self.data.mode == FireMode::BoltAction {
            self.bolt_remaining = self.data.bolt_seconds;
        }
        true
    }
}

/// 個別の装弾数・リロード状態を保つ3武器ロードアウト。
pub const SWITCH_SECONDS: f32 = 0.32;
const SWITCH_LOWER_SECONDS: f32 = 0.14;
const SWITCH_DROP: f32 = 1.05;
pub struct Loadout {
    weapons: [Weapon; 3],
    active: WeaponKind,
    switch_from: Option<WeaponKind>,
    switch_remaining: f32,
}
impl Default for Loadout {
    fn default() -> Self {
        Self {
            weapons: [
                Weapon::new(WeaponKind::Pistol, PISTOL),
                Weapon::default(),
                Weapon::new(WeaponKind::Sniper, SNIPER),
            ],
            active: WeaponKind::AssaultRifle,
            switch_from: None,
            switch_remaining: 0.0,
        }
    }
}
impl Loadout {
    fn index(kind: WeaponKind) -> usize {
        match kind {
            WeaponKind::Pistol => 0,
            WeaponKind::AssaultRifle => 1,
            WeaponKind::Sniper => 2,
        }
    }
    pub fn active_kind(&self) -> WeaponKind {
        self.active
    }
    pub fn active(&self) -> &Weapon {
        &self.weapons[Self::index(self.active)]
    }
    pub fn active_mut(&mut self) -> &mut Weapon {
        &mut self.weapons[Self::index(self.active)]
    }
    pub fn switch(&mut self, kind: WeaponKind) -> bool {
        if self.active == kind {
            return false;
        }
        let old_kind = self.active;
        let old = self.active_mut();
        // 切替は装填中断。残弾は維持し、ボルト操作はホルスター中も継続する。
        old.reload_remaining = 0.0;
        old.ads_progress = 0.0;
        self.switch_from = Some(old_kind);
        self.switch_remaining = SWITCH_SECONDS;
        self.active = kind;
        self.active_mut().ads_progress = 0.0;
        true
    }
    pub fn advance(&mut self, dt: f32, ads: bool) {
        let active = self.active;
        self.switch_remaining = (self.switch_remaining - dt).max(0.0);
        if self.switch_remaining < 1e-5 {
            self.switch_remaining = 0.0;
            self.switch_from = None;
        }
        for weapon in &mut self.weapons {
            weapon.advance(
                dt,
                weapon.kind == active && ads && self.switch_remaining == 0.0,
            );
        }
    }
    pub fn is_switching(&self) -> bool {
        self.switch_remaining > 0.0
    }
    pub fn switch_remaining(&self) -> f32 {
        self.switch_remaining
    }
    /// 切替の前半は旧武器を下ろし、後半は新武器を構える。
    pub fn presentation(&self) -> (&Weapon, f32) {
        if let Some(old) = self.switch_from {
            let elapsed = SWITCH_SECONDS - self.switch_remaining;
            if elapsed < SWITCH_LOWER_SECONDS {
                return (
                    &self.weapons[Self::index(old)],
                    -SWITCH_DROP * elapsed / SWITCH_LOWER_SECONDS,
                );
            }
            let raise = ((elapsed - SWITCH_LOWER_SECONDS)
                / (SWITCH_SECONDS - SWITCH_LOWER_SECONDS))
                .clamp(0.0, 1.0);
            return (self.active(), -SWITCH_DROP * (1.0 - raise));
        }
        (self.active(), 0.0)
    }
    pub fn fire(&mut self, held: bool, pressed: bool) -> bool {
        if self.is_switching() {
            return false;
        }
        let weapon = self.active_mut();
        let accepts = match weapon.data.mode {
            FireMode::Automatic => held || pressed,
            FireMode::SemiAutomatic | FireMode::BoltAction => pressed,
        };
        accepts && weapon.fire()
    }
    pub fn fov(&self) -> f32 {
        self.active().kind.fov(self.active().ads_progress)
    }
}

/// 描画の間に押下・解放されても一度は射撃を試みる。冷却中のクリックは予約しない。
#[derive(Default)]
pub struct FireInput {
    held: bool,
    pressed: bool,
    pub ads: bool,
    pub reload: bool,
}
impl FireInput {
    pub fn set_trigger(&mut self, held: bool) {
        self.pressed |= held && !self.held;
        self.held = held;
    }
    pub fn take_state(&mut self) -> (bool, bool) {
        let state = (self.held, self.pressed);
        self.pressed = false;
        state
    }
    pub fn discard_press(&mut self) {
        self.pressed = false;
    }
    pub fn take_trigger(&mut self) -> bool {
        let (held, pressed) = self.take_state();
        held || pressed
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
