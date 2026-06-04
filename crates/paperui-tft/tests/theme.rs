use paperui::{
    Color, Constraints, DrawCtx, DrawOp, MockCanvas, Rect, Size, UpdateHint, WidgetTheme,
};
use paperui_tft::TftTheme;

#[test]
fn default_theme_measures_button_from_label_plus_padding() {
    let theme = TftTheme;
    // "OK" = 2 chars * 6px = 12 + 2*8 horizontal padding = 28 wide; 8 + 2*6 = 20 tall.
    let size = <TftTheme as WidgetTheme<MockCanvas>>::measure_button(&theme, "OK", Constraints::new(0, 0, 1000, 1000));
    assert_eq!(size, Size::new(28, 20));
}

#[test]
fn default_theme_draws_button_background_border_and_label() {
    let theme = TftTheme;
    let mut canvas = MockCanvas::new();
    let mut hint = UpdateHint::None;
    let bounds = Rect::new(0, 0, 28, 20);
    {
        let mut ctx = DrawCtx::new(&mut canvas, bounds, /*focused*/ false, &mut hint);
        theme.draw_button(&mut ctx, "OK", false);
    }
    // Expect: filled background, a border stroke, then the label text.
    assert!(matches!(canvas.ops[0], DrawOp::FillRect(_, _)));
    assert!(matches!(canvas.ops[1], DrawOp::StrokeRect(_, _, _)));
    assert!(matches!(canvas.ops[2], DrawOp::Text(_, _)));
    // TFT theme is a color panel: it requests Text-quality (ignored by non-eink).
    assert_eq!(hint, UpdateHint::Text);
}

#[test]
fn focused_button_uses_a_distinct_border_but_same_op_shape() {
    let theme = TftTheme;
    let mut canvas = MockCanvas::new();
    let mut hint = UpdateHint::None;
    {
        let mut ctx = DrawCtx::new(&mut canvas, Rect::new(0, 0, 28, 20), /*focused*/ true, &mut hint);
        theme.draw_button(&mut ctx, "OK", false);
    }
    if let DrawOp::StrokeRect(_, color, width) = canvas.ops[1] {
        assert_eq!(width, 2, "focused border is thicker");
        assert_eq!(color, Color::rgb(0xFF, 0x80, 0x00), "focused border is orange accent");
    } else {
        panic!("expected a stroke as op[1]");
    }
}
