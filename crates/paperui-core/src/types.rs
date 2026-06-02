use crate::geometry::Size;

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

/// Semantic e-ink refresh-quality hint, set by the theme during draw.
/// Ordered worst-last so a region can take the max. Non-e-ink renderers ignore it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdateHint { None, Mono, Fast, Text, Quality }
impl Default for UpdateHint { fn default() -> Self { UpdateHint::Fast } }

/// Opaque font selector resolved by the backend/theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonId { A, B, C }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonEvent { Click, Hold, DoubleClick }

/// Layout size constraints (min/max), in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constraints { pub min_w: i16, pub min_h: i16, pub max_w: i16, pub max_h: i16 }
impl Constraints {
    pub const fn new(min_w: i16, min_h: i16, max_w: i16, max_h: i16) -> Self {
        Self { min_w, min_h, max_w, max_h }
    }
    pub fn clamp(&self, s: Size) -> Size {
        Size::new(s.w.clamp(self.min_w, self.max_w), s.h.clamp(self.min_h, self.max_h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_hint_is_ordered_for_worst_case_merge() {
        assert!(UpdateHint::Quality > UpdateHint::Mono);
        assert_eq!(UpdateHint::Mono.max(UpdateHint::Text), UpdateHint::Text);
    }

    #[test]
    fn constraints_clamp_works() {
        let c = Constraints::new(10, 10, 100, 50);
        assert_eq!(c.clamp(Size::new(5, 200)), Size::new(10, 50));
        assert_eq!(c.clamp(Size::new(40, 30)), Size::new(40, 30));
    }

    #[test]
    fn color_rgb_packs() {
        assert_eq!(Color::rgb(0xFF, 0x80, 0x00).0, 0x00FF_8000);
    }
}
