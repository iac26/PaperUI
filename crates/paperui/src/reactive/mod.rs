//! Layer #1 reactive core: signals + scopes + node tree + sync driver. `no_std`, zero-heap.
//! See docs/superpowers/specs/2026-06-03-paperui-reactive-core-design.md.
#![deny(unsafe_code)] // bounded_fn.rs re-enables unsafe locally via #![allow]. (deny, not forbid — forbid can't be overridden.)

// --- compile-time capacities (override via patching these consts) ---
pub const N_SIGNALS: usize = 32;
// Nodes are append-only within a runtime (Layer #1 never reclaims node slots), and the host test
// suite shares ONE global runtime, so node usage accumulates across all tests in a run. An app
// needs only a handful (~9 for the Electrolux remote); this headroom is sized for the test suite.
pub const N_NODES: usize = 256;
pub const N_EFFECTS: usize = 32;
pub const N_OWNERS: usize = 16;
pub const FANOUT: usize = 4;
pub const MAX_CHILDREN: usize = 8;
pub const TEXT_CAP: usize = 32;
pub const VISIBLE: usize = 3;
pub const ANIM_STEPS: u8 = 3;
pub const SIGNAL_SLOT_WORDS: usize = 2;
pub const CLOSURE_WORDS: usize = 8;

mod bounded_fn;
pub use bounded_fn::BoundedFn;

mod arena;
pub(crate) use arena::Arena;

mod runtime;
pub use runtime::{EffectId, NodeId, OwnerId, SignalId};
#[allow(unused_imports)]
pub(crate) use runtime::with_runtime;

mod storage;
pub use storage::Storage;

mod signal;
pub use signal::{ReactiveValue, Signal};

mod scope;
pub use scope::{use_state, Scope};

mod node;
pub use node::{button, carousel, carousel_select_first, col, row, text, text_static, IntoChildren, Kind, Node, TextSource};

mod layout;
pub use layout::layout;

mod render;
pub use render::{render_frame_full, render_tick};

mod driver;
pub use driver::{dispatch, run, EventSource, UiEvent};
