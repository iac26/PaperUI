use crate::{Canvas, Constraints, DrawCtx, Size, Theme};

/// The render contract for THIS widget set: one measure/draw pair per widget kind.
/// Themes (in the board addons `paperui-tft`/`paperui-eink`, or user crates)
/// implement it. Generic over the Canvas, so one theme works with any backend.
pub trait WidgetTheme<C: Canvas>: Theme {
    fn measure_button(&self, label: &str, c: Constraints) -> Size;
    fn draw_button(&self, ctx: &mut DrawCtx<C>, label: &str, pressed: bool);
}
