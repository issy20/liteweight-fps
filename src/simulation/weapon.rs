/// ARの仮パラメータ。描画や入力APIから独立させる。
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
};

pub struct Weapon {
    pub data: WeaponData,
    pub ammo: u32,
    pub ads: f32,
    pub recoil: f32,
    pub visual_recoil: f32,
    cooldown: f32,
    pub reload_remaining: f32,
}
impl Default for Weapon {
    fn default() -> Self {
        Self {
            data: AR,
            ammo: AR.magazine,
            ads: 0.0,
            recoil: 0.0,
            visual_recoil: 0.0,
            cooldown: 0.0,
            reload_remaining: 0.0,
        }
    }
}
impl Weapon {
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
        if self.reload_remaining > 0.0 {
            self.reload_remaining = (self.reload_remaining - dt).max(0.0);
            if self.reload_remaining < 1e-5 {
                self.reload_remaining = 0.0;
                self.ammo = self.data.magazine;
            }
        }
    }
    pub fn spread(&self, moving: bool) -> f32 {
        (self.data.hip_spread + (self.data.ads_spread - self.data.hip_spread) * self.ads)
            * if moving { 2.5 } else { 1.0 }
    }
    pub fn reload(&mut self) -> bool {
        if self.reload_remaining > 0.0 || self.ammo == self.data.magazine {
            return false;
        }
        self.reload_remaining = self.data.reload_seconds;
        true
    }
    pub fn fire(&mut self) -> bool {
        if self.reload_remaining > 0.0 || self.cooldown > 1e-6 || self.ammo == 0 {
            return false;
        }
        self.ammo -= 1;
        self.recoil = (self.recoil + self.data.recoil_kick).min(0.15);
        self.visual_recoil = 1.0;
        self.cooldown = self.data.interval;
        true
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
    pub fn take_trigger(&mut self) -> bool {
        let trigger = self.held || self.pressed;
        self.pressed = false;
        trigger
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
