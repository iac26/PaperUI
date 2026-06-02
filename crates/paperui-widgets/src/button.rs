use paperui_core::{Canvas, Constraints, DrawCtx, Size, Widget};
use crate::widget_theme::WidgetTheme;

/// Pure-logic button. Holds its label, pressed state, and a function-pointer
/// click handler (no heap — matches the static-memory design). Rendering is the
/// theme's job via `draw_button`.
pub struct Button {
    label: &'static str,
    pressed: bool,
    on_click: Option<fn()>,
}

impl Button {
    pub fn new(label: &'static str) -> Self { Self { label, pressed: false, on_click: None } }
    pub fn on_click(mut self, cb: fn()) -> Self { self.on_click = Some(cb); self }
    pub fn label(&self) -> &str { self.label }
    pub fn is_pressed(&self) -> bool { self.pressed }
}

impl<C: Canvas, T: WidgetTheme<C>> Widget<C, T> for Button {
    fn measure(&self, theme: &T, c: Constraints) -> Size { theme.measure_button(self.label, c) }
    fn draw(&self, ctx: &mut DrawCtx<C>, theme: &T) { theme.draw_button(ctx, self.label, self.pressed) }
    fn focusable(&self) -> bool { true }
    fn on_activate(&mut self) { if let Some(cb) = self.on_click { cb(); } }
}

/// Minimal concrete Canvas/Theme used only to name the generic trait in unit tests.
#[cfg(test)]
pub(crate) mod tests_support {
    use paperui_core::{Canvas, Color, FontId, Point, Rect, Size, Theme};

    pub struct NoCanvas;
    impl Canvas for NoCanvas {
        fn fill_rect(&mut self, _r: Rect, _c: Color) {}
        fn stroke_rect(&mut self, _r: Rect, _c: Color, _w: u16) {}
        fn text(&mut self, _a: Point, s: &str, _f: FontId, _c: Color) -> Size {
            Size::new(s.len() as i16, 8)
        }
    }
    pub struct NoTheme;
    impl Theme for NoTheme {}
    impl super::WidgetTheme<NoCanvas> for NoTheme {
        fn measure_button(&self, _l: &str, c: paperui_core::Constraints) -> Size { c.clamp(Size::new(0, 0)) }
        fn draw_button(&self, _ctx: &mut paperui_core::DrawCtx<NoCanvas>, _l: &str, _p: bool) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};

    static FIRED: AtomicBool = AtomicBool::new(false);
    fn on_click() { FIRED.store(true, Ordering::SeqCst); }

    #[test]
    fn button_is_focusable_and_activate_fires_callback() {
        use super::tests_support::{NoCanvas, NoTheme};
        FIRED.store(false, Ordering::SeqCst);
        let mut b = Button::new("OK").on_click(on_click);
        // Pin C/T to name the generic Widget trait (these methods ignore C/T).
        assert!(paperui_core::Widget::<NoCanvas, NoTheme>::focusable(&b));
        paperui_core::Widget::<NoCanvas, NoTheme>::on_activate(&mut b);
        assert!(FIRED.load(Ordering::SeqCst));
    }
}
