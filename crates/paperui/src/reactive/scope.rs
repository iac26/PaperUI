//! A disposal scope. Signals created under a scope are freed when it is disposed.
//! Layer #1 uses the root scope; `child` is the seam Navigation (Layer #2) needs.

use crate::reactive::anim::{Anim, AnimKind, Animatable, Animated};
use crate::reactive::bounded_fn::BoundedFn;
use crate::reactive::runtime::{with_runtime, EffectId, OwnerId};
use crate::reactive::signal::{ReactiveValue, Signal};
use crate::reactive::CLOSURE_WORDS;

#[derive(Copy, Clone)]
pub struct Scope {
    pub(crate) owner: OwnerId,
}

impl Scope {
    /// The root scope. Layer #1 apps build under this.
    pub fn root() -> Self {
        with_runtime(|rt| Scope { owner: rt.alloc_owner() })
    }
    /// A child scope. Layer #1 treats it as a fresh owner; Layer #2 (navigation) will
    /// parent it for nested disposal.
    pub fn child(self) -> Self {
        with_runtime(|rt| Scope { owner: rt.alloc_owner() })
    }
    pub fn signal<T: ReactiveValue>(self, init: T) -> Signal<T> {
        with_runtime(|rt| Signal::alloc_in(rt, self.owner, init))
    }
    /// A clock-driven reactive value. Allocates a signal (seeded with `init`) and a Transition
    /// animator (idle: `from == to == init`), both owned by this scope, so `dispose` frees them.
    pub fn tween<T: Animatable>(self, init: T, anim: Anim) -> Animated<T> {
        with_runtime(|rt| {
            let sig = Signal::alloc_in(rt, self.owner, init);
            let id = rt.alloc_anim(self.owner, sig.id, AnimKind::Transition, anim.dur_ms);
            let a = &mut rt.animators[id.0 as usize];
            a.from = init.to_av();
            a.to = init.to_av();
            Animated::new(sig, id)
        })
    }
    pub fn dispose(self) {
        with_runtime(|rt| rt.free_owner(self.owner));
    }
    /// Store a handler/effect closure; returns its id for a node to reference.
    pub fn handler(self, f: impl FnMut() + 'static) -> EffectId {
        let bf = BoundedFn::<CLOSURE_WORDS, ()>::new(f);
        with_runtime(|rt| rt.alloc_effect(self.owner, bf))
    }
}

/// React-style alias for `cx.signal(init())`.
pub fn use_state<T: ReactiveValue>(cx: Scope, init: impl FnOnce() -> T) -> Signal<T> {
    cx.signal(init())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::runtime::fresh_runtime;

    #[test]
    fn signals_created_under_a_scope_are_freed_on_dispose() {
        fresh_runtime();
        let used_before = with_runtime(|rt| rt.signals.iter().filter(|s| s.in_use).count());
        let cx = Scope::root();
        let s = cx.signal(5i32);
        assert_eq!(s.get(), 5);
        let used_during = with_runtime(|rt| rt.signals.iter().filter(|s| s.in_use).count());
        assert_eq!(used_during, used_before + 1);
        cx.dispose();
        let used_after = with_runtime(|rt| rt.signals.iter().filter(|s| s.in_use).count());
        assert_eq!(used_after, used_before, "dispose must free the scope's signals");
    }

    #[test]
    fn use_state_is_sugar_over_signal() {
        fresh_runtime();
        let cx = Scope::root();
        let count = use_state(cx, || 0u32);
        count.set(3);
        assert_eq!(count.get(), 3);
        cx.dispose();
    }
}
