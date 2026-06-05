//! Adapter: implements PaperUI's `Canvas` over any embedded-graphics
//! `DrawTarget<Color = Rgb565>`. Behind the `eg` feature so the engine never
//! depends on a graphics library unless this adapter is requested.

use embedded_graphics::draw_target::DrawTargetExt;
use embedded_graphics::mono_font::{ascii::FONT_6X9, MonoTextStyle};
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use embedded_graphics::text::{Baseline, Text};
use crate::{Canvas, Color, FontId, Point as PPoint, Rect, Size as PSize};

/// Reinterpret PaperUI's packed RGB565 as an embedded-graphics `Rgb565`. Same bit
/// layout, so this is free — the down-convert already happened in `Color::rgb`.
pub fn to_rgb565(c: Color) -> Rgb565 {
    Rgb565::from(RawU16::new(c.0))
}

fn rect_to_eg(r: Rect) -> Rectangle {
    Rectangle::new(
        embedded_graphics::geometry::Point::new(r.x as i32, r.y as i32),
        embedded_graphics::geometry::Size::new(r.w.max(0) as u32, r.h.max(0) as u32),
    )
}

/// `Canvas` implemented over a mutable borrow of any Rgb565 `DrawTarget`.
pub struct EgCanvas<'a, D> {
    target: &'a mut D,
    /// Active scissor rectangle; `None` means draw to the whole surface.
    clip: Option<Rect>,
}

impl<'a, D> EgCanvas<'a, D> {
    pub fn new(target: &'a mut D) -> Self { Self { target, clip: None } }
}

impl<'a, D> EgCanvas<'a, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    /// Draw `d`, confined to the active clip rect if `set_clip` set one. Single home for the
    /// clip branch, so each `Canvas` method (and any future one) stays a one-liner.
    fn draw_clipped<Dr: Drawable<Color = Rgb565>>(&mut self, d: Dr) {
        match self.clip {
            Some(c) => { let _ = d.draw(&mut self.target.clipped(&rect_to_eg(c))); }
            None => { let _ = d.draw(self.target); }
        }
    }
}

impl<'a, D> Canvas for EgCanvas<'a, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    fn fill_rect(&mut self, r: Rect, color: Color) {
        let style = PrimitiveStyle::with_fill(to_rgb565(color));
        self.draw_clipped(rect_to_eg(r).into_styled(style));
    }

    fn stroke_rect(&mut self, r: Rect, color: Color, width: u16) {
        // Inside alignment: the whole stroke stays within `r`. With the default centered
        // alignment, half the stroke spills outside `r`, and a later same-rect `fill_rect`
        // (the surgical repaint on focus change) can't cover it — leaving border ghosts.
        let style: PrimitiveStyle<Rgb565> = PrimitiveStyleBuilder::new()
            .stroke_color(to_rgb565(color))
            .stroke_width(width as u32)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();
        self.draw_clipped(rect_to_eg(r).into_styled(style));
    }

    fn text(&mut self, at: PPoint, s: &str, _font: FontId, color: Color) -> PSize {
        let style = MonoTextStyle::new(&FONT_6X9, to_rgb565(color));
        let eg_point = embedded_graphics::geometry::Point::new(at.x as i32, at.y as i32);
        self.draw_clipped(Text::with_baseline(s, eg_point, style, Baseline::Top));
        PSize::new(s.chars().count() as i16 * 6, 8)
    }

    fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::mock_display::MockDisplay;
    use embedded_graphics::pixelcolor::Rgb565;
    use crate::{Canvas, Color, FontId, Point, Rect};

    #[test]
    fn to_rgb565_downconverts_channels() {
        assert_eq!(to_rgb565(Color::WHITE), Rgb565::new(0x1F, 0x3F, 0x1F));
        assert_eq!(to_rgb565(Color::BLACK), Rgb565::new(0, 0, 0));
    }

    #[test]
    fn to_rgb565_matches_direct_downconvert() {
        // Pack-then-reinterpret must produce exactly the Rgb565 the old per-call
        // down-convert did, for representative theme colors (orange accent + grays).
        for (r, g, b) in [(0xFF, 0x80, 0x00), (0x80, 0x80, 0x80), (0x20, 0x20, 0x20), (0x60, 0x60, 0x60)] {
            assert_eq!(
                to_rgb565(Color::rgb(r, g, b)),
                Rgb565::new(r >> 3, g >> 2, b >> 3),
            );
        }
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
        assert_eq!(sz, crate::Size::new(2 * 6, 8));
    }

    #[test]
    fn set_clip_confines_fill_rect_to_clip_region() {
        // Draw a rect that spans (0,0,10,10) but clip to (0,0,5,5).
        // Pixels outside the clip should not be affected.
        let mut display: MockDisplay<Rgb565> = MockDisplay::new();
        display.set_allow_overdraw(true);
        let mut c = EgCanvas::new(&mut display);
        c.set_clip(Some(Rect::new(0, 0, 5, 5)));
        c.fill_rect(Rect::new(0, 0, 10, 10), Color::WHITE);
        // Affected area must be confined to the 5x5 clip
        let area = display.affected_area();
        assert!(area.size.width <= 5, "width {} should be <=5", area.size.width);
        assert!(area.size.height <= 5, "height {} should be <=5", area.size.height);
    }

    #[test]
    fn set_clip_none_restores_full_draw() {
        let mut display: MockDisplay<Rgb565> = MockDisplay::new();
        display.set_allow_overdraw(true);
        let mut c = EgCanvas::new(&mut display);
        c.set_clip(Some(Rect::new(0, 0, 3, 3)));
        c.set_clip(None);
        c.fill_rect(Rect::new(0, 0, 8, 8), Color::WHITE);
        assert_eq!(display.affected_area().size, embedded_graphics::geometry::Size::new(8, 8));
    }
}
