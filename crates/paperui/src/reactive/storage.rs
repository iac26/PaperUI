//! App-owned backing storage for the reactive runtime. This is the ONLY place the four pool
//! sizes live: the app declares a concrete `Storage<NS,NN,NE,NO>` as a `static` and hands the
//! engine a borrow via `install`. No capacity const lives in the library.

use crate::reactive::node::{Node, EMPTY_NODE};
use crate::reactive::runtime::{EffectSlot, NodeId, SignalSlot, EMPTY_EFFECT, EMPTY_SIGNAL};

/// Fixed-capacity backing arrays for the reactive runtime.
/// - `NS` signals, `NN` nodes (also bounds the dirty list), `NE` effects, `NO` owners.
pub struct Storage<const NS: usize, const NN: usize, const NE: usize, const NO: usize> {
    pub(crate) signals: [SignalSlot; NS],
    pub(crate) effects: [EffectSlot; NE],
    pub(crate) owners_used: [bool; NO],
    pub(crate) nodes: [Node; NN],
    // backing slice for Arena<'_, NodeId> in the future Runtime<'a> view (Task 2)
    pub(crate) dirty: [NodeId; NN],
}

impl<const NS: usize, const NN: usize, const NE: usize, const NO: usize> Default
    for Storage<NS, NN, NE, NO>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const NS: usize, const NN: usize, const NE: usize, const NO: usize> Storage<NS, NN, NE, NO> {
    /// Const-construct empty backing arrays. Place this in a `static` and pass a
    /// `&'static mut` borrow to `install()` before building any UI nodes.
    pub const fn new() -> Self {
        Self {
            signals: [EMPTY_SIGNAL; NS],
            effects: [EMPTY_EFFECT; NE],
            owners_used: [false; NO],
            nodes: [EMPTY_NODE; NN],
            dirty: [NodeId(0); NN],
        }
    }
}

use crate::reactive::runtime::{install_view, Runtime};
use crate::reactive::Arena;

/// Hand the engine a borrow of app-owned storage. Call exactly once, before any `Scope::root`,
/// `use_state`, or view builder. A missing call makes the first reactive op panic (not UB).
pub fn install<const NS: usize, const NN: usize, const NE: usize, const NO: usize>(
    storage: &'static mut Storage<NS, NN, NE, NO>,
) {
    let view = Runtime {
        signals: &mut storage.signals,
        effects: &mut storage.effects,
        owners_used: &mut storage.owners_used,
        nodes: Arena::new(&mut storage.nodes),
        dirty: Arena::new(&mut storage.dirty),
        current_effect: None,
        focus: None,
        epoch: 0,
    };
    install_view(view);
}
