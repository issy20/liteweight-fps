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
    pub fn fov(self, ads: f32) -> f32 {
        let ads_fov = match self {
            Self::Pistol => 55.,
            Self::AssaultRifle => 50.,
            Self::Sniper => 32.,
        };
        70. + (ads_fov - 70.) * ads
    }
}

/// 描画や入力APIから独立した武器パラメータ。
#[derive(Clone, Copy, Debug)]
pub struct WeaponData {
    pub magazine: u32,
    pub interval: f32,
    pub reload_seconds: f32,
    pub damage: u32,
    pub ads_seconds: f32,
    pub hip_spread: f32,
    pub ads_spread: f32,
    pub recoil_kick: f32,
    pub recoil_recovery: f32,
    pub mode: FireMode,
    /// ADS精度補間が完了する進行度。NoneはADS全体を線形補間する。
    pub precision_at: Option<f32>,
    pub bolt_seconds: f32,
}
pub const AR: WeaponData = WeaponData {
    magazine: 30,
    interval: 0.1,
    reload_seconds: 2.0,
    damage: 25,
    ads_seconds: 0.18,
    hip_spread: 0.018,
    ads_spread: 0.001,
    recoil_kick: 0.012,
    recoil_recovery: 0.045,
    mode: FireMode::Automatic,
    precision_at: None,
    bolt_seconds: 0.0,
};
pub const PISTOL: WeaponData = WeaponData {
    magazine: 7,
    interval: 60. / 180.,
    reload_seconds: 1.8,
    damage: 50,
    ads_seconds: 0.16,
    hip_spread: 0.035,
    ads_spread: 0.004,
    recoil_kick: 0.045,
    recoil_recovery: 0.035,
    mode: FireMode::SemiAutomatic,
    precision_at: None,
    bolt_seconds: 0.0,
};
pub const SNIPER: WeaponData = WeaponData {
    magazine: 5,
    interval: 60. / 50.,
    reload_seconds: 2.8,
    damage: 100,
    ads_seconds: 0.2,
    hip_spread: 0.24,
    ads_spread: 0.0008,
    recoil_kick: 0.09,
    recoil_recovery: 0.025,
    mode: FireMode::BoltAction,
    precision_at: Some(0.85),
    bolt_seconds: 1.2,
};

pub struct Weapon {
    pub kind: WeaponKind,
    pub data: WeaponData,
    pub ammo: u32,
    pub ads: f32,
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
            ads: 0.,
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
        let direction = if ads && self.reload_remaining == 0.0 {
            1.0
        } else {
            -1.0
        };
        self.ads = (self.ads + direction * dt / self.data.ads_seconds).clamp(0.0, 1.0);
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
        let ads_fraction = if let Some(threshold) = self.data.precision_at {
            (self.ads / threshold).clamp(0.0, 1.0)
        } else {
            self.ads
        };
        let spread =
            self.data.hip_spread + (self.data.ads_spread - self.data.hip_spread) * ads_fraction;
        spread * if moving { 2.5 } else { 1.0 }
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
pub struct Loadout {
    weapons: [Weapon; 3],
    active: WeaponKind,
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
        let old = self.active_mut();
        // 切替は装填中断。残弾は維持し、ボルト操作はホルスター中も継続する。
        old.reload_remaining = 0.0;
        old.ads = 0.0;
        self.active = kind;
        self.active_mut().ads = 0.0;
        true
    }
    pub fn advance(&mut self, dt: f32, ads: bool) {
        let active = self.active;
        for weapon in &mut self.weapons {
            weapon.advance(dt, weapon.kind == active && ads);
        }
    }
    pub fn fire(&mut self, held: bool, pressed: bool) -> bool {
        let weapon = self.active_mut();
        let accepts = match weapon.data.mode {
            FireMode::Automatic => held || pressed,
            FireMode::SemiAutomatic | FireMode::BoltAction => pressed,
        };
        accepts && weapon.fire()
    }
    pub fn fov(&self) -> f32 {
        self.active().kind.fov(self.active().ads)
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
