//! RGB888 color and the opaque font selector.

/// RGB888 color. Backends down-convert (e.g. to RGB565 or grayscale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u32);
impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(0xFF, 0xFF, 0xFF);
    pub const GRAY: Color = Color::rgb(0x80, 0x80, 0x80);
}

/// Opaque font selector resolved by the backend/theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(pub u8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_rgb_packs() {
        assert_eq!(Color::rgb(0xFF, 0x80, 0x00).0, 0x00FF_8000);
    }
}
