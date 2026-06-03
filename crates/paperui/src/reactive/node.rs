//! The retained view tree. Builders allocate nodes into the runtime arena and return ids.

use crate::geometry::Rect;
use crate::reactive::runtime::{with_runtime, EffectId, NodeId, OwnerId, Runtime};
use crate::reactive::scope::Scope;
use crate::reactive::{MAX_CHILDREN, TEXT_CAP};
use heapless::Vec;

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

#[allow(dead_code)] // gains a non-test caller in Task 10 (focus/activate).
pub(crate) fn handler_of_button(rt: &Runtime, node: NodeId) -> Option<EffectId> {
    match &rt.nodes[node.0 as usize].kind {
        Kind::Button { on_press, .. } => Some(*on_press),
        _ => None,
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
}
