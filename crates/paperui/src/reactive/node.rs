//! The retained view tree. Builders allocate nodes into the runtime arena and return ids.

use crate::geometry::{Point, Rect};
use crate::reactive::runtime::{with_runtime, EffectId, NodeId, OwnerId, Runtime};
use crate::reactive::scope::Scope;
use crate::reactive::{MAX_CHILDREN, TEXT_CAP};
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

/// Advance focus to the next `Button` node (wrapping). With no current focus, starts at the
/// first node. Non-Button nodes are skipped.
pub(crate) fn focus_next() {
    with_runtime(|rt| {
        let n = rt.nodes.len();
        if n == 0 {
            return;
        }
        let old = rt.focus;
        let cur = rt.focus.map(|f| f.0 as usize);
        for off in 1..=n {
            let i = match cur {
                Some(c) => (c + off) % n,
                None => off - 1, // scan from index 0 when nothing is focused
            };
            if matches!(rt.nodes[i].kind, Kind::Button { .. }) {
                let new = NodeId(i as u16);
                rt.focus = Some(new);
                mark_dirty(rt, old);
                mark_dirty(rt, Some(new));
                return;
            }
        }
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
        // b1 is the node immediately after b0, so FocusNext from b0 lands on b1.
        with_runtime(|rt| {
            rt.focus = Some(b0);
            rt.dirty.clear();
        });
        focus_next();
        with_runtime(|rt| {
            assert_eq!(rt.focus, Some(b1), "focus advanced to the next button");
            assert!(rt.dirty.contains(&b0), "old focus repaints");
            assert!(rt.dirty.contains(&b1), "new focus repaints");
        });
        cx.dispose();
    }
}
