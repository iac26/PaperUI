use paperui_core::{Canvas, Color, Constraints, DrawCtx, FontId, Point, Size, Theme, UpdateHint, FONT0_H, FONT0_W};
use paperui_widgets::WidgetTheme;

const PAD_X: i16 = 10;
const PAD_Y: i16 = 8;
const PAPER: Color = Color::WHITE;
const PAPER_PRESSED: Color = Color::GRAY;
const INK: Color = Color::BLACK;

/// The e-ink look: black ink on white paper, thick focused borders, text-quality hint.
/// Stateless singleton.
pub struct EinkTheme;
impl Theme for EinkTheme {}

impl<C: Canvas> WidgetTheme<C> for EinkTheme {
    fn measure_button(&self, label: &str, c: Constraints) -> Size {
        let w = label.chars().count() as i16 * FONT0_W + 2 * PAD_X;
        let h = FONT0_H + 2 * PAD_Y;
        c.clamp(Size::new(w, h))
    }

    fn draw_button(&self, ctx: &mut DrawCtx<C>, label: &str, pressed: bool) {
        ctx.require_hint(UpdateHint::Text);
        let b = ctx.bounds;
        ctx.canvas.fill_rect(b, if pressed { PAPER_PRESSED } else { PAPER });
        let width = if ctx.focused { 3 } else { 1 };
        ctx.canvas.stroke_rect(b, INK, width);
        ctx.canvas.text(Point::new(b.x + PAD_X, b.y + PAD_Y), label, FontId(0), INK);
    }
}
