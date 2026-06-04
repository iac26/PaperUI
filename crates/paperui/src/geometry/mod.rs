//! Geometric value types: points, sizes, and rectangles. Layout `Constraints` live in the
//! `constraints` submodule, re-exported here.

mod constraints;
pub use constraints::Constraints;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point { pub x: i16, pub y: i16 }
impl Point { pub const fn new(x: i16, y: i16) -> Self { Self { x, y } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size { pub w: i16, pub h: i16 }
impl Size { pub const fn new(w: i16, h: i16) -> Self { Self { w, h } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub x: i16, pub y: i16, pub w: i16, pub h: i16 }

impl Rect {
    pub const fn new(x: i16, y: i16, w: i16, h: i16) -> Self { Self { x, y, w, h } }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        !(self.x + self.w <= o.x || o.x + o.w <= self.x
            || self.y + self.h <= o.y || o.y + o.h <= self.y)
    }

    pub fn is_empty(&self) -> bool { self.w == 0 && self.h == 0 }

    pub fn unite(&self, o: &Rect) -> Rect {
        if self.is_empty() { return *o; }
        if o.is_empty() { return *self; }
        let nx = self.x.min(o.x);
        let ny = self.y.min(o.y);
        let nr = (self.x + self.w).max(o.x + o.w);
        let nb = (self.y + self.h).max(o.y + o.h);
        Rect::new(nx, ny, nr - nx, nb - ny)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_point_inclusive_origin_exclusive_far_edge() {
        let r = Rect::new(10, 10, 20, 20); // covers x in [10,30), y in [10,30)
        assert!(r.contains(Point::new(10, 10)));
        assert!(r.contains(Point::new(29, 29)));
        assert!(!r.contains(Point::new(30, 30)));
        assert!(!r.contains(Point::new(9, 10)));
    }

    #[test]
    fn rect_intersects_detects_overlap_and_gap() {
        let a = Rect::new(0, 0, 10, 10);
        assert!(a.intersects(&Rect::new(5, 5, 10, 10)));
        assert!(!a.intersects(&Rect::new(10, 0, 5, 5))); // touching edge = no overlap
    }

    #[test]
    fn rect_unite_covers_both() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(20, 20, 5, 5);
        let u = a.unite(&b);
        assert_eq!(u, Rect::new(0, 0, 25, 25));
    }
}
