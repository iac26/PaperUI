//! The render pass: re-run dirty reactive effects, then draw dirty nodes onto the canvas.
//! Buttons go through `WidgetTheme::draw_button`; text is drawn directly (fill + glyph) with
//! Layer-#1 placeholder colors — a themed text primitive is a later (widget-library) concern.
//! Drawing happens OUTSIDE `with_runtime` (we snapshot node data first), so the global lock
//! is never held across a canvas/theme call and never nests.

use crate::canvas::Canvas;
use crate::draw::DrawCtx;
use crate::geometry::{Point, Rect};
use crate::reactive::layout::layout;
use crate::reactive::node::Kind;
use crate::reactive::runtime::{run_effect_of, with_runtime, NodeId};
use crate::reactive::TEXT_CAP;
use crate::types::{Color, FontId, UpdateHint};
use crate::widget_theme::WidgetTheme;

// Layer-#1 placeholder text colors + inset. TEXT_PAD mirrors layout::PAD so the glyph sits
// inside the measured cell. (Text becomes a themed primitive in the widget-library layer.)
const TEXT_FG: Color = Color::BLACK;
const TEXT_BG: Color = Color::WHITE;
const TEXT_PAD: i16 = 6;

/// What a node needs drawn, snapshotted out of the runtime lock.
enum Draw {
    Text { bounds: Rect, content: heapless::String<TEXT_CAP> },
    Button { bounds: Rect, label: &'static str, pressed: bool },
    Container,
}

fn snapshot(node: NodeId) -> Draw {
    with_runtime(|rt| {
        let n = &rt.nodes[node.0 as usize];
        match &n.kind {
            Kind::Text { content, .. } => Draw::Text { bounds: n.bounds, content: content.clone() },
            Kind::Button { label, pressed, .. } => Draw::Button { bounds: n.bounds, label, pressed: *pressed },
            Kind::Column { .. } | Kind::Row { .. } => Draw::Container,
        }
    })
}

fn draw_node<C: Canvas, T: WidgetTheme<C>>(node: NodeId, canvas: &mut C, theme: &T) {
    match snapshot(node) {
        Draw::Text { bounds, content } => {
            canvas.fill_rect(bounds, TEXT_BG);
            canvas.text(Point::new(bounds.x + TEXT_PAD, bounds.y + TEXT_PAD), &content, FontId(0), TEXT_FG);
        }
        Draw::Button { bounds, label, pressed } => {
            let mut hint = UpdateHint::default();
            let mut ctx = DrawCtx::new(canvas, bounds, false, &mut hint);
            theme.draw_button(&mut ctx, label, pressed);
        }
        Draw::Container => {} // containers paint nothing; children are drawn by draw_subtree
    }
}

fn draw_subtree<C: Canvas, T: WidgetTheme<C>>(node: NodeId, canvas: &mut C, theme: &T) {
    draw_node(node, canvas, theme);
    let children = with_runtime(|rt| match &rt.nodes[node.0 as usize].kind {
        Kind::Column { children, .. } | Kind::Row { children, .. } => Some(children.clone()),
        _ => None,
    });
    if let Some(children) = children {
        for c in children.iter() {
            draw_subtree(*c, canvas, theme);
        }
    }
}

/// Render only dirty nodes: re-run their (Text) effects, reflow within the root's current
/// bounds, then draw just the dirty nodes. This is the surgical update.
pub fn render_frame<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    let dirty = with_runtime(|rt| {
        let d = rt.dirty.clone();
        rt.dirty.clear();
        d
    });
    for &n in dirty.iter() {
        run_effect_of(n); // recomputes Text content; no-op for non-reactive-text nodes
    }
    // Reflow within the root's current bounds so a Text whose content size changed re-places.
    // (In a Column, text height is fixed, so siblings never shift; safe for Layer #1 layouts.)
    let area = with_runtime(|rt| rt.nodes[root.0 as usize].bounds);
    layout(root, area);
    for &n in dirty.iter() {
        draw_node(n, canvas, theme);
    }
}

/// Full first render: draw the whole subtree, then clear any dirty accumulated during build.
pub fn render_frame_full<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    draw_subtree(root, canvas, theme);
    with_runtime(|rt| rt.dirty.clear());
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::canvas::{FONT0_H, FONT0_W};
    use crate::geometry::Size;
    use crate::reactive::node::{button, col, text};
    use crate::reactive::scope::Scope;
    use crate::types::Constraints;
    use crate::{DrawOp, MockCanvas, Theme};

    struct TestTheme;
    impl Theme for TestTheme {}
    impl WidgetTheme<MockCanvas> for TestTheme {
        fn measure_button(&self, label: &str, _c: Constraints) -> Size {
            Size::new(label.chars().count() as i16 * FONT0_W + 12, FONT0_H + 12)
        }
        fn draw_button(&self, ctx: &mut DrawCtx<MockCanvas>, label: &str, _pressed: bool) {
            let b = ctx.bounds;
            ctx.canvas.fill_rect(b, Color::GRAY);
            ctx.canvas.text(Point::new(b.x + 6, b.y + 6), label, FontId(0), Color::BLACK);
        }
    }

    /// Is this draw op entirely within `r`? Rect ops must be contained; the text op's anchor
    /// point must fall inside `r`.
    fn op_within(op: &DrawOp, r: Rect) -> bool {
        fn rect_in(a: Rect, b: Rect) -> bool {
            a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w && a.y + a.h <= b.y + b.h
        }
        match op {
            DrawOp::FillRect(a, _) => rect_in(*a, r),
            DrawOp::StrokeRect(a, _, _) => rect_in(*a, r),
            DrawOp::Text(p, _) => r.contains(*p),
        }
    }

    #[test]
    fn changing_a_signal_repaints_only_the_dependent_text() {
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let root = col(cx, (
            text(cx, move || {
                let mut s = heapless::String::<32>::new();
                let _ = core::fmt::write(&mut s, format_args!("{}", count.get()));
                s
            }),
            button(cx, "+", move || count.update(|c| *c += 1)),
        ));
        layout(root, Rect::new(0, 0, 200, 200));

        let mut canvas = MockCanvas::new();
        render_frame_full(root, &mut canvas, &TestTheme);
        canvas.ops.clear();

        // change the signal, then render only what's dirty
        count.set(5);
        render_frame(root, &mut canvas, &TestTheme);

        let text_bounds = with_runtime(|rt| {
            let c = match &rt.nodes[root.0 as usize].kind {
                Kind::Column { children, .. } => children[0],
                _ => unreachable!(),
            };
            rt.nodes[c.0 as usize].bounds
        });
        assert!(!canvas.ops.is_empty(), "the dependent text must repaint");
        assert!(
            canvas.ops.iter().all(|op| op_within(op, text_bounds)),
            "ONLY the text region should repaint, not the button"
        );
        cx.dispose();
    }
}
