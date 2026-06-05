//! Host-test recording canvas: records each draw call into a fixed-capacity log.
use super::{Canvas, Color, FontId, FONT0_H, FONT0_W};
use crate::geometry::{Point, Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawOp {
    FillRect(Rect, Color),
    StrokeRect(Rect, Color, u16),
    Text(Point, Color),
    Clip(Option<Rect>),
}

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
    fn set_clip(&mut self, clip: Option<Rect>) {
        let _ = self.ops.push(DrawOp::Clip(clip));
    }
}

/// Returns `true` iff every `FillRect`/`StrokeRect` rect and `Text` point appearing
/// *after* a `Clip(Some(clip))` op lies within `clip`. Used by widget layout tests
/// to assert that container children don't draw outside their bounds.
#[cfg(test)]
pub(crate) fn ops_after_clip_within(ops: &[DrawOp], clip: Rect) -> bool {
    let mut clipped = false;
    for op in ops {
        match op {
            DrawOp::Clip(Some(c)) if *c == clip => { clipped = true; }
            DrawOp::Clip(_) => { clipped = false; }
            DrawOp::FillRect(r, _) | DrawOp::StrokeRect(r, _, _) if clipped => {
                // both corners must be inside the clip
                if !clip.contains(Point::new(r.x, r.y))
                    || !clip.contains(Point::new(r.x + r.w - 1, r.y + r.h - 1))
                {
                    return false;
                }
            }
            DrawOp::Text(p, _) if clipped => {
                if !clip.contains(*p) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Color;

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

    #[test]
    fn set_clip_records_clip_op_and_subsequent_fill_rect() {
        let mut c = MockCanvas::new();
        let clip = Rect::new(0, 0, 10, 10);
        c.set_clip(Some(clip));
        c.fill_rect(Rect::new(1, 1, 5, 5), Color::WHITE);
        assert_eq!(c.ops.len(), 2);
        assert_eq!(c.ops[0], DrawOp::Clip(Some(clip)));
        assert_eq!(c.ops[1], DrawOp::FillRect(Rect::new(1, 1, 5, 5), Color::WHITE));
    }

    #[test]
    fn set_clip_none_records_none_variant() {
        let mut c = MockCanvas::new();
        c.set_clip(None);
        assert_eq!(c.ops.len(), 1);
        assert_eq!(c.ops[0], DrawOp::Clip(None));
    }

    #[test]
    fn ops_after_clip_within_passes_for_rects_inside_clip() {
        let mut c = MockCanvas::new();
        let clip = Rect::new(0, 0, 20, 20);
        c.set_clip(Some(clip));
        c.fill_rect(Rect::new(1, 1, 5, 5), Color::WHITE);
        c.stroke_rect(Rect::new(2, 2, 4, 4), Color::BLACK, 1);
        assert!(ops_after_clip_within(&c.ops, clip));
    }

    #[test]
    fn ops_after_clip_within_fails_for_rect_outside_clip() {
        let mut c = MockCanvas::new();
        let clip = Rect::new(0, 0, 10, 10);
        c.set_clip(Some(clip));
        // This rect extends beyond the clip boundary
        c.fill_rect(Rect::new(5, 5, 20, 20), Color::WHITE);
        assert!(!ops_after_clip_within(&c.ops, clip));
    }

    #[test]
    fn ops_after_clip_within_ignores_ops_before_clip() {
        let mut c = MockCanvas::new();
        let clip = Rect::new(0, 0, 10, 10);
        // Draw outside any clip — should not be checked
        c.fill_rect(Rect::new(100, 100, 50, 50), Color::WHITE);
        c.set_clip(Some(clip));
        // Draw inside clip
        c.fill_rect(Rect::new(1, 1, 5, 5), Color::WHITE);
        assert!(ops_after_clip_within(&c.ops, clip));
    }
}
