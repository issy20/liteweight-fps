use glam::Vec3;

/// 衝突判定用の軸平行境界ボックス。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    /// 高さ位置と半径を与えられた円筒（水平円+垂直線分）がこのAABBと交差するかを返す。
    pub fn hits_cylinder(&self, center: Vec3, radius: f32, height: f32) -> bool {
        let half = height * 0.5;
        let cy_min = center.y - half;
        let cy_max = center.y + half;
        if cy_max < self.min.y || cy_min > self.max.y {
            return false;
        }
        let cx = center.x.clamp(self.min.x, self.max.x);
        let cz = center.z.clamp(self.min.z, self.max.z);
        (center.x - cx) * (center.x - cx) + (center.z - cz) * (center.z - cz) < radius * radius
    }
}
