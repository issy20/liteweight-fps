use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use bytemuck::{Pod, Zeroable};
use std::sync::OnceLock;

const FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/Cantarell-Regular.ttf");
const FONT_SIZE: f32 = 48.0;
const COLUMNS: usize = 16;
const ROWS: usize = 6;
const CELL_WIDTH: usize = 64;
const CELL_HEIGHT: usize = 64;
const FIRST_CHAR: u8 = 32;
const LAST_CHAR: u8 = 126;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Default)]
pub struct TextMesh {
    pub vertices: Vec<TextVertex>,
    pub indices: Vec<u16>,
}

#[derive(Clone, Debug)]
pub struct TextLayout {
    pub mesh: TextMesh,
    pub width_px: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Glyph {
    atlas: [f32; 4],
    offset: [f32; 2],
    size: [f32; 2],
    advance: f32,
}

/// Embedded Cantarell atlas and proportional metrics for printable Basic Latin.
pub struct FontAtlas {
    pixels: Vec<u8>,
    glyphs: [Glyph; (LAST_CHAR - FIRST_CHAR + 1) as usize],
    ascent: f32,
}

impl FontAtlas {
    pub fn shared() -> &'static Self {
        static FONT: OnceLock<FontAtlas> = OnceLock::new();
        FONT.get_or_init(Self::embedded)
    }

    pub fn embedded() -> Self {
        let font = FontRef::try_from_slice(FONT_BYTES).expect("embedded font is valid");
        let scaled = font.as_scaled(PxScale::from(FONT_SIZE));
        let width = COLUMNS * CELL_WIDTH;
        let height = ROWS * CELL_HEIGHT;
        let mut pixels = vec![0_u8; width * height];
        let mut glyphs = [Glyph::default(); (LAST_CHAR - FIRST_CHAR + 1) as usize];

        for code in FIRST_CHAR..=LAST_CHAR {
            let index = usize::from(code - FIRST_CHAR);
            let column = index % COLUMNS;
            let row = index / COLUMNS;
            let cell_x = (column * CELL_WIDTH) as f32;
            let cell_y = (row * CELL_HEIGHT) as f32;
            let mut glyph = scaled.scaled_glyph(char::from(code));
            glyph.position = point(cell_x + 4.0, cell_y + scaled.ascent());
            let mut metrics = Glyph {
                advance: scaled.h_advance(glyph.id),
                ..Glyph::default()
            };
            if let Some(outline) = font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                let left = bounds.min.x.floor() as i32;
                let top = bounds.min.y.floor() as i32;
                let glyph_width = (bounds.max.x.ceil() as i32 - left).max(0) as usize;
                let glyph_height = (bounds.max.y.ceil() as i32 - top).max(0) as usize;
                metrics.offset = [
                    left as f32 - cell_x - 4.0,
                    top as f32 - cell_y - scaled.ascent(),
                ];
                metrics.size = [glyph_width as f32, glyph_height as f32];
                metrics.atlas = [
                    (left.max(0) as f32) / width as f32,
                    (top.max(0) as f32) / height as f32,
                    (left.max(0) as usize + glyph_width).min(width) as f32 / width as f32,
                    (top.max(0) as usize + glyph_height).min(height) as f32 / height as f32,
                ];
                outline.draw(|x, y, coverage| {
                    let px = left + x as i32;
                    let py = top + y as i32;
                    if px >= 0 && py >= 0 && (px as usize) < width && (py as usize) < height {
                        pixels[py as usize * width + px as usize] =
                            (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                });
            }
            glyphs[index] = metrics;
        }

        Self {
            pixels,
            glyphs,
            ascent: scaled.ascent(),
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        ((COLUMNS * CELL_WIDTH) as u32, (ROWS * CELL_HEIGHT) as u32)
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Build one indexed quad per glyph. Coordinates are pixels from the top-left.
    pub fn layout(
        &self,
        text: &str,
        origin_px: [f32; 2],
        font_px: f32,
        viewport: [u32; 2],
        color: [f32; 4],
    ) -> TextLayout {
        let [width, height] = viewport;
        if width == 0 || height == 0 || !font_px.is_finite() || font_px <= 0.0 {
            return TextLayout {
                mesh: TextMesh::default(),
                width_px: 0.0,
            };
        }
        let scale = font_px / FONT_SIZE;
        let mut cursor_x = origin_px[0];
        let mut baseline_y = origin_px[1] + self.ascent * scale;
        let mut mesh = TextMesh {
            vertices: Vec::with_capacity(text.chars().count() * 4),
            indices: Vec::with_capacity(text.chars().count() * 6),
        };
        let mut max_width = 0.0_f32;

        for character in text.chars() {
            if character == '\n' {
                max_width = max_width.max(cursor_x - origin_px[0]);
                cursor_x = origin_px[0];
                baseline_y += font_px * 1.25;
                continue;
            }
            let code = u32::from(character);
            let index = if (u32::from(FIRST_CHAR)..=u32::from(LAST_CHAR)).contains(&code) {
                code as usize - usize::from(FIRST_CHAR)
            } else {
                usize::from(b'?' - FIRST_CHAR)
            };
            let glyph = self.glyphs[index];
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                let left_px = cursor_x + glyph.offset[0] * scale;
                let top_px = baseline_y + glyph.offset[1] * scale;
                let right_px = left_px + glyph.size[0] * scale;
                let bottom_px = top_px + glyph.size[1] * scale;
                let left = left_px * 2.0 / width as f32 - 1.0;
                let right = right_px * 2.0 / width as f32 - 1.0;
                let top = 1.0 - top_px * 2.0 / height as f32;
                let bottom = 1.0 - bottom_px * 2.0 / height as f32;
                let [u0, v0, u1, v1] = glyph.atlas;
                let base = mesh.vertices.len() as u16;
                for (position, uv) in [
                    ([left, top], [u0, v0]),
                    ([right, top], [u1, v0]),
                    ([right, bottom], [u1, v1]),
                    ([left, bottom], [u0, v1]),
                ] {
                    mesh.vertices.push(TextVertex {
                        position,
                        uv,
                        color,
                    });
                }
                mesh.indices
                    .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
            cursor_x += glyph.advance * scale;
        }
        max_width = max_width.max(cursor_x - origin_px[0]);
        TextLayout {
            mesh,
            width_px: max_width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FontAtlas;

    #[test]
    fn embedded_font_atlas_has_coverage_and_proportional_metrics() {
        let font = FontAtlas::embedded();
        assert_eq!(font.dimensions(), (1024, 384));
        assert!(font.pixels().iter().any(|coverage| *coverage > 0));
        let i = font.layout("I", [0.0, 0.0], 32.0, [1280, 720], [1.0; 4]);
        let w = font.layout("W", [0.0, 0.0], 32.0, [1280, 720], [1.0; 4]);
        assert!(i.width_px < w.width_px);
        assert_eq!(i.mesh.vertices.len(), 4);
        assert_eq!(i.mesh.indices.len(), 6);
    }
}
