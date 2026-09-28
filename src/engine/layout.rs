/// 内部描画と表示先を分離する。表示先の0サイズは描画休止を表す。
#[derive(Debug, Clone, Copy)]
pub struct FrameLayout {
    internal: (u32, u32),
    display: (u32, u32),
}
impl FrameLayout {
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        Self::validate(width, height)?;
        Ok(Self {
            internal: (width, height),
            display: (width, height),
        })
    }
    fn validate(width: u32, height: u32) -> Result<(), &'static str> {
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
            return Err("内部解像度は各辺1〜4096pxを指定してください");
        }
        Ok(())
    }
    pub fn set_internal(&mut self, width: u32, height: u32) -> Result<(), &'static str> {
        Self::validate(width, height)?;
        self.internal = (width, height);
        Ok(())
    }
    pub fn internal_size(&self) -> (u32, u32) {
        self.internal
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.display = (width, height);
    }
    pub fn viewport(&self) -> Option<(u32, u32, u32, u32)> {
        let (dw, dh) = self.display;
        if dw == 0 || dh == 0 {
            return None;
        }
        let (iw, ih) = self.internal;
        let (w, h) = if u64::from(dw) * u64::from(ih) <= u64::from(dh) * u64::from(iw) {
            (dw, (u64::from(dw) * u64::from(ih) / u64::from(iw)) as u32)
        } else {
            ((u64::from(dh) * u64::from(iw) / u64::from(ih)) as u32, dh)
        };
        let (w, h) = (w.max(1), h.max(1));
        Some(((dw - w) / 2, (dh - h) / 2, w, h))
    }
}
