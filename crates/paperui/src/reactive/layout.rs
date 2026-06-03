//! Two-pass layout: measure desired sizes, then place absolute bounds. Pure integer math.
//! A leaf (Text/Button) is chars*FONT0_W + 2*PAD wide, FONT0_H + 2*PAD tall. A Column stacks
//! children (max width, summed heights + spacing); a Row is the transpose.

use crate::canvas::{FONT0_H, FONT0_W};
use crate::geometry::{Rect, Size};
use crate::reactive::node::Kind;
use crate::reactive::runtime::{with_runtime, NodeId, Runtime};

const PAD: i16 = 6;

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
        Kind::Carousel { .. } => Size::new(0, 0), // layout handled by Task 3
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
                place_inner(rt, c, x, pos, cs.w, cs.h);
                pos += cs.h + spacing;
            } else {
                place_inner(rt, c, pos, y, cs.w, cs.h);
                pos += cs.w + spacing;
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
    use crate::reactive::node::{col, text_static};
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
}
