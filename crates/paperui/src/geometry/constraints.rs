//! Layout size constraints (min/max), in pixels.
use super::Size;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constraints { pub min_w: i16, pub min_h: i16, pub max_w: i16, pub max_h: i16 }
impl Constraints {
    pub const fn new(min_w: i16, min_h: i16, max_w: i16, max_h: i16) -> Self {
        Self { min_w, min_h, max_w, max_h }
    }
    pub fn clamp(&self, s: Size) -> Size {
        Size::new(s.w.clamp(self.min_w, self.max_w), s.h.clamp(self.min_h, self.max_h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraints_clamp_works() {
        let c = Constraints::new(10, 10, 100, 50);
        assert_eq!(c.clamp(Size::new(5, 200)), Size::new(10, 50));
        assert_eq!(c.clamp(Size::new(40, 30)), Size::new(40, 30));
    }
}
