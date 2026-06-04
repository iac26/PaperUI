#![deny(unsafe_code)] // bounded_fn.rs re-enables unsafe locally via #![allow]. (deny, not forbid — forbid can't be overridden.)

// --- per-element / inline-storage bounds (how big ONE element is, not how many can exist;
//     pool counts are app-owned via `Storage`) ---
pub const FANOUT: usize = 4;          // subscribers per signal
pub const MAX_CHILDREN: usize = 8;    // children per container
pub const TEXT_CAP: usize = 32;       // chars per Text node
pub const SIGNAL_SLOT_WORDS: usize = 2;  // inline signal payload words
pub const CLOSURE_WORDS: usize = 8;      // inline BoundedFn words

mod bounded_fn;
pub use bounded_fn::BoundedFn;

mod arena;
pub(crate) use arena::Arena;

mod runtime;
pub use runtime::{EffectId, NodeId, OwnerId, SignalId};
#[allow(unused_imports)]
pub(crate) use runtime::with_runtime;

mod storage;
pub use storage::{install, Storage};

/// Name the four pool sizes in one place; expands to a `Storage<…>` TYPE so it reads well in a
/// `StaticCell<storage!(…)>` declaration.
/// All four keyword labels are required. Example:
/// `static S: StaticCell<storage!(signals: 1, nodes: 4, effects: 2, owners: 1)> = StaticCell::new();`
#[macro_export]
macro_rules! storage {
    (signals: $s:expr, nodes: $n:expr, effects: $e:expr, owners: $o:expr $(,)?) => {
        $crate::reactive::Storage<{ $s }, { $n }, { $e }, { $o }>
    };
}
pub use crate::storage;

mod signal;
pub use signal::{ReactiveValue, Signal};

mod scope;
pub use scope::{use_state, Scope};

mod node;
pub use node::{button, carousel, carousel_select_first, col, row, text, text_static, IntoChildren, Kind, Node, TextSource};
pub use node::{ANIM_STEPS, VISIBLE};

mod layout;
pub use layout::layout;

mod render;
pub use render::{render_frame_full, render_tick};

mod driver;
pub use driver::{dispatch, run, EventSource, UiEvent};

#[cfg(test)]
mod macro_tests {
    use crate::reactive::runtime::fresh_runtime;
    #[test]
    fn storage_macro_expands_to_a_constructible_type() {
        fresh_runtime();
        let _s: crate::storage!(signals: 1, nodes: 2, effects: 1, owners: 1) =
            crate::reactive::Storage::new();
        let _s2: crate::reactive::storage!(signals: 1, nodes: 1, effects: 1, owners: 1) =
            crate::reactive::Storage::new();
    }
}
