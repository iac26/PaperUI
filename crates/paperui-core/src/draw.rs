use crate::canvas::Canvas;
use crate::geometry::Rect;
use crate::types::UpdateHint;

/// Opaque theme marker. The core knows nothing beyond this; the widgets crate
/// defines the real `WidgetTheme` render contract as a sub-trait.
pub trait Theme {}

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
