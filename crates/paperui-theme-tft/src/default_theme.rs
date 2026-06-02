use paperui_core::{Canvas, Color, Constraints, DrawCtx, FontId, Point, Size, Theme, UpdateHint, FONT0_H, FONT0_W};
use paperui_widgets::WidgetTheme;

const PAD_X: i16 = 8;
const PAD_Y: i16 = 6;
const BG: Color = Color::rgb(0xE0, 0xE0, 0xE0);
const BG_PRESSED: Color = Color::rgb(0xA0, 0xA0, 0xA0);
const BORDER: Color = Color::rgb(0x40, 0x40, 0x40);
const BORDER_FOCUS: Color = Color::rgb(0x1E, 0x90, 0xFF);
const LABEL: Color = Color::BLACK;

/// The default color/TFT look. Stateless singleton — holds no data.
pub struct DefaultTheme;
impl Theme for DefaultTheme {}

impl<C: Canvas> WidgetTheme<C> for DefaultTheme {
    fn measure_button(&self, label: &str, c: Constraints) -> Size {
        let w = label.chars().count() as i16 * FONT0_W + 2 * PAD_X;
        let h = FONT0_H + 2 * PAD_Y;
        c.clamp(Size::new(w, h))
    }

    fn draw_button(&self, ctx: &mut DrawCtx<C>, label: &str, pressed: bool) {
        ctx.require_hint(UpdateHint::Text);
        let b = ctx.bounds;
        ctx.canvas.fill_rect(b, if pressed { BG_PRESSED } else { BG });
        let (border, width) = if ctx.focused { (BORDER_FOCUS, 2) } else { (BORDER, 1) };
        ctx.canvas.stroke_rect(b, border, width);
        let tx = b.x + PAD_X;
        let ty = b.y + PAD_Y;
        ctx.canvas.text(Point::new(tx, ty), label, FontId(0), LABEL);
    }
}
