//! Synchronous driver: poll an EventSource, dispatch to focus/activate, render dirties.
//! Generic over EventSource so an async (embassy) driver can drop in at a later layer
//! without touching the engine. Layer #1 is intentionally blocking.

use crate::canvas::Canvas;
use crate::reactive::node::{focus_next, invoke_handler_of_focus};
use crate::reactive::render::{render_frame, render_frame_full};
use crate::reactive::runtime::{with_runtime, NodeId};
use crate::widget_theme::WidgetTheme;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum UiEvent {
    FocusNext,
    Activate,
}

/// A source of UI events (buttons, touch, …). `now_ms` is a monotonic-ish millisecond clock
/// the source may use for gesture timing.
pub trait EventSource {
    fn poll(&mut self, now_ms: u32) -> Option<UiEvent>;
}

fn dispatch(ev: UiEvent) {
    match ev {
        UiEvent::FocusNext => focus_next(),
        UiEvent::Activate => invoke_handler_of_focus(),
    }
}

/// The synchronous app loop. Never returns. Renders once, then polls the source, dispatches
/// events, and re-renders only when something is dirty.
///
/// NOTE (Layer #1): `now_ms` here is a placeholder clock advanced per iteration; a real driver
/// should feed a true millisecond timer (the EventSource may also keep its own). Gesture
/// timing (hold/double) depends on a real clock — wire one in at integration (Task 12).
pub fn run<C, T, S>(root: NodeId, canvas: &mut C, theme: &T, src: &mut S, mut now_ms: u32) -> !
where
    C: Canvas,
    T: WidgetTheme<C>,
    S: EventSource,
{
    render_frame_full(root, canvas, theme);
    loop {
        while let Some(ev) = src.poll(now_ms) {
            dispatch(ev);
        }
        let has_dirty = with_runtime(|rt| !rt.dirty.is_empty());
        if has_dirty {
            render_frame(root, canvas, theme);
        }
        now_ms = now_ms.wrapping_add(5);
    }
}

#[cfg(test)]
pub(crate) fn pump_until_empty(src: &mut impl EventSource) {
    while let Some(ev) = src.poll(0) {
        dispatch(ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::node::{button, col, text_static};
    use crate::reactive::scope::Scope;

    struct Scripted(heapless::Deque<UiEvent, 8>);
    impl EventSource for Scripted {
        fn poll(&mut self, _now: u32) -> Option<UiEvent> {
            self.0.pop_front()
        }
    }

    #[test]
    fn focus_next_then_activate_runs_the_focused_handler() {
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let tx = text_static(cx, "x");
        let b = button(cx, "+", move || count.update(|c| *c += 1));
        let _root = col(cx, (tx, b));
        // The node arena is global + append-only across tests, so a bare FocusNext could land
        // on another test's leftover button. Pin focus to OUR text node; since `b` is the
        // immediately-following node, the next FocusNext deterministically steps onto `b`.
        with_runtime(|rt| rt.focus = Some(tx));
        let mut ev = heapless::Deque::<UiEvent, 8>::new();
        let _ = ev.push_back(UiEvent::FocusNext);
        let _ = ev.push_back(UiEvent::Activate);
        let mut src = Scripted(ev);
        pump_until_empty(&mut src);
        assert_eq!(count.get(), 1, "FocusNext onto the button, then Activate, runs its handler");
        cx.dispose();
    }
}
