use crate::canvas::Canvas;
use crate::draw::DrawCtx;
use crate::geometry::Size;
use crate::types::Constraints;

/// A UI node's LOGIC. Drawing/sizing are delegated to the theme `T` (which the
/// widgets crate constrains to `WidgetTheme`). The core leaves `T` UNBOUNDED so
/// it names no concrete theme or widget kind.
pub trait Widget<C: Canvas, T> {
    fn measure(&self, theme: &T, c: Constraints) -> Size;
    fn draw(&self, ctx: &mut DrawCtx<C>, theme: &T);
    fn focusable(&self) -> bool { false }
    fn on_activate(&mut self) {}
    fn on_adjust(&mut self, _dir: i8) {}
    fn on_focus_changed(&mut self, _focused: bool) {}
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::{Canvas, Color, Constraints, DrawCtx, MockCanvas, Rect, Size, Theme, UpdateHint};

    // A trivial theme + widget defined inline to exercise the trait wiring.
    struct TestTheme;
    impl Theme for TestTheme {}

    struct Dot;
    impl<C: Canvas> Widget<C, TestTheme> for Dot {
        fn measure(&self, _t: &TestTheme, c: Constraints) -> Size { c.clamp(Size::new(4, 4)) }
        fn draw(&self, ctx: &mut DrawCtx<C>, _t: &TestTheme) {
            ctx.require_hint(UpdateHint::Mono);
            ctx.canvas.fill_rect(ctx.bounds, Color::BLACK);
        }
    }

    #[test]
    fn widget_draws_into_ctx_and_sets_hint() {
        let mut canvas = MockCanvas::new();
        let mut hint = UpdateHint::None;
        let bounds = Rect::new(2, 2, 4, 4);
        {
            let mut ctx = DrawCtx::new(&mut canvas, bounds, false, &mut hint);
            Dot.draw(&mut ctx, &TestTheme);
        }
        assert_eq!(hint, UpdateHint::Mono);
        assert_eq!(canvas.ops.len(), 1);
    }

    #[test]
    fn default_widget_flags() {
        // `focusable` is a Widget<C,T> method that doesn't use C/T, so the type
        // params must be pinned explicitly (no value to infer them from).
        assert!(!Widget::<MockCanvas, TestTheme>::focusable(&Dot));
    }
}
