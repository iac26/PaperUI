//! Host-test recording canvas: records each draw call into a fixed-capacity log.
use super::{Canvas, Color, FontId, FONT0_H, FONT0_W};
use crate::geometry::{Point, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawOp { FillRect(Rect, Color), StrokeRect(Rect, Color, u16), Text(Point, Color) }

/// Records each draw call into a fixed-capacity log.
pub struct MockCanvas { pub ops: heapless::Vec<DrawOp, 64> }

impl MockCanvas {
    pub fn new() -> Self { Self { ops: heapless::Vec::new() } }
}

impl Default for MockCanvas {
    fn default() -> Self { Self::new() }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
