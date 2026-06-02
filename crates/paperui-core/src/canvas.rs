use crate::geometry::{Point, Rect, Size};
use crate::types::{Color, FontId};

/// The engine's own minimal drawing surface. The core and themes depend ONLY on
/// this trait — never on a concrete graphics library. Backends provide an impl
/// (e.g. an adapter over embedded-graphics::DrawTarget) in a later plan.
pub trait Canvas {
    fn fill_rect(&mut self, r: Rect, color: Color);
    fn stroke_rect(&mut self, r: Rect, color: Color, width: u16);
    /// Draw `s` at `at`; returns the pixel size the text occupied.
    fn text(&mut self, at: Point, s: &str, font: FontId, color: Color) -> Size;
}

/// Glyph cell size for FontId(0): 6x8 px (matches a classic 5x7+gap bitmap font).
pub const FONT0_W: i16 = 6;
pub const FONT0_H: i16 = 8;

#[cfg(feature = "mock")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawOp { FillRect(Rect, Color), StrokeRect(Rect, Color, u16), Text(Point, Color) }

/// Host-test recording canvas. Records each draw call into a fixed-capacity log.
#[cfg(feature = "mock")]
pub struct MockCanvas { pub ops: heapless::Vec<DrawOp, 64> }

#[cfg(feature = "mock")]
impl MockCanvas {
    pub fn new() -> Self { Self { ops: heapless::Vec::new() } }
}

#[cfg(feature = "mock")]
impl Default for MockCanvas {
    fn default() -> Self { Self::new() }
}

#[cfg(feature = "mock")]
impl Canvas for MockCanvas {
    fn fill_rect(&mut self, r: Rect, color: Color) {
        let _ = self.ops.push(DrawOp::FillRect(r, color));
    }
    fn stroke_rect(&mut self, r: Rect, color: Color, width: u16) {
        let _ = self.ops.push(DrawOp::StrokeRect(r, color, width));
    }
    fn text(&mut self, at: Point, s: &str, _font: FontId, color: Color) -> Size {
        let _ = self.ops.push(DrawOp::Text(at, color));
        Size::new(s.chars().count() as i16 * FONT0_W, FONT0_H)
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::{Color, FontId, Point, Rect};

    #[test]
    fn mock_canvas_records_ops_in_order() {
        let mut c = MockCanvas::new();
        c.fill_rect(Rect::new(0, 0, 10, 5), Color::WHITE);
        let sz = c.text(Point::new(1, 1), "Hi", FontId(0), Color::BLACK);
        assert_eq!(sz, Size::new(2 * 6, 8)); // 6px/char, 8px tall at font 0
        assert_eq!(c.ops.len(), 2);
        assert_eq!(c.ops[0], DrawOp::FillRect(Rect::new(0, 0, 10, 5), Color::WHITE));
        assert_eq!(c.ops[1], DrawOp::Text(Point::new(1, 1), Color::BLACK));
    }
}
