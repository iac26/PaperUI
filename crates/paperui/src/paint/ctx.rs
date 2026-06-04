//! The per-draw context handed to themes, plus the e-ink refresh-quality hint it carries.
use super::Canvas;
use crate::geometry::Rect;

/// Semantic e-ink refresh-quality hint, set by the theme during draw.
/// Ordered worst-last so a region can take the max. Non-e-ink renderers ignore it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum UpdateHint { None, Mono, #[default] Fast, Text, Quality }

/// Per-draw context handed to a widget's `draw`. Carries the canvas, the widget's
/// absolute bounds, focus state, and an accumulator for the e-ink UpdateHint.
pub struct DrawCtx<'a, C: Canvas> {
    pub canvas: &'a mut C,
    pub bounds: Rect,
    pub focused: bool,
    hint: &'a mut UpdateHint,
}

impl<'a, C: Canvas> DrawCtx<'a, C> {
    pub fn new(canvas: &'a mut C, bounds: Rect, focused: bool, hint: &'a mut UpdateHint) -> Self {
        Self { canvas, bounds, focused, hint }
    }
    /// Theme calls this during draw to request an e-ink refresh quality.
    /// The region keeps the maximum (worst) hint requested.
    pub fn require_hint(&mut self, h: UpdateHint) {
        if h > *self.hint { *self.hint = h; }
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
}
