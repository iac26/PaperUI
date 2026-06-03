//! Layer #1 reactive core: signals + scopes + node tree + sync driver. `no_std`, zero-heap.
//! See docs/superpowers/specs/2026-06-03-paperui-reactive-core-design.md.
#![deny(unsafe_code)] // bounded_fn.rs re-enables unsafe locally via #![allow]. (deny, not forbid — forbid can't be overridden.)

// --- compile-time capacities (override via patching these consts) ---
pub const N_SIGNALS: usize = 32;
pub const N_NODES: usize = 64;
pub const N_EFFECTS: usize = 32;
pub const N_OWNERS: usize = 16;
pub const FANOUT: usize = 4;
pub const MAX_CHILDREN: usize = 8;
pub const TEXT_CAP: usize = 32;
pub const SIGNAL_SLOT_WORDS: usize = 2;
pub const CLOSURE_WORDS: usize = 8;

mod bounded_fn;
pub use bounded_fn::BoundedFn;

mod runtime;
pub use runtime::{EffectId, NodeId, OwnerId, SignalId};
#[allow(unused_imports)]
pub(crate) use runtime::with_runtime;

mod signal;
pub use signal::{ReactiveValue, Signal};

mod scope;
pub use scope::{use_state, Scope};

mod node;
pub use node::{col, row, text_static, IntoChildren, Kind, Node, TextSource};
