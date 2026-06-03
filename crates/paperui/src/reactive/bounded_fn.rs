//! Inline-stored `FnMut` up to `W` machine words. The no-alloc analog of `Box<dyn FnMut>`.
//!
//! # Safety
//! The closure is stored in a usize-aligned byte buffer. `new` const-asserts the closure
//! fits (`size_of` ≤ `W*word`, `align_of` ≤ word) — an oversized/over-aligned closure is a
//! *compile* error, not a runtime panic. `call`/`drop` reconstruct `*mut F` from the buffer;
//! this is sound because of two invariants: (1) the buffer is only ever read after `new::<F>`
//! wrote it, and the closure is never extracted or moved out (there is no such API), so the
//! `MaybeUninit` is always initialized at `call`/`drop`; (2) the stored `call`/`drop`
//! fn-pointers are monomorphized for that exact `F`, set in the same `new::<F>` call.
//!
//! # Thread-safety
//! `BoundedFn` auto-derives `Send`/`Sync` (its fields are an array of `MaybeUninit<usize>`
//! plus two fn-pointers; the closure `F` is type-erased, not a field). This is *intentional
//! and required*: the reactive `Runtime` stores `BoundedFn`s and lives in a
//! `static critical_section::Mutex<RefCell<Runtime>>`, which only satisfies `Sync` if its
//! contents are `Send`. Synchronization is provided by `critical_section::with` (interrupt
//! masking / cross-core lock), NOT by the type system. Constraint for callers: closures
//! stored here (effects, handlers) must not capture state that is unsound to touch across
//! the critical-section boundary — in Layer #1 captures are `Copy` signal handles and
//! esp-hal peripheral handles, which are `Send`.
#![allow(unsafe_code)]

use core::mem::{align_of, size_of, MaybeUninit};

pub struct BoundedFn<const W: usize, R = ()> {
    storage: [MaybeUninit<usize>; W],
    call: unsafe fn(*mut u8) -> R,
    drop_fn: unsafe fn(*mut u8),
}

impl<const W: usize, R> BoundedFn<W, R> {
    pub fn new<F: FnMut() -> R>(f: F) -> Self {
        // Compile-time (monomorphization-time) guards: an oversized/over-aligned closure
        // fails the build rather than panicking at runtime — important on embedded.
        const { assert!(size_of::<F>() <= W * size_of::<usize>(), "closure too large for BoundedFn<W>") };
        const { assert!(align_of::<F>() <= align_of::<usize>(), "closure over-aligned for BoundedFn<W>") };

        let mut storage = [MaybeUninit::<usize>::uninit(); W];
        // SAFETY: buffer is usize-aligned and large enough (asserted above).
        unsafe {
            let ptr = storage.as_mut_ptr() as *mut F;
            ptr.write(f);
        }
        unsafe fn call_impl<F: FnMut() -> R, R>(p: *mut u8) -> R {
            // SAFETY: p points at a live F written by `new`.
            (*(p as *mut F))()
        }
        unsafe fn drop_impl<F>(p: *mut u8) {
            // SAFETY: p points at a live F written by `new`.
            core::ptr::drop_in_place(p as *mut F);
        }
        Self { storage, call: call_impl::<F, R>, drop_fn: drop_impl::<F> }
    }

    pub fn call(&mut self) -> R {
        // SAFETY: storage holds a live F; `call` is its monomorphized caller.
        unsafe { (self.call)(self.storage.as_mut_ptr() as *mut u8) }
    }
}

impl<const W: usize, R> Drop for BoundedFn<W, R> {
    fn drop(&mut self) {
        // SAFETY: storage holds a live F; `drop_fn` is its monomorphized dropper.
        unsafe { (self.drop_fn)(self.storage.as_mut_ptr() as *mut u8) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    #[test]
    fn stores_and_calls_a_capturing_closure() {
        let captured = 41i32;
        let mut f = BoundedFn::<4, i32>::new(move || captured + 1);
        assert_eq!(f.call(), 42);
        assert_eq!(f.call(), 42); // FnMut, repeatable
    }

    #[test]
    fn mutates_captured_state_across_calls() {
        let mut n = 0u32;
        let mut f = BoundedFn::<4, u32>::new(move || { n += 1; n });
        assert_eq!(f.call(), 1);
        assert_eq!(f.call(), 2);
    }

    #[test]
    fn runs_drop_on_owned_capture() {
        // A counter (not a bool) proves drop runs EXACTLY once, catching a double-drop regression.
        struct Bomb<'a>(&'a Cell<u32>);
        impl Drop for Bomb<'_> {
            fn drop(&mut self) { self.0.set(self.0.get() + 1); }
        }
        let drops = Cell::new(0u32);
        {
            let bomb = Bomb(&drops);
            // `move` makes the closure OWN `bomb`; referencing its field forces the capture.
            let _f = BoundedFn::<4, ()>::new(move || { let _ = bomb.0; });
            assert_eq!(drops.get(), 0, "captured Bomb must not drop while the BoundedFn is alive");
        } // _f drops here -> drop_fn -> drops the closure -> drops `bomb` -> increments counter
        assert_eq!(drops.get(), 1, "dropping the BoundedFn must drop the captured Bomb exactly once");
    }
}
