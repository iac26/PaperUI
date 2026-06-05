//! Synchronous driver: poll an EventSource, dispatch to focus/activate, render dirties.
//! Generic over EventSource so an async (embassy) driver can drop in at a later layer
//! without touching the engine. Layer #1 is intentionally blocking.

use crate::geometry::Point;
use crate::paint::{Canvas, WidgetTheme};
use crate::reactive::anim::advance_anims;
use crate::reactive::node::{invoke_handler_at, invoke_handler_of_focus, nav};
use crate::reactive::render::{render_frame_full, render_tick};
use crate::reactive::runtime::{with_runtime, NodeId};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum UiEvent {
    FocusNext,
    FocusPrev,
    Activate,
    Pointer(Point),
}

/// A source of UI events (buttons, touch, …). `now_ms` is a monotonic-ish millisecond clock
/// the source may use for gesture timing; the driver reads it each iteration to pace animations.
pub trait EventSource {
    fn poll(&mut self, now_ms: u32) -> Option<UiEvent>;
    /// Real monotonic millisecond clock (the app reads its hardware timer). Drives animation
    /// progress; elapsed is computed with `wrapping_sub`, so the ~49-day `u32` wrap is harmless.
    fn now_ms(&mut self) -> u32;
}

/// Route a UI event into the reactive core. Public so an external driver (e.g. the desktop
/// `paperui-sim` backend) can dispatch events it pumped from a window.
pub fn dispatch(ev: UiEvent) {
    match ev {
        UiEvent::FocusNext => nav(1),
        UiEvent::FocusPrev => nav(-1),
        UiEvent::Activate => invoke_handler_of_focus(),
        UiEvent::Pointer(p) => invoke_handler_at(p),
    }
}

/// The synchronous app loop. Never returns. Renders once, then per iteration reads the source's
/// real clock, polls + dispatches events, advances active animations (which dirty their
/// dependents), and re-renders only when something is dirty.
pub fn run<C, T, S>(root: NodeId, canvas: &mut C, theme: &T, src: &mut S) -> !
where
    C: Canvas,
    T: WidgetTheme<C>,
    S: EventSource,
{
    render_frame_full(root, canvas, theme);
    loop {
        let now = src.now_ms();
        while let Some(ev) = src.poll(now) {
            dispatch(ev);
        }
        with_runtime(|rt| advance_anims(rt, now));
        render_tick(root, canvas, theme);
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
    use crate::reactive::node::{button, carousel, carousel_select_first, col, text, text_static};
    use crate::reactive::runtime::{fresh_runtime, with_runtime};
    use crate::reactive::scope::Scope;
    use crate::geometry::{Point, Rect};

    struct Scripted(heapless::Deque<UiEvent, 8>);
    impl EventSource for Scripted {
        fn poll(&mut self, _now: u32) -> Option<UiEvent> {
            self.0.pop_front()
        }
        fn now_ms(&mut self) -> u32 {
            0
        }
    }

    #[test]
    fn focus_next_then_activate_runs_the_focused_handler() {
        fresh_runtime();
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

    #[test]
    fn dispatch_pointer_activates_the_button_under_the_point() {
        fresh_runtime();
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let b = button(cx, "+", move || count.update(|c| *c += 1));
        with_runtime(|rt| rt.nodes[b.0 as usize].bounds = Rect::new(3000, 3000, 40, 20));
        dispatch(UiEvent::Pointer(Point::new(3010, 3010)));
        assert_eq!(count.get(), 1, "Pointer routes to invoke_handler_at");
        cx.dispose();
    }

    #[test]
    fn signal_change_dirties_its_subscriber() {
        fresh_runtime();
        let cx = Scope::root();
        let count = cx.signal(0i32);
        let _t = text(cx, move || {
            let mut s = heapless::String::<32>::new();
            let _ = core::fmt::write(&mut s, format_args!("{}", count.get()));
            s
        });
        with_runtime(|rt| rt.dirty.clear());
        assert!(with_runtime(|rt| rt.dirty.is_empty()), "no work pending right after a clear");
        count.set(1);
        assert!(with_runtime(|rt| !rt.dirty.is_empty()), "a signal change dirties its subscriber");
        cx.dispose();
    }

    fn six_carousel() -> (Scope, NodeId, [NodeId; 6]) {
        let cx = Scope::root();
        let items = [
            button(cx, "0", || {}), button(cx, "1", || {}), button(cx, "2", || {}),
            button(cx, "3", || {}), button(cx, "4", || {}), button(cx, "5", || {}),
        ];
        let car = carousel(cx, &items);
        carousel_select_first(car);
        (cx, car, items)
    }

    fn carousel_state(car: NodeId) -> (usize, usize) {
        with_runtime(|rt| match &rt.nodes[car.0 as usize].kind {
            crate::reactive::Kind::Carousel { selected, offset, .. } => (*selected, *offset),
            _ => unreachable!(),
        })
    }

    #[test]
    fn down_centers_selection_and_wraps() {
        fresh_runtime();
        let (cx, car, items) = six_carousel();
        let mut s = Scripted(heapless::Deque::new());
        for _ in 0..3 { let _ = s.0.push_back(UiEvent::FocusNext); }
        pump_until_empty(&mut s);
        // Selection stays centered; offset is the item above it (selected-1, wrapping).
        assert_eq!(carousel_state(car), (3, 2), "selected=3 centered, top slot=2");
        assert_eq!(with_runtime(|rt| rt.focus), Some(items[3]));
        // Six more downs wrap all the way around back to 3.
        for _ in 0..6 { let _ = s.0.push_back(UiEvent::FocusNext); }
        pump_until_empty(&mut s);
        assert_eq!(carousel_state(car).0, 3, "down wraps around the list");
        cx.dispose();
    }

    #[test]
    fn up_navigates_backward_and_wraps() {
        fresh_runtime();
        let (cx, car, items) = six_carousel();
        let mut s = Scripted(heapless::Deque::new());
        let _ = s.0.push_back(UiEvent::FocusPrev); // up from first wraps to last
        pump_until_empty(&mut s);
        assert_eq!(carousel_state(car), (5, 4), "up from first wraps to last (top slot=4)");
        assert_eq!(with_runtime(|rt| rt.focus), Some(items[5]));
        cx.dispose();
    }
}
