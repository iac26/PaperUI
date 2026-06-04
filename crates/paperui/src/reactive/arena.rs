//! A bounded, growable buffer over a borrowed slice: the engine's non-generic stand-in for
//! `heapless::Vec<T, N>`. It carries no capacity in its type, so the runtime view can hold it
//! without leaking the app's compile-time pool sizes into engine signatures. Safe: the backing
//! slice is fully initialized by the app's `Storage`; `len` tracks the live prefix `[0..len]`.

use core::ops::{Index, IndexMut};

pub(crate) struct Arena<'a, T> {
    buf: &'a mut [T],
    len: usize,
}

impl<'a, T> Arena<'a, T> {
    pub(crate) fn new(buf: &'a mut [T]) -> Self {
        Self { buf, len: 0 }
    }
    pub(crate) fn len(&self) -> usize {
        self.len
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub(crate) fn clear(&mut self) {
        self.len = 0;
    }
    /// Append `v`, returning `true` on success or `false` if the backing slice is full.
    pub(crate) fn push(&mut self, v: T) -> bool {
        if self.len < self.buf.len() {
            self.buf[self.len] = v;
            self.len += 1;
            true
        } else {
            false
        }
    }
    pub(crate) fn as_slice(&self) -> &[T] {
        &self.buf[..self.len]
    }
    pub(crate) fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.buf[..self.len]
    }
    pub(crate) fn iter(&self) -> core::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
    pub(crate) fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<'a, T: PartialEq> Arena<'a, T> {
    pub(crate) fn contains(&self, v: &T) -> bool {
        self.as_slice().contains(v)
    }
}

impl<'a, T> Index<usize> for Arena<'a, T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        &self.as_slice()[i]
    }
}
impl<'a, T> IndexMut<usize> for Arena<'a, T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.as_mut_slice()[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_increments_len_and_indexes_back() {
        let mut buf = [0u16; 4];
        let mut a = Arena::new(&mut buf);
        assert!(a.is_empty());
        assert!(a.push(10));
        assert!(a.push(20));
        assert_eq!(a.len(), 2);
        assert_eq!(a[0], 10);
        assert_eq!(a[1], 20);
        assert_eq!(a.as_slice(), &[10, 20]);
        assert!(a.push(30));
        assert!(a.push(40));
        assert_eq!(a.len(), 4);
        assert_eq!(a.as_slice(), &[10, 20, 30, 40]);
    }

    #[test]
    fn push_past_capacity_returns_false_and_does_not_grow() {
        let mut buf = [0u16; 2];
        let mut a = Arena::new(&mut buf);
        assert!(a.push(1));
        assert!(a.push(2));
        assert!(!a.push(3), "push past the backing slice fails gracefully");
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn clear_resets_len_and_contains_tracks_live_prefix() {
        let mut buf = [0u16; 4];
        let mut a = Arena::new(&mut buf);
        a.push(7);
        assert!(a.contains(&7));
        a.clear();
        assert!(a.is_empty());
        assert!(!a.contains(&7), "cleared elements are no longer live");
    }

    #[test]
    fn index_mut_writes_through() {
        let mut buf = [0u16; 4];
        let mut a = Arena::new(&mut buf);
        a.push(1);
        a[0] = 99;
        assert_eq!(a[0], 99);
    }
}
