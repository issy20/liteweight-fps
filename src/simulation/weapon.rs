use glam::{Vec2, Vec3};

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
        self.fov_with_base(70.0, ads_progress)
    }
    pub fn fov_with_base(self, base_fov: f32, ads_progress: f32) -> f32 {
        let ads_fov = match self {
            Self::Pistol => 67.,
            Self::AssaultRifle => 65.,
            Self::Sniper => 32.,
        };
        let progress = ads_progress.clamp(0.0, 1.0);
        base_fov + (ads_fov - base_fov) * progress
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
    mode: FireMode::BoltAction,
    bolt_seconds: 1.2,
};

pub struct Weapon {
    pub kind: WeaponKind,
    pub data: WeaponData,
    pub ammo: u32,
    pub ads_progress: f32,
    pub visual_recoil: f32,
    cooldown: f32,
    pub reload_remaining: f32,
    pub bolt_remaining: f32,
}

/// Shared screen-space layout for the HUD weapon, sight axis, and firing origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponViewModelLayout {
    pub center_x: f32,
    pub base_y: f32,
    pub body_width: f32,
    pub body_height: f32,
    pub sight_screen: Vec2,
    pub muzzle_screen: Vec2,
}

const VIEW_MODEL_MUZZLE_DEPTH: f32 = 0.65;
const SNIPER_SCOPE_SIGHT_HEIGHT: f32 = 0.62;

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
            visual_recoil: 0.,
            cooldown: 0.,
            reload_remaining: 0.,
            bolt_remaining: 0.,
        }
    }
    pub fn advance(&mut self, dt: f32, ads: bool) {
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
    /// Reproducible per-shot hip-fire deviation; the sniper keeps its dedicated alignment rule.
    pub fn spread_direction(&self, view_direction: Vec3, moving: bool, shot_index: u32) -> Vec3 {
        let forward = self.shot_direction(view_direction);
        if self.kind == WeaponKind::Sniper {
            return forward;
        }
        let sample = shot_index.wrapping_mul(1664525).wrapping_add(1013904223);
        let angle = (sample & 65535) as f32 / 65536.0 * std::f32::consts::TAU;
        let radius = ((sample >> 16) as f32 / 65536.0).sqrt() * self.spread(moving);
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward);
        (forward + (right * angle.cos() + up * angle.sin()) * radius).normalize_or_zero()
    }
    pub fn view_model_layout(&self, weapon_y_offset: f32) -> WeaponViewModelLayout {
        let progress = self.ads_progress.clamp(0.0, 1.0);
        let (body_width, body_height, sight_height, muzzle_height) = match self.kind {
            WeaponKind::Pistol => (0.11, 0.28, 0.28 * 0.91 + 0.12, 0.28 * 0.91 + 0.12),
            WeaponKind::AssaultRifle => (0.17, 0.4, 0.4 + 0.07 + 0.19, 0.4 + 0.07 + 0.19),
            WeaponKind::Sniper => (0.12, 0.48, SNIPER_SCOPE_SIGHT_HEIGHT, 0.48 * 0.52 + 0.31),
        };
        let recoil_drop = self.visual_recoil * 0.045;
        let reload_drop = if self.reload_remaining > 0.0 {
            0.22
        } else {
            0.0
        };
        let hip_base_y = -0.7 + weapon_y_offset - recoil_drop - reload_drop;
        let ads_raise = (0.7 - sight_height) * progress;
        let center_x = 0.36 * (1.0 - progress);
        let base_y = hip_base_y + ads_raise;
        WeaponViewModelLayout {
            center_x,
            base_y,
            body_width,
            body_height,
            sight_screen: Vec2::new(center_x, base_y + sight_height),
            muzzle_screen: Vec2::new(center_x, base_y + muzzle_height),
        }
    }
    /// Returns the world-space point corresponding to the rendered barrel tip.
    pub fn muzzle_position(
        &self,
        eye: Vec3,
        view_direction: Vec3,
        aspect: f32,
        fov_degrees: f32,
        weapon_y_offset: f32,
    ) -> Vec3 {
        let forward = view_direction.normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let layout = self.view_model_layout(weapon_y_offset);
        let scale = VIEW_MODEL_MUZZLE_DEPTH * (fov_degrees.to_radians() * 0.5).tan();
        let ndc_x = layout.muzzle_screen.x / aspect.max(f32::EPSILON);
        let camera_plane_width = aspect.max(f32::EPSILON) * scale;
        eye + forward * VIEW_MODEL_MUZZLE_DEPTH
            + right * (ndc_x * camera_plane_width)
            + up * (layout.muzzle_screen.y * scale)
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
        self.fire_with_infinite_ammo(held, pressed, false)
    }
    pub fn fire_with_infinite_ammo(
        &mut self,
        held: bool,
        pressed: bool,
        infinite_ammo: bool,
    ) -> bool {
        if self.is_switching() {
            return false;
        }
        let weapon = self.active_mut();
        if infinite_ammo {
            weapon.ammo = weapon.data.magazine;
        }
        let accepts = match weapon.data.mode {
            FireMode::Automatic => held || pressed,
            FireMode::SemiAutomatic | FireMode::BoltAction => pressed,
        };
        let fired = accepts && weapon.fire();
        if infinite_ammo {
            weapon.ammo = weapon.data.magazine;
        }
        fired
    }
    pub fn fov(&self) -> f32 {
        self.active().kind.fov(self.active().ads_progress)
    }
    pub fn fov_with_base(&self, base_fov: f32) -> f32 {
        self.active()
            .kind
            .fov_with_base(base_fov, self.active().ads_progress)
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
