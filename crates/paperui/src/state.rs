/// Reactive value with a monotonic generation counter. Widgets compare the
/// generation against a stored value to detect changes (no heap, no observers).
pub struct State<T> { value: T, generation: u32 }

impl<T: PartialEq> State<T> {
    pub const fn new(value: T) -> Self { Self { value, generation: 1 } }
    pub fn get(&self) -> &T { &self.value }
    pub fn generation(&self) -> u32 { self.generation }
    /// Sets the value; bumps the generation only if the value actually changed.
    pub fn set(&mut self, value: T) {
        if self.value != value {
            self.value = value;
            self.generation = self.generation.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bumps_generation_only_on_change() {
        let mut s = State::new(3u32);
        let g0 = s.generation();
        s.set(3); // same value
        assert_eq!(s.generation(), g0, "no-op set must not bump generation");
        s.set(4);
        assert!(s.generation() > g0);
        assert_eq!(*s.get(), 4);
    }
}
