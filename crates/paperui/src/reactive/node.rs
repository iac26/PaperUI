//! The retained view tree. Builders allocate nodes into the runtime arena and return ids.

use crate::geometry::{Point, Rect};
use crate::reactive::runtime::{with_runtime, EffectId, NodeId, OwnerId, Runtime};
use crate::reactive::scope::Scope;
use crate::reactive::{ANIM_STEPS, MAX_CHILDREN, TEXT_CAP, VISIBLE};
use heapless::Vec;

/// Mark a node dirty (idempotent). Used when focus moves so the affected buttons repaint.
fn mark_dirty(rt: &mut Runtime, node: Option<NodeId>) {
    if let Some(n) = node {
        if !rt.dirty.contains(&n) {
            let _ = rt.dirty.push(n);
        }
    }
}

/// Hit-test: run the handler of the `Button` whose laid-out `bounds` contain `p`, focus it,
/// and dirty the old + new focused buttons so the highlight repaints. On overlap the LAST
/// matching button (latest in the arena = drawn on top) wins; no-op when no button is hit.
/// Focus/dirty are set inside the lock; the handler runs OUTSIDE it (take-out / put-back),
/// so there is no `with_runtime` nesting — matching `invoke_handler_of_focus`.
pub(crate) fn invoke_handler_at(p: Point) {
    let hit = with_runtime(|rt| {
        let mut found = None;
        for (i, n) in rt.nodes.iter().enumerate() {
            if matches!(n.kind, Kind::Button { .. }) && n.bounds.contains(p) {
                found = Some(NodeId(i as u16));
            }
        }
        if let Some(id) = found {
            let old = rt.focus;
            rt.focus = Some(id);
            mark_dirty(rt, old);
            mark_dirty(rt, Some(id));
        }
        found
    });
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
    Carousel { children: Vec<NodeId, MAX_CHILDREN>, selected: usize, offset: usize, anim_frame: u8, anim_dir: i8 },
}

#[derive(Clone)]
pub struct Node {
    pub kind: Kind,
    pub bounds: Rect,
    pub owner: OwnerId,
}

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

pub(crate) fn effect_of_text(rt: &Runtime, node: NodeId) -> Option<EffectId> {
    match &rt.nodes[node.0 as usize].kind {
        Kind::Text { src: TextSource::Reactive(eid), .. } => Some(*eid),
        _ => None,
    }
}

pub(crate) fn handler_of_button(rt: &Runtime, node: NodeId) -> Option<EffectId> {
    match &rt.nodes[node.0 as usize].kind {
        Kind::Button { on_press, .. } => Some(*on_press),
        _ => None,
    }
}

/// Forward focus to the next Button node (wrapping), operating on an already-locked runtime.
fn focus_next_inner(rt: &mut Runtime) {
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
fn focus_prev_inner(rt: &mut Runtime) {
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

/// The Carousel node that OWNS the current focus, else `None`. Returning `None` when no carousel
/// owns the focus lets `nav` fall back to plain focus cycling — and keeps the shared global test
/// arena honest (leftover carousels never hijack a non-carousel focus).
fn find_carousel(rt: &Runtime) -> Option<NodeId> {
    let focus = rt.focus?;
    for (i, node) in rt.nodes.iter().enumerate() {
        if let Kind::Carousel { children, .. } = &node.kind {
            if children.contains(&focus) {
                return Some(NodeId(i as u16));
            }
        }
    }
    None
}

/// Move carousel selection by `delta`, scrolling the window and moving focus. Arms the slide
/// animation when the window actually scrolls (offset changed). Dirties the visible slots.
fn carousel_nav_inner(rt: &mut Runtime, cnode: NodeId, delta: i32) {
    let (children, sel, off) = match &rt.nodes[cnode.0 as usize].kind {
        Kind::Carousel { children, selected, offset, .. } => (children.clone(), *selected, *offset),
        _ => return,
    };
    let n = children.len();
    if n == 0 { return; }
    let new_sel = (sel as i32 + delta).clamp(0, n as i32 - 1) as usize;
    let new_off = window_offset(n, new_sel, off);
    let scrolled = new_off != off;
    if let Kind::Carousel { selected, offset, anim_frame, anim_dir, .. } = &mut rt.nodes[cnode.0 as usize].kind {
        *selected = new_sel;
        *offset = new_off;
        if scrolled {
            *anim_frame = ANIM_STEPS;
            *anim_dir = if delta >= 0 { 1 } else { -1 };
        }
    }
    rt.focus = Some(children[new_sel]);
    let end = (new_off + VISIBLE).min(n);
    for i in new_off..end {
        let nid = children[i];
        if !rt.dirty.contains(&nid) {
            let _ = rt.dirty.push(nid);
        }
    }
}

/// True if any carousel has an animation in progress (`anim_dir != 0`).
pub(crate) fn any_carousel_animating(rt: &Runtime) -> bool {
    rt.nodes.iter().any(|nd| matches!(&nd.kind, Kind::Carousel { anim_dir, .. } if *anim_dir != 0))
}

/// Advance every animating carousel by one frame. At frame 0 the resting frame has been drawn,
/// so clear the animation and drop pending dirties (the anim path already painted them).
pub(crate) fn step_carousel_anim(rt: &mut Runtime) {
    let mut finished = false;
    for nd in rt.nodes.iter_mut() {
        if let Kind::Carousel { anim_frame, anim_dir, .. } = &mut nd.kind {
            if *anim_dir != 0 {
                if *anim_frame == 0 {
                    *anim_dir = 0;
                    finished = true;
                } else {
                    *anim_frame -= 1;
                }
            }
        }
    }
    if finished {
        rt.dirty.clear();
    }
}

/// Public navigation entry: one critical section. Carousel-aware, else plain button cycling.
pub(crate) fn nav(delta: i32) {
    with_runtime(|rt| match find_carousel(rt) {
        Some(cnode) => carousel_nav_inner(rt, cnode, delta),
        None => if delta >= 0 { focus_next_inner(rt) } else { focus_prev_inner(rt) },
    });
}

/// Run the focused node's handler (if any). Reads focus inside the lock, then invokes OUTSIDE
/// it (invoke_handler_of uses the take-out/put-back pattern and must not be nested).
pub(crate) fn invoke_handler_of_focus() {
    let focused = with_runtime(|rt| rt.focus);
    if let Some(node) = focused {
        crate::reactive::runtime::invoke_handler_of(node);
    }
}

pub(crate) fn set_text_content(rt: &mut Runtime, node: NodeId, s: &str) {
    if let Kind::Text { content, .. } = &mut rt.nodes[node.0 as usize].kind {
        content.clear();
        let _ = content.push_str(s);
    }
}

pub(crate) fn set_text_effect(rt: &mut Runtime, node: NodeId, eid: EffectId) {
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
    with_runtime(|rt| {
        rt.push_node(Node {
            kind: Kind::Carousel { children, selected: 0, offset: 0, anim_frame: 0, anim_dir: 0 },
            bounds: Rect::new(0, 0, 0, 0),
            owner: cx.owner,
        })
    })
}

/// Reset a carousel to its first item and focus it (call once after building the tree).
pub fn carousel_select_first(c: NodeId) {
    with_runtime(|rt| {
        let first = match &mut rt.nodes[c.0 as usize].kind {
            Kind::Carousel { children, selected, offset, .. } => {
                *selected = 0;
                *offset = 0;
                children.first().copied()
            }
            _ => None,
        };
        rt.focus = first;
    });
}

/// Scroll-to-keep-visible window math: return the offset that keeps `selected` inside the
/// `VISIBLE`-sized window, clamped to a valid range. Pure (no runtime access) for easy testing.
pub(crate) fn window_offset(n: usize, selected: usize, offset: usize) -> usize {
    let mut off = offset;
    if selected < off {
        off = selected;
    } else if selected >= off + VISIBLE {
        off = selected + 1 - VISIBLE;
    }
    off.min(n.saturating_sub(VISIBLE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_of_two_texts_allocates_three_nodes() {
        let cx = Scope::root();
        let n0 = with_runtime(|rt| rt.nodes.len());
        let _root = col(cx, (text_static(cx, "a"), text_static(cx, "b")));
        let n1 = with_runtime(|rt| rt.nodes.len());
        assert_eq!(n1 - n0, 3, "two texts + one column");
        cx.dispose();
    }

    #[test]
    fn reactive_text_runs_closure_and_tracks_signal() {
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
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let b = button(cx, "inc", move || count.update(|c| *c += 1));
        crate::reactive::runtime::invoke_handler_of(b);
        assert_eq!(count.get(), 1);
        cx.dispose();
    }

    #[test]
    fn pointer_inside_button_runs_its_handler_and_focuses_it() {
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
        let cx = Scope::root();
        let items = [
            button(cx, "a", || {}), button(cx, "b", || {}), button(cx, "c", || {}),
            button(cx, "d", || {}),
        ];
        let car = carousel(cx, &items);
        with_runtime(|rt| match &rt.nodes[car.0 as usize].kind {
            Kind::Carousel { children, selected, offset, anim_frame, anim_dir } => {
                assert_eq!(children.len(), 4);
                assert_eq!((*selected, *offset, *anim_frame, *anim_dir), (0, 0, 0, 0));
            }
            _ => panic!("expected a carousel"),
        });
        cx.dispose();
    }

    #[test]
    fn window_offset_scrolls_to_keep_selected_visible() {
        assert_eq!(window_offset(6, 0, 0), 0);
        assert_eq!(window_offset(6, 2, 0), 0);
        assert_eq!(window_offset(6, 3, 0), 1);
        assert_eq!(window_offset(6, 5, 1), 3);
        assert_eq!(window_offset(6, 5, 0), 3);
        assert_eq!(window_offset(6, 2, 3), 2);
        assert_eq!(window_offset(6, 0, 3), 0);
        assert_eq!(window_offset(2, 1, 0), 0);
    }

    #[test]
    fn carousel_select_first_sets_focus_to_first_child() {
        let cx = Scope::root();
        let a = button(cx, "a", || {});
        let car = carousel(cx, &[a, button(cx, "b", || {})]);
        carousel_select_first(car);
        with_runtime(|rt| assert_eq!(rt.focus, Some(a)));
        cx.dispose();
    }

    #[test]
    fn scroll_arms_animation_but_in_window_move_does_not() {
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}), button(cx, "4", || {}), button(cx, "5", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        // 0->1: in-window, no animation.
        nav(1);
        with_runtime(|rt| if let Kind::Carousel { anim_dir, .. } = &rt.nodes[car.0 as usize].kind {
            assert_eq!(*anim_dir, 0, "in-window move does not animate");
        });
        // 1->2 (in window), 2->3 (scroll -> animate).
        nav(1);
        nav(1);
        with_runtime(|rt| if let Kind::Carousel { anim_dir, anim_frame, .. } = &rt.nodes[car.0 as usize].kind {
            assert_eq!(*anim_dir, 1, "scroll down arms a forward animation");
            assert_eq!(*anim_frame, crate::reactive::ANIM_STEPS);
        });
        // disarm so this carousel doesn't pollute other tests' animation scans.
        for _ in 0..=(crate::reactive::ANIM_STEPS as usize) {
            with_runtime(|rt| step_carousel_anim(rt));
        }
        cx.dispose();
    }

    #[test]
    fn step_counts_down_then_clears() {
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        nav(1); nav(1); nav(1); // 0->1->2->3 ; 2->3 scrolls (offset 0->1) -> arms
        let armed = with_runtime(|rt| matches!(&rt.nodes[car.0 as usize].kind,
            Kind::Carousel { anim_dir, .. } if *anim_dir != 0));
        assert!(armed, "scroll armed this carousel");
        for _ in 0..=(crate::reactive::ANIM_STEPS as usize) {
            with_runtime(|rt| step_carousel_anim(rt));
        }
        let still = with_runtime(|rt| matches!(&rt.nodes[car.0 as usize].kind,
            Kind::Carousel { anim_dir, .. } if *anim_dir != 0));
        assert!(!still, "animation clears after ANIM_STEPS+1 steps");
        cx.dispose();
    }
}
