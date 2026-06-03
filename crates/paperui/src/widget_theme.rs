use crate::{Canvas, Constraints, DrawCtx, Size, Theme};

/// The render contract for THIS widget set: one measure/draw pair per widget kind.
/// Themes (paperui-theme-*) implement it. Generic over the Canvas so the same
/// theme works with any backend surface.
pub trait WidgetTheme<C: Canvas>: Theme {
    fn measure_button(&self, label: &str, c: Constraints) -> Size;
    fn draw_button(&self, ctx: &mut DrawCtx<C>, label: &str, pressed: bool);
}
