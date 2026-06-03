//! The single global owner of all reactive storage. Accessed via `with_runtime`, which
//! takes ONE non-reentrant critical section. Never call `with_runtime` from inside another.

use crate::reactive::{FANOUT, N_NODES, N_OWNERS, N_SIGNALS, SIGNAL_SLOT_WORDS};
use core::cell::RefCell;
use core::mem::MaybeUninit;
use critical_section::Mutex;
use heapless::Vec;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct OwnerId(pub u16);
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SignalId(pub u16);
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct EffectId(pub u16);
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct NodeId(pub u16);

#[allow(dead_code)]
pub(crate) struct SignalSlot {
    pub value: [MaybeUninit<usize>; SIGNAL_SLOT_WORDS],
    pub type_id: Option<core::any::TypeId>,
    pub subs: Vec<NodeId, FANOUT>,
    pub owner: OwnerId,
    pub in_use: bool,
}

#[allow(dead_code)]
pub(crate) struct Runtime {
    pub signals: [SignalSlot; N_SIGNALS],
    pub owners_used: [bool; N_OWNERS],
    pub current_effect: Option<NodeId>,
    pub dirty: Vec<NodeId, N_NODES>,
    pub epoch: u32,
    // node/effect arenas added in later tasks
}

#[allow(dead_code)]
const EMPTY_SIGNAL: SignalSlot = SignalSlot {
    value: [MaybeUninit::uninit(); SIGNAL_SLOT_WORDS],
    type_id: None,
    subs: Vec::new(),
    owner: OwnerId(0),
    in_use: false,
};

#[allow(dead_code)]
static RUNTIME: Mutex<RefCell<Runtime>> = Mutex::new(RefCell::new(Runtime {
    signals: [EMPTY_SIGNAL; N_SIGNALS],
    owners_used: [false; N_OWNERS],
    current_effect: None,
    dirty: Vec::new(),
    epoch: 0,
}));

/// Run `f` with exclusive access to the runtime. NON-REENTRANT: never nest.
#[allow(dead_code)]
pub(crate) fn with_runtime<R>(f: impl FnOnce(&mut Runtime) -> R) -> R {
    critical_section::with(|cs| f(&mut RUNTIME.borrow_ref_mut(cs)))
}

#[allow(dead_code)]
impl Runtime {
    pub(crate) fn alloc_owner(&mut self) -> OwnerId {
        for (i, used) in self.owners_used.iter_mut().enumerate() {
            if !*used {
                *used = true;
                return OwnerId(i as u16);
            }
        }
        debug_assert!(false, "owner arena exhausted (raise N_OWNERS)");
        OwnerId((N_OWNERS - 1) as u16)
    }

    pub(crate) fn free_owner(&mut self, o: OwnerId) {
        // free signals owned by this owner
        for s in self.signals.iter_mut() {
            if s.in_use && s.owner == o {
                s.in_use = false;
                s.subs.clear();
                s.type_id = None;
            }
        }
        self.owners_used[o.0 as usize] = false;
    }
}

/// A `TypeId` wrapper so `signal.rs` can hand the stored type into `alloc_signal`.
pub(crate) struct TypeIdShim(pub core::any::TypeId);

#[allow(dead_code)]
impl Runtime {
    pub(crate) fn alloc_signal(&mut self, owner: OwnerId, tid: TypeIdShim) -> SignalId {
        for (i, s) in self.signals.iter_mut().enumerate() {
            if !s.in_use {
                s.in_use = true;
                s.owner = owner;
                s.type_id = Some(tid.0);
                s.subs.clear();
                return SignalId(i as u16);
            }
        }
        debug_assert!(false, "signal arena exhausted (raise N_SIGNALS)");
        SignalId((N_SIGNALS - 1) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alloc_and_free_owner_round_trips() {
        with_runtime(|rt| {
            let o = rt.alloc_owner();
            assert!(rt.owners_used[o.0 as usize]);
            rt.free_owner(o);
            assert!(!rt.owners_used[o.0 as usize]);
        });
    }
}
