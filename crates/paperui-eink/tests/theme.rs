use paperui::{
    Button, Color, Constraints, DrawCtx, DrawOp, MockCanvas, Rect, Size, UpdateHint, Widget,
    WidgetTheme,
};
use paperui_eink::EinkTheme;

#[test]
fn eink_theme_measures_button_with_eink_padding() {
    let theme = EinkTheme;
    let size = <EinkTheme as WidgetTheme<MockCanvas>>::measure_button(
        &theme,
        "OK",
        Constraints::new(0, 0, 1000, 1000),
    );
    assert_eq!(size, Size::new(32, 24));
}

#[test]
fn eink_theme_draws_paper_bg_black_border_and_label_and_sets_text_hint() {
    let theme = EinkTheme;
    let mut canvas = MockCanvas::new();
    let mut hint = UpdateHint::None;
    let btn = Button::new("OK");
    {
        let mut ctx = DrawCtx::new(&mut canvas, Rect::new(0, 0, 32, 24), false, &mut hint);
        Widget::<MockCanvas, EinkTheme>::draw(&btn, &mut ctx, &theme);
    }
    assert!(matches!(canvas.ops[0], DrawOp::FillRect(_, _)));
    assert!(matches!(canvas.ops[1], DrawOp::StrokeRect(_, _, _)));
    assert!(matches!(canvas.ops[2], DrawOp::Text(_, _)));
    assert_eq!(canvas.ops[0], DrawOp::FillRect(Rect::new(0, 0, 32, 24), Color::WHITE));
    assert_eq!(hint, UpdateHint::Text);
}

#[test]
fn focused_button_uses_thicker_black_border() {
    let theme = EinkTheme;
    let mut canvas = MockCanvas::new();
    let mut hint = UpdateHint::None;
    let btn = Button::new("OK");
    {
        let mut ctx = DrawCtx::new(&mut canvas, Rect::new(0, 0, 32, 24), true, &mut hint);
        Widget::<MockCanvas, EinkTheme>::draw(&btn, &mut ctx, &theme);
    }
    if let DrawOp::StrokeRect(_, color, width) = canvas.ops[1] {
        assert_eq!(width, 3, "focused e-ink border is thicker (no color highlight)");
        assert_eq!(color, Color::BLACK);
    } else {
        panic!("expected a stroke as op[1]");
    }
}
