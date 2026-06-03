//! Inline-stored `FnMut` up to `W` machine words. The no-alloc analog of `Box<dyn FnMut>`.
//!
//! # Safety
//! The closure is stored in a usize-aligned byte buffer. `new` const-asserts the closure
//! fits (`size_of` ≤ `W*word`, `align_of` ≤ word). `call`/`drop` reconstruct `*mut F` from
//! the buffer; this is sound because only a value written by `new::<F>` is ever read, and
//! the stored `call`/`drop` fn-pointers are monomorphized for that exact `F`.
#![allow(unsafe_code)]

use core::mem::{align_of, size_of, MaybeUninit};

pub struct BoundedFn<const W: usize, R = ()> {
    storage: [MaybeUninit<usize>; W],
    call: unsafe fn(*mut u8) -> R,
    drop_fn: unsafe fn(*mut u8),
}

impl<const W: usize, R> BoundedFn<W, R> {
    pub fn new<F: FnMut() -> R>(f: F) -> Self {
        assert!(size_of::<F>() <= W * size_of::<usize>(), "closure too large for BoundedFn<W>");
        assert!(align_of::<F>() <= align_of::<usize>(), "closure over-aligned for BoundedFn<W>");

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
        struct Bomb<'a>(&'a Cell<bool>);
        impl Drop for Bomb<'_> {
            fn drop(&mut self) { self.0.set(true); }
        }
        let flag = Cell::new(false);
        {
            let bomb = Bomb(&flag);
            // `move` makes the closure OWN `bomb`; referencing its field forces the capture.
            let _f = BoundedFn::<4, ()>::new(move || { let _ = bomb.0; });
            assert!(!flag.get(), "captured Bomb must not drop while the BoundedFn is alive");
        } // _f drops here -> drop_fn -> drops the closure -> drops `bomb` -> sets flag
        assert!(flag.get(), "dropping the BoundedFn must drop the captured Bomb exactly once");
    }
}
