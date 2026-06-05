//! A bounded 4-bit grayscale framebuffer that implements paperui-core's `Canvas`.
//! The EinkRenderer flushes its packed bytes to the IT8951. Static (no alloc).

use paperui::{Canvas, Color, FontId, Point, Rect, Size, FONT0_H, FONT0_W};

/// Modest windowed buffer (one byte/pixel, gray 0..=15) — a full 960x540 buffer would
/// overflow ESP32 internal RAM, so full-screen rendering streams per-region / uses PSRAM.
pub const FB_W: usize = 320;
pub const FB_H: usize = 240;
const FB_PIXELS: usize = FB_W * FB_H;

pub struct Gray4Canvas {
    pub buf: [u8; FB_PIXELS],
    pub w: i16,
    pub h: i16,
    /// Active scissor rectangle; `None` means the whole framebuffer is drawable.
    clip: Option<Rect>,
}

impl Gray4Canvas {
    pub fn new() -> Self {
        Self { buf: [0x0F; FB_PIXELS], w: FB_W as i16, h: FB_H as i16, clip: None }
    }

    fn put(&mut self, x: i16, y: i16, gray: u8) {
        // Bounds-check the framebuffer dimensions first, then honour the scissor.
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        if let Some(c) = self.clip {
            if !c.contains(Point::new(x, y)) {
                return;
            }
        }
        self.buf[y as usize * FB_W + x as usize] = gray & 0x0F;
    }
}

impl Default for Gray4Canvas {
    fn default() -> Self {
        Self::new()
    }
}

fn gray4(c: Color) -> u8 {
    // Color is now packed RGB565. Unpack each channel and expand back to ~8-bit
    // (replicate the high bits into the low gap) so the luma weights stay calibrated
    // for 0..=255 inputs. Precision is 5/6-bit-limited, but the output is 4-bit gray.
    let (r5, g6, b5) = c.channels();
    let r = ((r5 << 3) | (r5 >> 2)) as u32;
    let g = ((g6 << 2) | (g6 >> 4)) as u32;
    let b = ((b5 << 3) | (b5 >> 2)) as u32;
    let luma = (r * 54 + g * 183 + b * 19) >> 8;
    (luma >> 4) as u8
}

impl Canvas for Gray4Canvas {
    fn fill_rect(&mut self, r: Rect, color: Color) {
        let g = gray4(color);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                self.put(x, y, g);
            }
        }
    }

    fn stroke_rect(&mut self, r: Rect, color: Color, width: u16) {
        let g = gray4(color);
        let wq = width as i16;
        for t in 0..wq {
            for x in r.x..r.x + r.w {
                self.put(x, r.y + t, g);
                self.put(x, r.y + r.h - 1 - t, g);
            }
            for y in r.y..r.y + r.h {
                self.put(r.x + t, y, g);
                self.put(r.x + r.w - 1 - t, y, g);
            }
        }
    }

    fn text(&mut self, at: Point, s: &str, _font: FontId, color: Color) -> Size {
        let g = gray4(color);
        let mut cx = at.x;
        for _ch in s.chars() {
            for y in at.y..at.y + FONT0_H {
                for x in cx..cx + FONT0_W - 1 {
                    self.put(x, y, g);
                }
            }
            cx += FONT0_W;
        }
        Size::new(s.chars().count() as i16 * FONT0_W, FONT0_H)
    }

    fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }
}
