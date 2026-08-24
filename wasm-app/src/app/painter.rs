//! Immediate-mode 2D "painter" that apps and the window manager draw
//! into every frame. It only knows about colored rectangles — including
//! text, which is rasterised from [`crate::font`] into per-pixel quads —
//! so the whole desktop is a single instanced-quad draw call on the GPU.

use super::font::{glyph_rows, GLYPH_COLS, GLYPH_ROWS};
use super::geometry::Rect;

#[derive(Clone, Copy, Debug)]
pub struct Color(pub f32, pub f32, pub f32, pub f32);

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }
}

/// One instance fed to the GPU quad pipeline: a pixel-space rect plus a
/// straight RGBA color (see `render()` in [`crate::gpu`] for the layout).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub rect: [f32; 4],
    pub color: [f32; 4],
}

#[derive(Default)]
pub struct Painter {
    pub quads: Vec<QuadInstance>,
}

impl Painter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.quads.clear();
    }

    pub fn rect(&mut self, r: Rect, color: Color) {
        self.quads.push(QuadInstance {
            rect: [r.x, r.y, r.w, r.h],
            color: [color.0, color.1, color.2, color.3],
        });
    }

    pub fn rect_outline(&mut self, r: Rect, thickness: f32, color: Color) {
        self.rect(Rect::new(r.x, r.y, r.w, thickness), color);
        self.rect(Rect::new(r.x, r.y + r.h - thickness, r.w, thickness), color);
        self.rect(Rect::new(r.x, r.y, thickness, r.h), color);
        self.rect(Rect::new(r.x + r.w - thickness, r.y, thickness, r.h), color);
    }

    /// Draws `text` (upper-cased) at `(x, y)` with each font pixel drawn
    /// as a `scale`-sized square. Returns the total width in pixels, which
    /// is handy for centering/laying out labels.
    pub fn text(&mut self, x: f32, y: f32, scale: f32, color: Color, text: &str) -> f32 {
        let mut cursor_x = x;
        for ch in text.chars() {
            let rows = glyph_rows(ch.to_ascii_uppercase());
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..GLYPH_COLS {
                    let lit = (bits >> (GLYPH_COLS - 1 - col)) & 1 == 1;
                    if lit {
                        self.rect(
                            Rect::new(
                                cursor_x + col as f32 * scale,
                                y + row as f32 * scale,
                                scale,
                                scale,
                            ),
                            color,
                        );
                    }
                }
            }
            cursor_x += (GLYPH_COLS + 1) as f32 * scale;
        }
        cursor_x - x
    }

    /// Width in pixels that [`Painter::text`] would occupy for `text`,
    /// without actually drawing it — used to right-align/center labels.
    pub fn text_width(text: &str, scale: f32) -> f32 {
        text.chars().count() as f32 * (GLYPH_COLS + 1) as f32 * scale
    }

    pub fn text_height(scale: f32) -> f32 {
        GLYPH_ROWS as f32 * scale
    }
}
