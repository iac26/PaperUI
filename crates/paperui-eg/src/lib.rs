#![no_std]
//! Adapter: implements paperui-core's `Canvas` over any embedded-graphics
//! `DrawTarget<Color = Rgb565>`. The engine and themes never depend on
//! embedded-graphics; only this crate (and the device backends) do.

use embedded_graphics::mono_font::{ascii::FONT_6X9, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use paperui_core::{Canvas, Color, FontId, Point as PPoint, Rect, Size as PSize};

/// Down-convert PaperUI RGB888 to embedded-graphics Rgb565.
pub fn to_rgb565(c: Color) -> Rgb565 {
    let r = ((c.0 >> 16) & 0xFF) as u8;
    let g = ((c.0 >> 8) & 0xFF) as u8;
    let b = (c.0 & 0xFF) as u8;
    Rgb565::new(r >> 3, g >> 2, b >> 3)
}

fn rect_to_eg(r: Rect) -> Rectangle {
    Rectangle::new(
        embedded_graphics::geometry::Point::new(r.x as i32, r.y as i32),
        embedded_graphics::geometry::Size::new(r.w.max(0) as u32, r.h.max(0) as u32),
    )
}

/// `Canvas` implemented over a mutable borrow of any Rgb565 `DrawTarget`.
pub struct EgCanvas<'a, D> { target: &'a mut D }

impl<'a, D> EgCanvas<'a, D> {
    pub fn new(target: &'a mut D) -> Self { Self { target } }
}

impl<'a, D> Canvas for EgCanvas<'a, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    fn fill_rect(&mut self, r: Rect, color: Color) {
        let style = PrimitiveStyle::with_fill(to_rgb565(color));
        let _ = rect_to_eg(r).into_styled(style).draw(self.target);
    }

    fn stroke_rect(&mut self, r: Rect, color: Color, width: u16) {
        let style: PrimitiveStyle<Rgb565> = PrimitiveStyleBuilder::new()
            .stroke_color(to_rgb565(color))
            .stroke_width(width as u32)
            .build();
        let _ = rect_to_eg(r).into_styled(style).draw(self.target);
    }

    fn text(&mut self, at: PPoint, s: &str, _font: FontId, color: Color) -> PSize {
        let style = MonoTextStyle::new(&FONT_6X9, to_rgb565(color));
        let _ = Text::with_baseline(
            s,
            embedded_graphics::geometry::Point::new(at.x as i32, at.y as i32),
            style,
            Baseline::Top,
        )
        .draw(self.target);
        PSize::new(s.chars().count() as i16 * 6, 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::mock_display::MockDisplay;
    use embedded_graphics::pixelcolor::Rgb565;
    use paperui_core::{Canvas, Color, FontId, Point, Rect};

    #[test]
    fn to_rgb565_downconverts_channels() {
        assert_eq!(to_rgb565(Color::WHITE), Rgb565::new(0x1F, 0x3F, 0x1F));
        assert_eq!(to_rgb565(Color::BLACK), Rgb565::new(0, 0, 0));
    }

    #[test]
    fn fill_rect_draws_into_target_without_panicking() {
        let mut display: MockDisplay<Rgb565> = MockDisplay::new();
        display.set_allow_overdraw(true);
        let mut c = EgCanvas::new(&mut display);
        c.fill_rect(Rect::new(0, 0, 3, 2), Color::WHITE);
        assert_eq!(display.affected_area().size, embedded_graphics::geometry::Size::new(3, 2));
    }

    #[test]
    fn text_returns_six_by_eight_per_char() {
        let mut display: MockDisplay<Rgb565> = MockDisplay::new();
        display.set_allow_overdraw(true);
        let mut c = EgCanvas::new(&mut display);
        let sz = c.text(Point::new(0, 0), "Hi", FontId(0), Color::WHITE);
        assert_eq!(sz, paperui_core::Size::new(2 * 6, 8));
    }
}
