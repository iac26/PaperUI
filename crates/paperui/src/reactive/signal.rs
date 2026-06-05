//! Copy, size-bounded reactive cells. `get` records a dependency if an effect is running;
//! `set` bumps the epoch and dirties subscribers only when the value actually changes.

use crate::paint::Color;
use crate::reactive::anim::AnimValue;
use crate::reactive::runtime::{with_runtime, OwnerId, Runtime, SignalId, TypeIdShim};
use core::any::TypeId;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};

/// Layer #1 signal payloads: `Copy + PartialEq + 'static`, small enough for a slot.
pub trait ReactiveValue: Copy + PartialEq + 'static {}
impl<T: Copy + PartialEq + 'static> ReactiveValue for T {}

#[derive(Copy, Clone)]
pub struct Signal<T> {
    pub(crate) id: SignalId,
    pub(crate) _pd: PhantomData<T>,
}

impl<T: ReactiveValue> Signal<T> {
    pub(crate) fn alloc_in(rt: &mut Runtime<'_>, owner: OwnerId, init: T) -> Self {
        // Compile-time guards: payload must fit the slot in BOTH size and alignment.
        // (Mirrors BoundedFn. `value` is `[MaybeUninit<usize>; SIGNAL_SLOT_WORDS]`.)
        const {
            assert!(
                size_of::<T>() <= crate::reactive::SIGNAL_SLOT_WORDS * size_of::<usize>(),
                "signal payload too large (raise SIGNAL_SLOT_WORDS)"
            );
        }
        const {
            assert!(
                align_of::<T>() <= align_of::<usize>(),
                "signal payload over-aligned for the slot"
            );
        }
        let id = rt.alloc_signal(owner, TypeIdShim(TypeId::of::<T>()));
        let s = Self { id, _pd: PhantomData };
        value_cell::write_value(rt, id, init);
        s
    }

    pub fn get(self) -> T {
        with_runtime(|rt| {
            if let Some(node) = rt.current_effect {
                let subs = &mut rt.signals[self.id.0 as usize].subs;
                if !subs.contains(&node) {
                    let _ = subs.push(node);
                }
            }
            value_cell::read_value::<T>(rt, self.id)
        })
    }

    pub fn set(self, v: T) {
        // Same write protocol as the animator tick path; share the one implementation.
        with_runtime(|rt| set_typed::<T>(rt, self.id, v));
    }

    pub fn update(self, f: impl FnOnce(&mut T)) {
        let mut v = self.get();
        f(&mut v);
        self.set(v);
    }

    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let v = self.get();
        f(&v)
    }

    /// Read the current value WITHOUT locking or subscribing. For callers that already hold the
    /// runtime (e.g. the animator tick inside `with_runtime`); using `get` there would re-enter
    /// the non-reentrant critical section and deadlock.
    pub(crate) fn get_in(self, rt: &Runtime) -> T {
        value_cell::read_value::<T>(rt, self.id)
    }
}

/// Non-generic, lock-free signal write used by the animator tick (which carries an `AnimValue`,
/// not a `T`). Dispatches on the variant to recover the concrete type, then mirrors `Signal::set`'s
/// change-detect + epoch-bump + subscriber-dirty — but on the already-held `rt`, NOT via
/// `with_runtime` (the caller already holds it; nesting would deadlock).
pub(crate) fn set_signal_av(rt: &mut Runtime, id: SignalId, av: AnimValue) {
    match av {
        AnimValue::I16(v) => set_typed::<i16>(rt, id, v),
        AnimValue::Color(c) => set_typed::<Color>(rt, id, c),
        AnimValue::Frame(f) => set_typed::<u8>(rt, id, f),
    }
}

fn set_typed<T: ReactiveValue>(rt: &mut Runtime, id: SignalId, v: T) {
    let old = value_cell::read_value::<T>(rt, id);
    if old != v {
        value_cell::write_value(rt, id, v);
        rt.epoch = rt.epoch.wrapping_add(1);
        for i in 0..rt.signals[id.0 as usize].subs.len() {
            let n = rt.signals[id.0 as usize].subs[i];
            if !rt.dirty.contains(&n) {
                let _ = rt.dirty.push(n);
            }
        }
    }
}

mod value_cell {
    //! The ONLY unsafe in this file: raw byte read/write of the signal slot.
    //!
    //! # Safety
    //! The slot is `[MaybeUninit<usize>; SIGNAL_SLOT_WORDS]`, hence `usize`-aligned.
    //! `Signal::alloc_in` const-asserts `size_of::<T>() <= slot bytes` AND
    //! `align_of::<T>() <= align_of::<usize>()` before any write, and the slot's `type_id`
    //! tracks the stored type. Therefore `read_value::<T>` only ever reads a `T` previously
    //! written by `write_value::<T>` into a correctly sized & aligned slot, and `T: Copy`
    //! makes the bitwise read sound.
    #![allow(unsafe_code)]
    use crate::reactive::runtime::{Runtime, SignalId};

    pub(super) fn write_value<T: Copy + 'static>(rt: &mut Runtime<'_>, id: SignalId, v: T) {
        // Enforce the type-match invariant the # Safety rationale relies on: the slot must
        // already be tagged with T's TypeId (set by `alloc_signal`). Catches a stale handle
        // or a wrong-T access in debug builds before it can reinterpret bytes.
        debug_assert_eq!(
            rt.signals[id.0 as usize].type_id,
            Some(core::any::TypeId::of::<T>()),
            "signal slot type mismatch on write (stale handle or wrong T)"
        );
        let slot = &mut rt.signals[id.0 as usize].value;
        // SAFETY: slot is usize-aligned and large enough for T (asserted in alloc_in), and the
        // slot is tagged with T's type (asserted above), so this writes a correctly-typed cell.
        unsafe {
            let dst = slot.as_mut_ptr() as *mut T;
            dst.write(v);
        }
    }

    pub(super) fn read_value<T: Copy + 'static>(rt: &Runtime<'_>, id: SignalId) -> T {
        // Same type-match invariant as write_value: only read a slot tagged with T's TypeId.
        debug_assert_eq!(
            rt.signals[id.0 as usize].type_id,
            Some(core::any::TypeId::of::<T>()),
            "signal slot type mismatch on read (stale handle or wrong T)"
        );
        let slot = &rt.signals[id.0 as usize].value;
        // SAFETY: a T was written here by write_value::<T> into a same-typed slot (asserted
        // above); T: Copy makes the bitwise read sound.
        unsafe {
            let src = slot.as_ptr() as *const T;
            src.read()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::runtime::fresh_runtime;

    fn alloc<T: ReactiveValue>(init: T) -> Signal<T> {
        with_runtime(|rt| Signal::<T>::alloc_in(rt, OwnerId(0), init))
    }

    #[test]
    fn get_returns_initial_value() {
        fresh_runtime();
        let s = alloc(7i32);
        assert_eq!(s.get(), 7);
    }

    #[test]
    fn set_changes_value_and_bumps_epoch_only_on_change() {
        fresh_runtime();
        let s = alloc(1i32);
        let e0 = with_runtime(|rt| rt.epoch);
        s.set(1); // no-op
        assert_eq!(with_runtime(|rt| rt.epoch), e0, "no-op set must not bump epoch");
        s.set(2);
        assert!(with_runtime(|rt| rt.epoch) > e0);
        assert_eq!(s.get(), 2);
    }
}
