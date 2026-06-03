use crate::{Canvas, Color, Constraints, DrawCtx, FontId, Point, Size, Theme};

/// Default text inset (px) and foreground for `WidgetTheme::draw_text`. Matches the layout PAD.
const DEFAULT_TEXT_INSET: i16 = 6;
const DEFAULT_TEXT_FG: Color = Color::rgb(0xE0, 0xE0, 0xE0);

/// The render contract for THIS widget set: one measure/draw pair per widget kind.
/// Themes (in the board addons `paperui-tft`/`paperui-eink`, or user crates)
/// implement it. Generic over the Canvas, so one theme works with any backend.
pub trait WidgetTheme<C: Canvas>: Theme {
    fn measure_button(&self, label: &str, c: Constraints) -> Size;
    fn draw_button(&self, ctx: &mut DrawCtx<C>, label: &str, pressed: bool);

    /// The surface background color (used to clear regions during animation). Default black.
    fn background(&self) -> Color { Color::BLACK }

    /// Draw text within `ctx.bounds`: clear to the background, then light glyphs. Themes may
    /// override for a different look (e.g. e-ink draws dark-on-light). Default suits a dark UI.
    fn draw_text(&self, ctx: &mut DrawCtx<C>, s: &str) {
        let bg = self.background();
        ctx.canvas.fill_rect(ctx.bounds, bg);
        ctx.canvas.text(
            Point::new(ctx.bounds.x + DEFAULT_TEXT_INSET, ctx.bounds.y + DEFAULT_TEXT_INSET),
            s,
            FontId(0),
            DEFAULT_TEXT_FG,
        );
    }
}
