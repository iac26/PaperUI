//! The render pass: re-run dirty reactive effects, then draw dirty nodes onto the canvas.
//! Buttons go through `WidgetTheme::draw_button`; text goes through `WidgetTheme::draw_text`.
//! Drawing happens OUTSIDE `with_runtime` (we snapshot node data first), so the global lock
//! is never held across a canvas/theme call and never nests.

use crate::geometry::Rect;
use crate::paint::{Canvas, DrawCtx, UpdateHint, WidgetTheme};
use crate::reactive::layout::{layout, CAROUSEL_ROW_H, CAROUSEL_ROW_PITCH};
use crate::reactive::node::Kind;
use crate::reactive::runtime::{run_effect_of, with_runtime, NodeId};
use crate::reactive::{ANIM_STEPS, MAX_CHILDREN, TEXT_CAP, VISIBLE};

/// What a node needs drawn, snapshotted out of the runtime lock.
enum Draw {
    Text { bounds: Rect, content: heapless::String<TEXT_CAP> },
    Button { bounds: Rect, label: &'static str, pressed: bool, focused: bool },
    Container,
}

fn snapshot(node: NodeId) -> Draw {
    with_runtime(|rt| {
        let focused = rt.focus == Some(node);
        let n = &rt.nodes[node.0 as usize];
        match &n.kind {
            Kind::Text { content, .. } => Draw::Text { bounds: n.bounds, content: content.clone() },
            Kind::Button { label, pressed, .. } => {
                Draw::Button { bounds: n.bounds, label, pressed: *pressed, focused }
            }
            Kind::Column { .. } | Kind::Row { .. } | Kind::Carousel { .. } => Draw::Container,
        }
    })
}

fn draw_node<C: Canvas, T: WidgetTheme<C>>(node: NodeId, canvas: &mut C, theme: &T) {
    match snapshot(node) {
        Draw::Text { bounds, content } => {
            let mut hint = UpdateHint::default();
            let mut ctx = DrawCtx::new(canvas, bounds, false, &mut hint);
            theme.draw_text(&mut ctx, &content);
        }
        Draw::Button { bounds, label, pressed, focused } => {
            let mut hint = UpdateHint::default();
            let mut ctx = DrawCtx::new(canvas, bounds, focused, &mut hint);
            theme.draw_button(&mut ctx, label, pressed);
        }
        Draw::Container => {} // containers paint nothing; children are drawn by draw_subtree
    }
}

fn draw_subtree<C: Canvas, T: WidgetTheme<C>>(node: NodeId, canvas: &mut C, theme: &T) {
    draw_node(node, canvas, theme);
    let children = with_runtime(|rt| match &rt.nodes[node.0 as usize].kind {
        Kind::Column { children, .. } | Kind::Row { children, .. } => Some(children.clone()),
        Kind::Carousel { children, offset, .. } => {
            // Visible window is VISIBLE slots starting at the top index `offset`, wrapping.
            let off = *offset;
            let n = children.len();
            let mut v: heapless::Vec<NodeId, MAX_CHILDREN> = heapless::Vec::new();
            if n > 0 {
                for k in 0..VISIBLE {
                    let _ = v.push(children[(off + k) % n]);
                }
            }
            Some(v)
        }
        _ => None,
    });
    if let Some(children) = children {
        for c in children.iter() {
            draw_subtree(*c, canvas, theme);
        }
    }
}

/// Render only dirty nodes: re-run their (Text) effects, reflow within the root's current
/// bounds, then draw just the dirty nodes. This is the surgical update. Internal: callers
/// drive a frame through [`render_tick`].
fn render_frame<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
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

/// Render one frame of a carousel slide: clear the carousel region, draw the visible window plus
/// the one incoming row shifted by the current `slide`, then repaint non-carousel root children
/// (e.g. the status line) on top as a crude clip mask. Internal: [`render_tick`] calls this once
/// per loop while a slide is animating.
fn render_anim_frame<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    let info = with_runtime(|rt| {
        let cnode = rt
            .nodes
            .iter()
            .position(|nd| matches!(&nd.kind, Kind::Carousel { anim_dir, .. } if *anim_dir != 0))
            .map(|i| NodeId(i as u16))?;
        match &rt.nodes[cnode.0 as usize].kind {
            Kind::Carousel { children, selected, offset, anim_frame, anim_dir, .. } => Some((
                cnode,
                rt.nodes[cnode.0 as usize].bounds,
                children.clone(),
                *selected,
                *offset,
                *anim_frame,
                *anim_dir,
            )),
            _ => None,
        }
    });
    let Some((cnode, cb, children, sel, off, frame, dir)) = info else { return; };
    let n = children.len();
    if n == 0 { return; }
    let slide = dir as i16 * CAROUSEL_ROW_PITCH * frame as i16 / ANIM_STEPS as i16;

    canvas.fill_rect(cb, theme.background());

    // Draw the VISIBLE slots plus one incoming slot (above on down-scroll, below on up-scroll),
    // each at its slot row shifted by `slide`. Slot indices wrap; the centered (selected) item is
    // highlighted. `off` is the top-slot index.
    let (slot_lo, slot_hi): (i32, i32) =
        if dir > 0 { (-1, VISIBLE as i32) } else { (0, VISIBLE as i32 + 1) };
    for slot in slot_lo..slot_hi {
        let idx = (off as i32 + slot).rem_euclid(n as i32) as usize;
        let yy = cb.y + slot as i16 * CAROUSEL_ROW_PITCH + slide;
        let label = with_runtime(|rt| match &rt.nodes[children[idx].0 as usize].kind {
            Kind::Button { label, .. } => Some(*label),
            _ => None,
        });
        if let Some(label) = label {
            let mut hint = UpdateHint::default();
            let mut ctx = DrawCtx::new(canvas, Rect::new(cb.x, yy, cb.w, CAROUSEL_ROW_H), idx == sel, &mut hint);
            theme.draw_button(&mut ctx, label, false);
        }
    }

    // Clip the slide: the incoming/outgoing slots are drawn up to one pitch beyond `cb`, and
    // there is no real clip primitive, so repaint the background over the one-pitch strips just
    // above and below the viewport. Without this, each frame's overflow accumulates as shadows
    // in the gap above the carousel and the wallpaper below.
    let top = (cb.y - CAROUSEL_ROW_PITCH).max(0);
    canvas.fill_rect(Rect::new(cb.x, top, cb.w, cb.y - top), theme.background());
    canvas.fill_rect(Rect::new(cb.x, cb.y + cb.h, cb.w, CAROUSEL_ROW_PITCH), theme.background());

    // Overpaint mask: redraw root's non-carousel children (the status overlay) on top.
    let overlays = with_runtime(|rt| match &rt.nodes[root.0 as usize].kind {
        Kind::Column { children, .. } | Kind::Row { children, .. } => {
            let mut v: heapless::Vec<NodeId, MAX_CHILDREN> = heapless::Vec::new();
            for &c in children.iter() {
                if c != cnode { let _ = v.push(c); }
            }
            Some(v)
        }
        _ => None,
    });
    if let Some(overlays) = overlays {
        for c in overlays.iter() {
            draw_subtree(*c, canvas, theme);
        }
    }
}

/// One frame of the run loop, shared by the embedded `driver::run` and the desktop `run_sim`.
/// If a carousel slide is animating, advance it one frame; otherwise surgically repaint any
/// dirty nodes (a no-op when nothing is dirty).
pub fn render_tick<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    use crate::reactive::node::{any_carousel_animating, step_carousel_anim};
    if with_runtime(|rt| any_carousel_animating(rt)) {
        render_anim_frame(root, canvas, theme);
        with_runtime(step_carousel_anim);
    } else if with_runtime(|rt| !rt.dirty.is_empty()) {
        render_frame(root, canvas, theme);
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::geometry::{Constraints, Point, Size};
    use crate::paint::{Color, FontId, FONT0_H, FONT0_W};
    use crate::reactive::node::{button, carousel, carousel_select_first, col, nav, text, text_static};
    use crate::reactive::scope::Scope;
    use crate::{DrawOp, MockCanvas};

    struct TestTheme;
    impl WidgetTheme<MockCanvas> for TestTheme {
        fn measure_button(&self, label: &str, _c: Constraints) -> Size {
            Size::new(label.chars().count() as i16 * FONT0_W + 12, FONT0_H + 12)
        }
        fn draw_button(&self, ctx: &mut DrawCtx<MockCanvas>, label: &str, _pressed: bool) {
            let b = ctx.bounds;
            ctx.canvas.fill_rect(b, Color::GRAY);
            ctx.canvas.stroke_rect(b, Color::BLACK, if ctx.focused { 2 } else { 1 });
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

    #[test]
    fn focused_button_draws_with_focused_true() {
        let cx = Scope::root();
        let b = button(cx, "X", || {});
        let root = col(cx, (text_static(cx, "t"), b));
        layout(root, Rect::new(0, 0, 200, 200));
        with_runtime(|rt| rt.focus = Some(b));
        let mut canvas = MockCanvas::new();
        render_frame_full(root, &mut canvas, &TestTheme);
        let has_focus_stroke = canvas.ops.iter().any(|op| matches!(op, DrawOp::StrokeRect(_, _, 2)));
        assert!(has_focus_stroke, "focused button must draw focused=true");

        // And with focus cleared, no button draws the focused (width-2) stroke.
        with_runtime(|rt| rt.focus = None);
        let mut canvas2 = MockCanvas::new();
        render_frame_full(root, &mut canvas2, &TestTheme);
        let any_focus_stroke = canvas2.ops.iter().any(|op| matches!(op, DrawOp::StrokeRect(_, _, 2)));
        assert!(!any_focus_stroke, "no button should draw focused=true when focus is cleared");
        cx.dispose();
    }

    #[test]
    fn carousel_draws_only_visible_window() {
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}), button(cx, "4", || {}), button(cx, "5", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        layout(car, Rect::new(0, 0, 240, 120));
        let mut canvas = MockCanvas::new();
        render_frame_full(car, &mut canvas, &TestTheme);
        let fills = canvas.ops.iter().filter(|op| matches!(op, DrawOp::FillRect(_, _))).count();
        assert_eq!(fills, 3, "only the 3 visible carousel buttons paint");
        cx.dispose();
    }

    #[test]
    fn anim_frame_shifts_rows_and_overpaints_status_last() {
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}), button(cx, "4", || {}), button(cx, "5", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        let status = text_static(cx, "STATUS");
        let root = col(cx, (status, car));
        layout(root, Rect::new(0, 0, 240, 135));
        // scroll to arm the animation (2->3 scrolls; dir = +1, offset = 1)
        nav(1); nav(1); nav(1);
        let mut canvas = MockCanvas::new();
        render_anim_frame(root, &mut canvas, &TestTheme);
        // carousel region cleared first
        assert!(matches!(canvas.ops[0], DrawOp::FillRect(_, _)), "carousel region cleared first");
        // during a slide, the visible window (3) PLUS one incoming row are drawn = 4 buttons
        let strokes = canvas.ops.iter().filter(|op| matches!(op, DrawOp::StrokeRect(_, _, _))).count();
        assert_eq!(strokes, 4, "VISIBLE rows + 1 incoming row drawn during the slide");
        // status overlay is drawn LAST (on top, masking any intrusion)
        assert!(matches!(canvas.ops.last(), Some(DrawOp::Text(_, _))), "status overlay drawn last");
        // disarm so this carousel doesn't pollute other tests.
        for _ in 0..=(crate::reactive::ANIM_STEPS as usize) {
            with_runtime(|rt| crate::reactive::node::step_carousel_anim(rt));
        }
        cx.dispose();
    }
}
