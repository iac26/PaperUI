//! Copy, size-bounded reactive cells. `get` records a dependency if an effect is running;
//! `set` bumps the epoch and dirties subscribers only when the value actually changes.

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

#[allow(dead_code)]
impl<T: ReactiveValue> Signal<T> {
    pub(crate) fn alloc_in(rt: &mut Runtime, owner: OwnerId, init: T) -> Self {
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
        with_runtime(|rt| {
            let old = value_cell::read_value::<T>(rt, self.id);
            if old != v {
                value_cell::write_value(rt, self.id, v);
                rt.epoch = rt.epoch.wrapping_add(1);
                // dirty all subscribers
                for i in 0..rt.signals[self.id.0 as usize].subs.len() {
                    let n = rt.signals[self.id.0 as usize].subs[i];
                    if !rt.dirty.contains(&n) {
                        let _ = rt.dirty.push(n);
                    }
                }
            }
        });
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

    pub(super) fn write_value<T: Copy + 'static>(rt: &mut Runtime, id: SignalId, v: T) {
        let slot = &mut rt.signals[id.0 as usize].value;
        // SAFETY: slot is usize-aligned and large enough for T (asserted in alloc_in).
        unsafe {
            let dst = slot.as_mut_ptr() as *mut T;
            dst.write(v);
        }
    }

    pub(super) fn read_value<T: Copy + 'static>(rt: &Runtime, id: SignalId) -> T {
        let slot = &rt.signals[id.0 as usize].value;
        // SAFETY: a T was written here by write_value::<T>; T: Copy makes the read sound.
        unsafe {
            let src = slot.as_ptr() as *const T;
            src.read()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alloc<T: ReactiveValue>(init: T) -> Signal<T> {
        with_runtime(|rt| Signal::<T>::alloc_in(rt, OwnerId(0), init))
    }

    #[test]
    fn get_returns_initial_value() {
        let s = alloc(7i32);
        assert_eq!(s.get(), 7);
    }

    #[test]
    fn set_changes_value_and_bumps_epoch_only_on_change() {
        let s = alloc(1i32);
        let e0 = with_runtime(|rt| rt.epoch);
        s.set(1); // no-op
        assert_eq!(with_runtime(|rt| rt.epoch), e0, "no-op set must not bump epoch");
        s.set(2);
        assert!(with_runtime(|rt| rt.epoch) > e0);
        assert_eq!(s.get(), 2);
    }
}
