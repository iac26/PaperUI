//! Two-pass layout: measure desired sizes, then place absolute bounds. Pure integer math.
//! A leaf (Text/Button) is chars*FONT0_W + 2*PAD wide, FONT0_H + 2*PAD tall. A Column stacks
//! children (max width, summed heights + spacing); a Row is the transpose.

use crate::canvas::{FONT0_H, FONT0_W};
use crate::geometry::{Rect, Size};
use crate::reactive::node::Kind;
use crate::reactive::runtime::{with_runtime, NodeId, Runtime};
use crate::reactive::VISIBLE;

const PAD: i16 = 6;
pub(crate) const CAROUSEL_ROW_H: i16 = FONT0_H + 2 * PAD;
pub(crate) const CAROUSEL_SPACING: i16 = 4;
pub(crate) const CAROUSEL_ROW_PITCH: i16 = CAROUSEL_ROW_H + CAROUSEL_SPACING;
pub(crate) const CAROUSEL_OFFSCREEN_Y: i16 = 1000;

/// Desired size of a node (recurses into children for Column/Row). Read-only.
fn measure_inner(rt: &Runtime, node: NodeId) -> Size {
    match &rt.nodes[node.0 as usize].kind {
        Kind::Text { content, .. } => {
            let chars = content.chars().count() as i16;
            Size::new(chars * FONT0_W + 2 * PAD, FONT0_H + 2 * PAD)
        }
        Kind::Button { label, .. } => {
            let chars = label.chars().count() as i16;
            Size::new(chars * FONT0_W + 2 * PAD, FONT0_H + 2 * PAD)
        }
        Kind::Column { children, spacing } => {
            let mut w = 0i16;
            let mut h = 0i16;
            let n = children.len();
            for (i, &c) in children.iter().enumerate() {
                let cs = measure_inner(rt, c);
                w = w.max(cs.w);
                h += cs.h;
                if i + 1 < n {
                    h += *spacing;
                }
            }
            Size::new(w, h)
        }
        Kind::Row { children, spacing } => {
            let mut w = 0i16;
            let mut h = 0i16;
            let n = children.len();
            for (i, &c) in children.iter().enumerate() {
                let cs = measure_inner(rt, c);
                h = h.max(cs.h);
                w += cs.w;
                if i + 1 < n {
                    w += *spacing;
                }
            }
            Size::new(w, h)
        }
        Kind::Carousel { children, offset, .. } => {
            // Centered carousel: VISIBLE slots are always shown (the window wraps), so height is
            // fixed at VISIBLE rows; width is the widest visible item.
            let n = children.len();
            let off = *offset;
            debug_assert!(n == 0 || off < n, "carousel offset out of bounds");
            let mut w = 0i16;
            if n > 0 {
                for k in 0..VISIBLE {
                    w = w.max(measure_inner(rt, children[(off + k) % n]).w);
                }
            }
            let rows = VISIBLE as i16;
            let h = rows * CAROUSEL_ROW_H + (rows - 1).max(0) * CAROUSEL_SPACING;
            Size::new(w, h)
        }
    }
}

/// Place absolute bounds: this node fills (x,y,w,h); a container lays each child out at the
/// child's measured size, advancing along the main axis by child extent + spacing.
fn place_inner(rt: &mut Runtime, node: NodeId, x: i16, y: i16, w: i16, h: i16) {
    rt.nodes[node.0 as usize].bounds = Rect::new(x, y, w, h);

    // Snapshot the (small) child list + spacing so the borrow on this node is released
    // before we mutate the children's bounds.
    let container = match &rt.nodes[node.0 as usize].kind {
        Kind::Column { children, spacing } => Some((true, children.clone(), *spacing)),
        Kind::Row { children, spacing } => Some((false, children.clone(), *spacing)),
        _ => None,
    };
    if let Some((is_col, children, spacing)) = container {
        let mut pos = if is_col { y } else { x };
        for &c in children.iter() {
            let cs = measure_inner(rt, c);
            if is_col {
                place_inner(rt, c, x, pos, w, cs.h); // stretch child to column width
                pos += cs.h + spacing;
            } else {
                place_inner(rt, c, pos, y, cs.w, h); // stretch child to row height
                pos += cs.w + spacing;
            }
        }
    }

    // Carousel: place the visible window as uniform full-width rows at constant y; hide the rest.
    let carousel = match &rt.nodes[node.0 as usize].kind {
        Kind::Carousel { children, offset, .. } => Some((children.clone(), *offset)),
        _ => None,
    };
    if let Some((children, offset)) = carousel {
        let n = children.len();
        if n > 0 {
            // Hide every child first, then place the VISIBLE window (wraps) into its slots. The
            // centered selection lands in the middle slot. Hiding first means a child that isn't
            // currently visible ends up off-screen even though the window wrapped past it.
            for &c in children.iter() {
                place_inner(rt, c, x, CAROUSEL_OFFSCREEN_Y, w, CAROUSEL_ROW_H);
            }
            for k in 0..VISIBLE {
                let c = children[(offset + k) % n];
                let yy = y + k as i16 * CAROUSEL_ROW_PITCH;
                place_inner(rt, c, x, yy, w, CAROUSEL_ROW_H);
            }
        }
    }
}

/// Public entry: place `root` to fill `area`, laying its descendants out within.
pub fn layout(root: NodeId, area: Rect) {
    with_runtime(|rt| place_inner(rt, root, area.x, area.y, area.w, area.h));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::node::{button, carousel, col, text_static, Kind};
    use crate::reactive::scope::Scope;

    #[test]
    fn column_stacks_children_with_spacing() {
        let cx = Scope::root();
        let root = col(cx, (text_static(cx, "ab"), text_static(cx, "cd")));
        layout(root, Rect::new(0, 0, 100, 100));
        let (y0, y1) = with_runtime(|rt| {
            let c = match &rt.nodes[root.0 as usize].kind {
                Kind::Column { children, .. } => (children[0], children[1]),
                _ => unreachable!(),
            };
            (rt.nodes[c.0.0 as usize].bounds.y, rt.nodes[c.1.0 as usize].bounds.y)
        });
        assert!(y1 > y0, "second child below first");
        cx.dispose();
    }

    fn make_carousel() -> (Scope, NodeId, [NodeId; 6]) {
        let cx = Scope::root();
        let items = [
            button(cx, "Power", || {}), button(cx, "Mode", || {}), button(cx, "Temp+", || {}),
            button(cx, "Temp-", || {}), button(cx, "Fan", || {}), button(cx, "Swing", || {}),
        ];
        let car = carousel(cx, &items);
        (cx, car, items)
    }

    #[test]
    fn carousel_places_visible_full_width_and_hides_the_rest() {
        let (cx, car, items) = make_carousel();
        layout(car, Rect::new(0, 0, 240, 120));
        with_runtime(|rt| {
            for &id in &items[0..3] {
                let b = rt.nodes[id.0 as usize].bounds;
                assert_eq!(b.w, 240, "visible rows are full carousel width");
                assert!(b.y < CAROUSEL_OFFSCREEN_Y, "visible row is on-screen");
                assert_eq!(b.h, CAROUSEL_ROW_H);
            }
            for &id in &items[3..6] {
                assert!(rt.nodes[id.0 as usize].bounds.y >= CAROUSEL_OFFSCREEN_Y, "hidden row off-screen");
            }
        });
        cx.dispose();
    }

    #[test]
    fn carousel_row_y_is_constant_regardless_of_offset() {
        let (cx, car, items) = make_carousel();
        layout(car, Rect::new(0, 0, 240, 120));
        let row0_y = with_runtime(|rt| rt.nodes[items[0].0 as usize].bounds.y);
        with_runtime(|rt| if let Kind::Carousel { offset, selected, .. } = &mut rt.nodes[car.0 as usize].kind {
            *offset = 2; *selected = 2;
        });
        layout(car, Rect::new(0, 0, 240, 120));
        let slot0_now = with_runtime(|rt| rt.nodes[items[2].0 as usize].bounds.y);
        assert_eq!(slot0_now, row0_y, "slot rects are constant across offset");
        cx.dispose();
    }

    #[test]
    fn column_stretches_children_to_its_width() {
        let cx = Scope::root();
        let t = text_static(cx, "hi");
        let root = col(cx, (t, text_static(cx, "yo")));
        layout(root, Rect::new(0, 0, 200, 100));
        assert_eq!(with_runtime(|rt| rt.nodes[t.0 as usize].bounds.w), 200);
        cx.dispose();
    }
}
