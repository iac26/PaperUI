//! The retained view tree. Builders allocate nodes into the runtime arena and return ids.

use crate::geometry::{Point, Rect};
use crate::reactive::anim::{Anim, Animated, Easing};
use crate::reactive::layout::CAROUSEL_ROW_PITCH;
use crate::reactive::runtime::{with_runtime, EffectId, NodeId, OwnerId, Runtime};
use crate::reactive::scope::Scope;
use crate::reactive::{MAX_CHILDREN, TEXT_CAP};
use heapless::Vec;

// --- carousel behavior (centered window + animated slide); NOT memory capacities ---
/// Rows shown at once in a carousel (the selection sits in the middle).
pub const VISIBLE: usize = 3;

/// Mark a node dirty (idempotent). Used when focus moves so the affected buttons repaint.
fn mark_dirty(rt: &mut Runtime<'_>, node: Option<NodeId>) {
    if let Some(n) = node {
        if !rt.dirty.contains(&n) {
            let _ = rt.dirty.push(n);
        }
    }
}

/// Hit-test a pointer/touch, then run the handler of the `Button` whose laid-out `bounds`
/// contain `p`. If that button lives in a carousel, the carousel scrolls so the clicked row
/// becomes the centered selection (animated, like an arrow press) and selection stays == focus;
/// a plain button simply takes focus. Only the VISIBLE rows are on-screen, so a carousel click is
/// always within ±(VISIBLE/2) slots of center. On overlap the LAST matching button (latest in the
/// arena = drawn on top) wins; no-op when no button is hit. Focus/selection are set inside the
/// lock; the handler runs OUTSIDE it (take-out / put-back), matching `invoke_handler_of_focus`.
pub(crate) fn invoke_handler_at(p: Point) {
    let mut armed = None;
    let hit = with_runtime(|rt| {
        let mut found = None;
        for (i, n) in rt.nodes.iter().enumerate() {
            if matches!(n.kind, Kind::Button { .. }) && n.bounds.contains(p) {
                found = Some(NodeId(i as u16));
            }
        }
        let id = found?;
        match carousel_owning(rt, id) {
            Some(cnode) => {
                // Re-center the clicked row; delta 0 means it's already the centered selection.
                if let Some(delta) = carousel_slot_delta(rt, cnode, id) {
                    if delta != 0 {
                        armed = carousel_nav_inner(rt, cnode, delta);
                    }
                }
            }
            None => {
                let old = rt.focus;
                rt.focus = Some(id);
                mark_dirty(rt, old);
                mark_dirty(rt, Some(id));
            }
        }
        Some(id)
    });
    // Arm the slide (re-enters the lock) only after the hit-test critical section is released.
    arm_carousel_slide(armed);
    if let Some(node) = hit {
        crate::reactive::runtime::invoke_handler_of(node);
    }
}

#[derive(Clone)]
pub enum TextSource {
    Static(&'static str),
    Reactive(EffectId),
}

#[derive(Clone)]
pub enum Kind {
    Text { src: TextSource, content: heapless::String<TEXT_CAP> },
    Button { label: &'static str, on_press: EffectId, pressed: bool },
    Column { children: Vec<NodeId, MAX_CHILDREN>, spacing: i16 },
    Row { children: Vec<NodeId, MAX_CHILDREN>, spacing: i16 },
    Carousel { children: Vec<NodeId, MAX_CHILDREN>, selected: usize, offset: usize, slide: Animated<i16> },
}

#[derive(Clone)]
pub struct Node {
    pub kind: Kind,
    pub bounds: Rect,
    pub owner: OwnerId,
}

/// A const placeholder occupying the unused tail of an app's `[Node; NN]` arena. `push_node`
/// overwrites slot `len` before it is ever read, so this value is never rendered or laid out.
pub(crate) const EMPTY_NODE: Node = Node {
    kind: Kind::Text {
        src: TextSource::Static(""),
        content: heapless::String::new(),
    },
    bounds: Rect::new(0, 0, 0, 0),
    owner: OwnerId(0),
};

pub fn text_static(cx: Scope, s: &'static str) -> NodeId {
    let mut content = heapless::String::<TEXT_CAP>::new();
    let _ = content.push_str(s); // truncates past TEXT_CAP; acceptable for Layer #1
    with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Text { src: TextSource::Static(s), content },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    })
}

pub(crate) fn effect_of_text(rt: &Runtime<'_>, node: NodeId) -> Option<EffectId> {
    match &rt.nodes[node.0 as usize].kind {
        Kind::Text { src: TextSource::Reactive(eid), .. } => Some(*eid),
        _ => None,
    }
}

pub(crate) fn handler_of_button(rt: &Runtime<'_>, node: NodeId) -> Option<EffectId> {
    match &rt.nodes[node.0 as usize].kind {
        Kind::Button { on_press, .. } => Some(*on_press),
        _ => None,
    }
}

/// Forward focus to the next Button node (wrapping), operating on an already-locked runtime.
fn focus_next_inner(rt: &mut Runtime<'_>) {
    let n = rt.nodes.len();
    if n == 0 { return; }
    let old = rt.focus;
    let cur = rt.focus.map(|f| f.0 as usize);
    for off in 1..=n {
        let i = match cur { Some(c) => (c + off) % n, None => off - 1 };
        if matches!(rt.nodes[i].kind, Kind::Button { .. }) {
            let new = NodeId(i as u16);
            rt.focus = Some(new);
            // Repaint both the button losing focus and the one gaining it (surgical highlight).
            mark_dirty(rt, old);
            mark_dirty(rt, Some(new));
            return;
        }
    }
}

/// Backward focus to the previous Button node (wrapping).
fn focus_prev_inner(rt: &mut Runtime<'_>) {
    let n = rt.nodes.len();
    if n == 0 { return; }
    let old = rt.focus;
    let cur = rt.focus.map(|f| f.0 as usize);
    for off in 1..=n {
        let i = match cur { Some(c) => (c + n - off) % n, None => n - off };
        if matches!(rt.nodes[i].kind, Kind::Button { .. }) {
            let new = NodeId(i as u16);
            rt.focus = Some(new);
            // Repaint both the button losing focus and the one gaining it (surgical highlight).
            mark_dirty(rt, old);
            mark_dirty(rt, Some(new));
            return;
        }
    }
}

/// The Carousel node that has `node` among its children, else `None`.
fn carousel_owning(rt: &Runtime<'_>, node: NodeId) -> Option<NodeId> {
    for (i, n) in rt.nodes.iter().enumerate() {
        if let Kind::Carousel { children, .. } = &n.kind {
            if children.contains(&node) {
                return Some(NodeId(i as u16));
            }
        }
    }
    None
}

/// The Carousel node that OWNS the current focus, else `None`. Returning `None` when no carousel
/// owns the focus lets `nav` fall back to plain focus cycling — and keeps the shared global test
/// arena honest (leftover carousels never hijack a non-carousel focus).
fn find_carousel(rt: &Runtime<'_>) -> Option<NodeId> {
    carousel_owning(rt, rt.focus?)
}

/// How far `target` sits from the centered (selected) slot in `cnode`'s visible window: the
/// signed slot distance (for VISIBLE = 3, top slot = -1, center = 0, bottom = +1), or `None`
/// if `target` is not in the currently visible window. Used to map a click on a visible row to
/// the carousel nav delta that re-centers it.
fn carousel_slot_delta(rt: &Runtime<'_>, cnode: NodeId, target: NodeId) -> Option<i32> {
    if let Kind::Carousel { children, offset, .. } = &rt.nodes[cnode.0 as usize].kind {
        let n = children.len();
        if n == 0 {
            return None;
        }
        let center = VISIBLE as i32 / 2;
        for k in 0..VISIBLE {
            if children[(*offset + k) % n] == target {
                return Some(k as i32 - center);
            }
        }
    }
    None
}

/// Move carousel selection by `delta`, wrapping. The selection always sits in the MIDDLE slot
/// (centered carousel): `offset` is the index shown in the TOP slot = the item just "above" the
/// selection. Does the instant logical jump (selection/offset/focus + dirty the carousel node)
/// while the caller already holds `rt`, and RETURNS `(slide, d)` — the slide handle and the one-
/// pitch displacement to arm it with. The caller arms it (`set_immediate`/`animate_to`) OUTSIDE
/// the lock via [`arm_carousel_slide`], because those re-enter `with_runtime` and must not nest.
fn carousel_nav_inner(rt: &mut Runtime<'_>, cnode: NodeId, delta: i32) -> Option<(Animated<i16>, i16)> {
    let (children, sel) = match &rt.nodes[cnode.0 as usize].kind {
        Kind::Carousel { children, selected, .. } => (children.clone(), *selected),
        _ => return None,
    };
    let n = children.len();
    if n == 0 { return None; }
    let new_sel = (sel as isize + delta as isize).rem_euclid(n as isize) as usize;
    let new_off = (new_sel + n - 1) % n; // top slot = the item above the centered selection
    let slide = match &mut rt.nodes[cnode.0 as usize].kind {
        Kind::Carousel { selected, offset, slide, .. } => {
            *selected = new_sel;
            *offset = new_off;
            *slide
        }
        _ => return None,
    };
    rt.focus = Some(children[new_sel]);
    // The carousel self-draws its clipped window now, so dirty the CAROUSEL node (not the child
    // buttons, which must never paint unclipped). The first repaint shows the armed (one-pitch-off)
    // window; the slide's `dirty_node` keeps it dirty for each subsequent animation tick.
    if !rt.dirty.contains(&cnode) {
        let _ = rt.dirty.push(cnode);
    }
    // Rows start one pitch off (below on a down-scroll, above on an up-scroll) and glide to 0.
    let d = if delta >= 0 { CAROUSEL_ROW_PITCH } else { -CAROUSEL_ROW_PITCH };
    Some((slide, d))
}

/// Arm a carousel slide OUTSIDE the runtime lock: jump it one pitch off, then animate it back to 0
/// so the new window slides into place. Both calls re-enter `with_runtime`, so this must run after
/// the nav critical section has been released.
fn arm_carousel_slide(armed: Option<(Animated<i16>, i16)>) {
    if let Some((slide, d)) = armed {
        slide.set_immediate(d);
        slide.animate_to(0);
    }
}

/// Public navigation entry: one critical section for the logical jump, then arm the slide OUTSIDE
/// it (the slide methods re-enter `with_runtime`). Carousel-aware, else plain button cycling.
pub(crate) fn nav(delta: i32) {
    let armed = with_runtime(|rt| match find_carousel(rt) {
        Some(cnode) => carousel_nav_inner(rt, cnode, delta),
        None => {
            if delta >= 0 { focus_next_inner(rt) } else { focus_prev_inner(rt) }
            None
        }
    });
    arm_carousel_slide(armed);
}

/// Run the focused node's handler (if any). Reads focus inside the lock, then invokes OUTSIDE
/// it (invoke_handler_of uses the take-out/put-back pattern and must not be nested).
pub(crate) fn invoke_handler_of_focus() {
    let focused = with_runtime(|rt| rt.focus);
    if let Some(node) = focused {
        crate::reactive::runtime::invoke_handler_of(node);
    }
}

pub(crate) fn set_text_content(rt: &mut Runtime<'_>, node: NodeId, s: &str) {
    if let Kind::Text { content, .. } = &mut rt.nodes[node.0 as usize].kind {
        content.clear();
        let _ = content.push_str(s);
    }
}

pub(crate) fn set_text_effect(rt: &mut Runtime<'_>, node: NodeId, eid: EffectId) {
    if let Kind::Text { src, .. } = &mut rt.nodes[node.0 as usize].kind {
        *src = TextSource::Reactive(eid);
    }
}

pub fn text(cx: Scope, mut f: impl FnMut() -> heapless::String<TEXT_CAP> + 'static) -> NodeId {
    let node = with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Text { src: TextSource::Static(""), content: heapless::String::new() },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    });
    let eid = cx.handler(move || {
        let s = f();
        with_runtime(|rt| set_text_content(rt, node, &s));
    });
    with_runtime(|rt| set_text_effect(rt, node, eid));
    // Run once so initial content + the signal subscription are established (run-once model).
    crate::reactive::runtime::run_effect_of(node);
    node
}

pub fn button(cx: Scope, label: &'static str, on_press: impl FnMut() + 'static) -> NodeId {
    let eid = cx.handler(on_press);
    with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Button { label, on_press: eid, pressed: false },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    })
}

/// Fixed-arity children via tuples. Implement for the arities Layer #1 needs (2 and 3).
pub trait IntoChildren {
    fn collect(self) -> Vec<NodeId, MAX_CHILDREN>;
}
impl IntoChildren for (NodeId, NodeId) {
    fn collect(self) -> Vec<NodeId, MAX_CHILDREN> {
        let mut v = Vec::new();
        let _ = v.push(self.0);
        let _ = v.push(self.1);
        v
    }
}
impl IntoChildren for (NodeId, NodeId, NodeId) {
    fn collect(self) -> Vec<NodeId, MAX_CHILDREN> {
        let mut v = Vec::new();
        let _ = v.push(self.0);
        let _ = v.push(self.1);
        let _ = v.push(self.2);
        v
    }
}

pub fn col(cx: Scope, children: impl IntoChildren) -> NodeId {
    with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Column { children: children.collect(), spacing: 4 },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    })
}
pub fn row(cx: Scope, children: impl IntoChildren) -> NodeId {
    with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Row { children: children.collect(), spacing: 4 },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    })
}

pub fn carousel(cx: Scope, items: &[NodeId]) -> NodeId {
    let mut children: Vec<NodeId, MAX_CHILDREN> = Vec::new();
    for &it in items {
        let _ = children.push(it); // truncates past MAX_CHILDREN; acceptable for Layer #1
    }
    // The slide offset (px) the visible rows are drawn at, tweened to 0 each nav. Allocate it,
    // push the node, then point the animator at the node — three SEQUENTIAL locks, never nested.
    let slide = cx.tween(0i16, Anim { dur_ms: 300, easing: Easing::Linear });
    let nid = with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Carousel { children, selected: 0, offset: 0, slide },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    });
    slide.set_dirty_node(nid); // so the driver's advance_anims repaints the carousel each tick
    nid
}

/// Reset a carousel to its first item and focus it (call once after building the tree).
pub fn carousel_select_first(c: NodeId) {
    with_runtime(|rt| {
        let first = match &mut rt.nodes[c.0 as usize].kind {
            Kind::Carousel { children, selected, offset, .. } => {
                let n = children.len();
                *selected = 0;
                // `offset` = top-slot index = the item just above the centered selection (wraps).
                *offset = n.saturating_sub(1);
                children.first().copied()
            }
            _ => None,
        };
        rt.focus = first;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::anim::advance_anims;
    use crate::reactive::runtime::fresh_runtime;

    #[test]
    fn empty_node_is_const_array_initializable() {
        const _NODES: [Node; 3] = [EMPTY_NODE, EMPTY_NODE, EMPTY_NODE];
        assert!(matches!(_NODES[0].kind, Kind::Text { .. }));
    }

    #[test]
    fn col_of_two_texts_allocates_three_nodes() {
        fresh_runtime();
        let cx = Scope::root();
        let n0 = with_runtime(|rt| rt.nodes.len());
        let _root = col(cx, (text_static(cx, "a"), text_static(cx, "b")));
        let n1 = with_runtime(|rt| rt.nodes.len());
        assert_eq!(n1 - n0, 3, "two texts + one column");
        cx.dispose();
    }

    #[test]
    fn reactive_text_runs_closure_and_tracks_signal() {
        fresh_runtime();
        let cx = Scope::root();
        let count = cx.signal(0i32);
        // a reactive text that reads `count`
        let t = text(cx, move || {
            let mut s = heapless::String::<32>::new();
            let _ = core::fmt::write(&mut s, format_args!("{}", count.get()));
            s
        });
        // text() runs the effect once at creation, so the subscription is already registered;
        // running it again is idempotent.
        crate::reactive::runtime::run_effect_of(t);
        let subscribed = with_runtime(|rt| rt.signals.iter().any(|s| s.in_use && s.subs.contains(&t)));
        assert!(subscribed, "reactive text must subscribe to the signal it reads");
        cx.dispose();
    }

    #[test]
    fn button_handler_mutates_signal_on_invoke() {
        fresh_runtime();
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let b = button(cx, "inc", move || count.update(|c| *c += 1));
        crate::reactive::runtime::invoke_handler_of(b);
        assert_eq!(count.get(), 1);
        cx.dispose();
    }

    #[test]
    fn pointer_inside_button_runs_its_handler_and_focuses_it() {
        fresh_runtime();
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let b = button(cx, "+", move || count.update(|c| *c += 1));
        // Far-away bounds: the node arena is global + append-only across tests, so place this
        // button where no other test's laid-out node can also contain the point.
        with_runtime(|rt| {
            rt.nodes[b.0 as usize].bounds = Rect::new(1000, 1000, 40, 20);
            rt.dirty.clear();
        });
        invoke_handler_at(Point::new(1010, 1010));
        assert_eq!(count.get(), 1, "the button under the point runs its handler");
        assert_eq!(with_runtime(|rt| rt.focus), Some(b), "and gains focus");
        assert!(with_runtime(|rt| rt.dirty.contains(&b)), "and is marked dirty to repaint");
        cx.dispose();
    }

    #[test]
    fn pointer_outside_any_button_does_not_run_our_handler() {
        fresh_runtime();
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let b = button(cx, "+", move || count.update(|c| *c += 1));
        with_runtime(|rt| rt.nodes[b.0 as usize].bounds = Rect::new(1000, 1000, 40, 20));
        invoke_handler_at(Point::new(5, 5)); // nowhere near our button
        assert_eq!(count.get(), 0, "no button at the point => our handler never runs");
        cx.dispose();
    }

    #[test]
    fn overlapping_buttons_last_one_wins() {
        fresh_runtime();
        let cx = Scope::root();
        let a_count = cx.signal(0i32);
        let b_count = cx.signal(0i32);
        let a = button(cx, "a", move || a_count.update(|c| *c += 1));
        let b = button(cx, "b", move || b_count.update(|c| *c += 1));
        let r = Rect::new(2000, 2000, 50, 50);
        with_runtime(|rt| {
            rt.nodes[a.0 as usize].bounds = r;
            rt.nodes[b.0 as usize].bounds = r; // same rect → overlap
        });
        invoke_handler_at(Point::new(2010, 2010));
        assert_eq!(a_count.get(), 0, "earlier button is skipped");
        assert_eq!(b_count.get(), 1, "later (top-most) button wins");
        assert_eq!(with_runtime(|rt| rt.focus), Some(b));
        cx.dispose();
    }

    #[test]
    fn focus_next_dirties_old_and_new_focused_buttons() {
        fresh_runtime();
        let cx = Scope::root();
        let b0 = button(cx, "a", || {});
        let b1 = button(cx, "b", || {});
        // b1 is the node immediately after b0, so nav(1) from b0 lands on b1.
        with_runtime(|rt| {
            rt.focus = Some(b0);
            rt.dirty.clear();
        });
        nav(1);
        with_runtime(|rt| {
            assert_eq!(rt.focus, Some(b1), "focus advanced to the next button");
            assert!(rt.dirty.contains(&b0), "old focus repaints");
            assert!(rt.dirty.contains(&b1), "new focus repaints");
        });
        cx.dispose();
    }

    #[test]
    fn carousel_holds_children_and_starts_at_zero() {
        fresh_runtime();
        let cx = Scope::root();
        let items = [
            button(cx, "a", || {}), button(cx, "b", || {}), button(cx, "c", || {}),
            button(cx, "d", || {}),
        ];
        let car = carousel(cx, &items);
        with_runtime(|rt| match &rt.nodes[car.0 as usize].kind {
            Kind::Carousel { children, selected, offset, slide } => {
                assert_eq!(children.len(), 4);
                assert_eq!((*selected, *offset), (0, 0));
                assert_eq!(slide.get_in(rt), 0, "slide rests at 0");
            }
            _ => panic!("expected a carousel"),
        });
        cx.dispose();
    }

    #[test]
    fn carousel_select_first_centers_first_and_focuses_it() {
        fresh_runtime();
        let cx = Scope::root();
        let a = button(cx, "a", || {});
        let car = carousel(cx, &[a, button(cx, "b", || {}), button(cx, "c", || {})]);
        carousel_select_first(car);
        with_runtime(|rt| {
            assert_eq!(rt.focus, Some(a), "first item is focused (centered)");
            if let Kind::Carousel { selected, offset, .. } = &rt.nodes[car.0 as usize].kind {
                assert_eq!(*selected, 0);
                assert_eq!(*offset, 2, "top slot wraps to the last item (n-1)");
            }
        });
        cx.dispose();
    }

    #[test]
    fn every_nav_centers_selection_and_arms_animation() {
        fresh_runtime();
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}), button(cx, "4", || {}), button(cx, "5", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        nav(1); // 0 -> 1 (down-scroll: rows start one pitch BELOW and glide up to 0)
        let slide = with_runtime(|rt| match &rt.nodes[car.0 as usize].kind {
            Kind::Carousel { selected, offset, slide, .. } => {
                assert_eq!(*selected, 1, "selection advanced and stays centered");
                assert_eq!(*offset, 0, "top slot = item above the centered selection");
                assert_eq!(slide.get_in(rt), CAROUSEL_ROW_PITCH, "down-scroll arms slide one pitch off");
                *slide
            }
            _ => unreachable!(),
        });
        assert!(with_runtime(|rt| rt.animators[slide.anim.0 as usize].active), "every nav arms the slide");
        with_runtime(|rt| assert_eq!(rt.focus, Some(items[1]), "focus follows the centered item"));
        // Drain the slide so the shared runtime isn't left animating for the next test.
        with_runtime(|rt| advance_anims(rt, 1000));
        cx.dispose();
    }

    #[test]
    fn click_on_a_visible_row_centers_and_activates_it() {
        fresh_runtime();
        use crate::reactive::layout::layout;
        let cx = Scope::root();
        let hits = cx.signal(0i32);
        // After select_first the visible slots are top=item5, center=item0, bottom=item1, so
        // item 1 is the bottom (clickable) row; it bumps `hits` when its handler runs.
        let items = [
            button(cx, "0", || {}),
            button(cx, "1", move || hits.update(|c| *c += 1)),
            button(cx, "2", || {}), button(cx, "3", || {}),
            button(cx, "4", || {}), button(cx, "5", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        // Lay out far from other tests' nodes (the global arena is shared) so the hit-test on the
        // click point can only match our own row.
        layout(car, Rect::new(5000, 5000, 240, 135));
        let b = with_runtime(|rt| rt.nodes[items[1].0 as usize].bounds);
        invoke_handler_at(Point::new(b.x + b.w / 2, b.y + b.h / 2));
        with_runtime(|rt| {
            assert_eq!(rt.focus, Some(items[1]), "clicked row becomes the focus");
            if let Kind::Carousel { selected, .. } = &rt.nodes[car.0 as usize].kind {
                assert_eq!(*selected, 1, "clicked row is re-centered as the selection");
            }
        });
        assert_eq!(hits.get(), 1, "select + activate: the row's handler also runs");
        // Drain the armed slide so the shared runtime isn't left animating for the next test.
        with_runtime(|rt| advance_anims(rt, 1000));
        cx.dispose();
    }

    #[test]
    fn nav_wraps_at_both_ends() {
        fresh_runtime();
        let cx = Scope::root();
        let items = [button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {})];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        nav(-1); // up from first wraps to last
        with_runtime(|rt| if let Kind::Carousel { selected, .. } = &rt.nodes[car.0 as usize].kind {
            assert_eq!(*selected, 2, "up from first wraps to last");
        });
        nav(1); // down from last wraps to first
        with_runtime(|rt| if let Kind::Carousel { selected, .. } = &rt.nodes[car.0 as usize].kind {
            assert_eq!(*selected, 0, "down from last wraps to first");
        });
        with_runtime(|rt| advance_anims(rt, 1000));
        cx.dispose();
    }

    #[test]
    fn slide_animates_then_deactivates() {
        fresh_runtime();
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        with_runtime(|rt| rt.now_ms = 0);
        nav(1); // arms the slide (set_immediate one pitch, animate_to 0)
        let slide = with_runtime(|rt| match &rt.nodes[car.0 as usize].kind {
            Kind::Carousel { slide, .. } => *slide,
            _ => unreachable!(),
        });
        assert!(with_runtime(|rt| rt.animators[slide.anim.0 as usize].active), "scroll armed the slide");
        // Past the 300ms duration the slide rests at 0 and the animator deactivates.
        with_runtime(|rt| advance_anims(rt, 300));
        assert_eq!(with_runtime(|rt| slide.get_in(rt)), 0, "slide settles back to 0");
        assert!(!with_runtime(|rt| rt.animators[slide.anim.0 as usize].active), "animation clears after the duration");
        cx.dispose();
    }
}
