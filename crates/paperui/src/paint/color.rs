//! Packed RGB565 color and the opaque font selector.

/// Packed RGB565 color, laid out `RRRRR_GGGGGG_BBBBB` — the same bit order as
/// `embedded_graphics::Rgb565`, so the TFT adapter reinterprets it for free.
/// Construct with [`Color::rgb`] using ordinary 8-bit channels; the down-convert
/// to 565 happens once, at compile time. The e-ink backend unpacks it back to
/// channels for its luma calc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u16);
impl Color {
    /// Pack 8-bit-per-channel RGB into RGB565 (truncating low bits: red/blue 8→5, green 8→6).
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self((((r >> 3) as u16) << 11) | (((g >> 2) as u16) << 5) | ((b >> 3) as u16))
    }
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(0xFF, 0xFF, 0xFF);
    pub const GRAY: Color = Color::rgb(0x80, 0x80, 0x80);

    /// Unpack the raw RGB565 channels `(r5, g6, b5)` — the single source for the bit layout.
    pub const fn channels(self) -> (u16, u16, u16) {
        ((self.0 >> 11) & 0x1F, (self.0 >> 5) & 0x3F, self.0 & 0x1F)
    }

    /// Repack raw RGB565 channels `(r5, g6, b5)` into a `Color`.
    pub const fn from_channels(r5: u16, g6: u16, b5: u16) -> Self {
        Self(((r5 & 0x1F) << 11) | ((g6 & 0x3F) << 5) | (b5 & 0x1F))
    }
}

/// Opaque font selector resolved by the backend/theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(pub u8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_rgb_packs_565() {
        // 0xFF→5 bits = 0x1F; 0x80→6 bits = 0x20; 0x00→0. Packed RRRRRGGGGGGBBBBB.
        assert_eq!(Color::rgb(0xFF, 0x80, 0x00).0, 0xFC00);
    }
}
