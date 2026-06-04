//! The TFT (full-color) theme — this board addon's default look: black wallpaper, orange highlight.
use paperui::{
    Canvas, Color, Constraints, DrawCtx, FontId, Point, Size, UpdateHint, WidgetTheme,
    FONT0_H, FONT0_W,
};

const PAD_X: i16 = 8;
const PAD_Y: i16 = 6;
const BG: Color = Color::BLACK;
const BTN_FILL: Color = Color::rgb(0x20, 0x20, 0x20);
const BTN_FILL_PRESSED: Color = Color::rgb(0x40, 0x40, 0x40);
const BORDER: Color = Color::rgb(0x60, 0x60, 0x60);
const ACCENT: Color = Color::rgb(0xFF, 0x80, 0x00); // orange highlight
const LABEL: Color = Color::rgb(0xE0, 0xE0, 0xE0);

/// The full-color TFT look: black wallpaper, orange highlight on the focused control.
pub struct TftTheme;

impl<C: Canvas> WidgetTheme<C> for TftTheme {
    fn measure_button(&self, label: &str, c: Constraints) -> Size {
        let w = label.chars().count() as i16 * FONT0_W + 2 * PAD_X;
        let h = FONT0_H + 2 * PAD_Y;
        c.clamp(Size::new(w, h))
    }

    fn draw_button(&self, ctx: &mut DrawCtx<C>, label: &str, pressed: bool) {
        ctx.require_hint(UpdateHint::Text);
        let b = ctx.bounds;
        ctx.canvas.fill_rect(b, if pressed { BTN_FILL_PRESSED } else { BTN_FILL });
        let (border, width, label_color) = if ctx.focused {
            (ACCENT, 2, ACCENT)
        } else {
            (BORDER, 1, LABEL)
        };
        ctx.canvas.stroke_rect(b, border, width);
        ctx.canvas.text(Point::new(b.x + PAD_X, b.y + PAD_Y), label, FontId(0), label_color);
    }

    fn background(&self) -> Color { BG }

    fn draw_text(&self, ctx: &mut DrawCtx<C>, s: &str) {
        ctx.canvas.fill_rect(ctx.bounds, BG);
        ctx.canvas.text(Point::new(ctx.bounds.x + 6, ctx.bounds.y + 6), s, FontId(0), LABEL);
    }
}
