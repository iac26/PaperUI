//! The render pass: re-run dirty reactive effects, then draw dirty nodes onto the canvas.
//! Buttons go through `WidgetTheme::draw_button`; text goes through `WidgetTheme::draw_text`.
//! Drawing happens OUTSIDE `with_runtime` (we snapshot node data first), so the global lock
//! is never held across a canvas/theme call and never nests.

use crate::geometry::Rect;
use crate::paint::{Canvas, DrawCtx, UpdateHint, WidgetTheme};
use crate::reactive::layout::{layout, CAROUSEL_ROW_H, CAROUSEL_ROW_PITCH};
use crate::reactive::node::Kind;
use crate::reactive::runtime::{run_effect_of, with_runtime, NodeId};
use crate::reactive::{TEXT_CAP, VISIBLE};

/// One slot of a carousel's drawn window: the row's label and whether it is the centered selection.
/// `VISIBLE + 1` slots are snapshotted (one extra above the window) so a mid-slide row peeking in
/// from the top is drawn too; the scissor clips whatever extends past the window.
type CarouselSlot = (&'static str, bool);

/// What a node needs drawn, snapshotted out of the runtime lock.
enum Draw {
    Text { bounds: Rect, content: heapless::String<TEXT_CAP> },
    Button { bounds: Rect, label: &'static str, pressed: bool, focused: bool },
    /// A self-rendering clipped window: `cb` is the carousel's bounds, `slide` the current px
    /// offset, and `slots[i]` the label/focus for slot `i - 1` (so `slots[0]` is the row one pitch
    /// above the window, `slots[1]` the top visible row, etc.).
    Carousel { cb: Rect, slide: i16, slots: heapless::Vec<CarouselSlot, { VISIBLE + 1 }> },
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
            Kind::Carousel { children, selected, offset, slide } => {
                // Read the animated slide WITH the lock already held (get_in, never get).
                let slide = slide.get_in(rt);
                let cb = n.bounds;
                let count = children.len();
                let mut slots: heapless::Vec<CarouselSlot, { VISIBLE + 1 }> = heapless::Vec::new();
                if count > 0 {
                    // Slots -1 .. VISIBLE: the visible window plus one row above it.
                    for slot in -1..(VISIBLE as i32) {
                        let idx = (*offset as i32 + slot).rem_euclid(count as i32) as usize;
                        let label = match &rt.nodes[children[idx].0 as usize].kind {
                            Kind::Button { label, .. } => *label,
                            _ => "",
                        };
                        let _ = slots.push((label, idx == *selected));
                    }
                }
                Draw::Carousel { cb, slide, slots }
            }
            Kind::Column { .. } | Kind::Row { .. } => Draw::Container,
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
        Draw::Carousel { cb, slide, slots } => {
            // Scissor to the window, clear it, then draw each slot row shifted by `slide`. Rows that
            // glide past the window (the extra slot, the partially-exited bottom row) are clipped by
            // the scissor — no overdraw strips, no overlay, no sibling references.
            canvas.set_clip(Some(cb));
            canvas.fill_rect(cb, theme.background());
            for (i, &(label, focused)) in slots.iter().enumerate() {
                let slot = i as i32 - 1; // slots[0] is slot -1 (one pitch above the window)
                let y = cb.y + slot as i16 * CAROUSEL_ROW_PITCH + slide;
                let mut hint = UpdateHint::default();
                let mut ctx = DrawCtx::new(canvas, Rect::new(cb.x, y, cb.w, CAROUSEL_ROW_H), focused, &mut hint);
                theme.draw_button(&mut ctx, label, false);
            }
            canvas.set_clip(None);
        }
        Draw::Container => {} // containers paint nothing; children are drawn by draw_subtree
    }
}

fn draw_subtree<C: Canvas, T: WidgetTheme<C>>(node: NodeId, canvas: &mut C, theme: &T) {
    draw_node(node, canvas, theme);
    // The carousel draws its own rows in `draw_node` (clipped window), so we do NOT recurse into
    // its children here — only Column/Row containers have children we descend into.
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
/// bounds, then draw just the dirty nodes. This is the surgical update. Internal: callers
/// drive a frame through [`render_tick`].
fn render_frame<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    // Snapshot the dirty count and index the live set across the passes below. Precondition
    // (Layer 1): reactive effects are read-only w.r.t. signals — none call `signal.set` during
    // recompute — so the dirty set never grows here and indices [0..count) stay valid. If that
    // is ever relaxed, the trailing `clear()` would silently drop nodes dirtied mid-frame; then
    // this must instead drain only [0..count) (retain the tail) or loop until the set stabilizes.
    let count = with_runtime(|rt| rt.dirty.len());
    for i in 0..count {
        let n = with_runtime(|rt| rt.dirty.as_slice()[i]);
        run_effect_of(n);
    }
    let area = with_runtime(|rt| rt.nodes[root.0 as usize].bounds);
    layout(root, area);
    for i in 0..count {
        let n = with_runtime(|rt| rt.dirty.as_slice()[i]);
        draw_node(n, canvas, theme);
    }
    with_runtime(|rt| rt.dirty.clear());
}

/// Full first render: draw the whole subtree, then clear any dirty accumulated during build.
pub fn render_frame_full<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    draw_subtree(root, canvas, theme);
    with_runtime(|rt| rt.dirty.clear());
}

/// One frame of the run loop, shared by the embedded `driver::run` and the desktop `run_sim`.
/// Surgically repaint any dirty nodes (a no-op when nothing is dirty). While a slide animates, the
/// driver's `advance_anims` dirties the carousel via its `dirty_node` each tick, so the normal
/// surgical repaint here redraws the carousel's clipped window — no separate animation path.
pub fn render_tick<C: Canvas, T: WidgetTheme<C>>(root: NodeId, canvas: &mut C, theme: &T) {
    if with_runtime(|rt| !rt.dirty.is_empty()) {
        render_frame(root, canvas, theme);
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::geometry::{Constraints, Point, Size};
    use crate::paint::{Color, FontId, FONT0_H, FONT0_W};
    use crate::reactive::anim::advance_anims;
    use crate::reactive::node::{button, carousel, carousel_select_first, col, nav, text, text_static};
    use crate::reactive::runtime::fresh_runtime;
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
            DrawOp::Clip(_) => true, // a clip op draws nothing; never a containment violation
        }
    }

    #[test]
    fn changing_a_signal_repaints_only_the_dependent_text() {
        fresh_runtime();
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
        fresh_runtime();
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
        fresh_runtime();
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
        // The carousel self-draws a bounded window: one row per slot (VISIBLE + 1: the window plus
        // the one row just above it), each a button = one stroke. Far fewer than all 6 children —
        // the rows outside the window are never emitted; the scissor confines the overhang.
        let strokes = canvas.ops.iter().filter(|op| matches!(op, DrawOp::StrokeRect(_, _, _))).count();
        assert_eq!(strokes, VISIBLE + 1, "only the windowed rows paint, not all 6 children");
        cx.dispose();
    }

    #[test]
    fn carousel_window_confines_drawing() {
        fresh_runtime();
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
        with_runtime(|rt| rt.now_ms = 0);
        nav(1); // arm a slide
        // Advance mid-slide (300ms total): the rows are part-way through their glide, so a row peeks
        // in from the top and the bottom row exits — the moment the scissor has to do real work.
        with_runtime(|rt| advance_anims(rt, 50));
        let cb = with_runtime(|rt| rt.nodes[car.0 as usize].bounds);

        let mut canvas = MockCanvas::new();
        render_frame(root, &mut canvas, &TestTheme);

        // Structural confinement is the real guarantee: the carousel brackets ALL its draws with the
        // scissor, so on hardware nothing paints outside cb — even though rows mid-slide deliberately
        // overhang (a row peeks in from the top, the bottom row exits). The mock can't crop pixels,
        // so we assert the bracket: the FIRST op opens the scissor on cb, the next clears cb to the
        // background, the LAST op resets the scissor, and no draw escapes that [open, close] window.
        assert!(
            matches!(canvas.ops.first(), Some(DrawOp::Clip(Some(c))) if *c == cb),
            "the carousel scissors to its window before any draw"
        );
        assert!(
            matches!(canvas.ops[1], DrawOp::FillRect(r, _) if r == cb),
            "the window is cleared to the background right after the scissor opens"
        );
        assert!(
            matches!(canvas.ops.last(), Some(DrawOp::Clip(None))),
            "the scissor is reset after the window so later widgets aren't clipped"
        );
        // Every other op falls strictly between the opening and closing clip — nothing is drawn
        // outside the scissor (which would otherwise paint over the status line above the carousel).
        let inner_clips = canvas.ops.iter().filter(|op| matches!(op, DrawOp::Clip(_))).count();
        assert_eq!(inner_clips, 2, "exactly one scissor open + one reset bracket all the draws");
        // Drain so this carousel doesn't leave the shared runtime animating for the next test.
        with_runtime(|rt| advance_anims(rt, 1000));
        cx.dispose();
    }
}
