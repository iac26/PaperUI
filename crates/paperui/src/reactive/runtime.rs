//! The single global owner of all reactive storage. Accessed via `with_runtime`, which
//! takes ONE non-reentrant critical section. Never call `with_runtime` from inside another.

use crate::reactive::bounded_fn::BoundedFn;
use crate::reactive::node::{effect_of_text, handler_of_button, Node};
use crate::reactive::{CLOSURE_WORDS, FANOUT, N_EFFECTS, N_NODES, N_OWNERS, N_SIGNALS, SIGNAL_SLOT_WORDS};
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

pub(crate) struct EffectSlot {
    pub func: Option<BoundedFn<CLOSURE_WORDS, ()>>,
    pub owner: OwnerId,
    pub in_use: bool,
}

const EMPTY_EFFECT: EffectSlot = EffectSlot { func: None, owner: OwnerId(0), in_use: false };

#[allow(dead_code)]
pub(crate) struct Runtime {
    pub signals: [SignalSlot; N_SIGNALS],
    pub owners_used: [bool; N_OWNERS],
    pub current_effect: Option<NodeId>,
    pub focus: Option<NodeId>,
    pub dirty: Vec<NodeId, N_NODES>,
    pub epoch: u32,
    pub nodes: Vec<Node, N_NODES>,
    pub effects: [EffectSlot; N_EFFECTS],
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
    focus: None,
    dirty: Vec::new(),
    epoch: 0,
    nodes: Vec::new(),
    effects: [EMPTY_EFFECT; N_EFFECTS],
}));

/// Run `f` with exclusive access to the runtime. NON-REENTRANT: never nest.
#[allow(dead_code)]
pub(crate) fn with_runtime<R>(f: impl FnOnce(&mut Runtime) -> R) -> R {
    critical_section::with(|cs| f(&mut RUNTIME.borrow_ref_mut(cs)))
}

#[allow(dead_code)]
impl Runtime {
    pub(crate) fn alloc_owner(&mut self) -> OwnerId {
        // OwnerId(0) is reserved as a sentinel "no real owner" (the EMPTY_SIGNAL placeholder
        // owner and the default). Real scopes start at 1 so a scope's dispose never sweeps
        // placeholder/leaked slots tagged with owner 0.
        for (i, used) in self.owners_used.iter_mut().enumerate().skip(1) {
            if !*used {
                *used = true;
                return OwnerId(i as u16);
            }
        }
        debug_assert!(false, "owner arena exhausted (raise N_OWNERS)");
        OwnerId((N_OWNERS - 1) as u16)
    }

    pub(crate) fn free_owner(&mut self, o: OwnerId) {
        for s in self.signals.iter_mut() {
            if s.in_use && s.owner == o {
                s.in_use = false;
                s.subs.clear();
                s.type_id = None;
            }
        }
        for e in self.effects.iter_mut() {
            if e.in_use && e.owner == o {
                e.in_use = false;
                e.func = None;
            }
        }
        self.owners_used[o.0 as usize] = false;
        // NOTE: nodes are intentionally NOT removed. NodeId is an index into the `nodes`
        // Vec; removing a node would shift later indices and corrupt every other NodeId.
        // Layer #1 builds the tree once and runs forever, so append-only is correct here;
        // node reclamation across screens is a Layer #2 (navigation) concern.
        //
        // LAYER #2 PRECONDITION: do not dispose an owner whose effect is mid-call (its func
        // is currently taken out of the slot by run_effect_of/invoke_handler_of). Clearing
        // such a slot here, then having the put-back write the func back, would resurrect a
        // freed slot. Unreachable in Layer #1 (single-threaded, no dispose-from-within-effect);
        // before Layer #2 allows it, gate the put-back on the slot still being `in_use`.
    }
}

/// A `TypeId` wrapper so `signal.rs` can hand the stored type into `alloc_signal`.
pub(crate) struct TypeIdShim(pub core::any::TypeId);

#[allow(dead_code)]
impl Runtime {
    pub(crate) fn push_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u16);
        if self.nodes.push(node).is_err() {
            debug_assert!(false, "node arena exhausted (raise N_NODES)");
        }
        id
    }

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

#[allow(dead_code)]
impl Runtime {
    pub(crate) fn alloc_effect(&mut self, owner: OwnerId, f: BoundedFn<CLOSURE_WORDS, ()>) -> EffectId {
        for (i, e) in self.effects.iter_mut().enumerate() {
            if !e.in_use {
                e.in_use = true;
                e.owner = owner;
                e.func = Some(f);
                return EffectId(i as u16);
            }
        }
        debug_assert!(false, "effect arena exhausted (raise N_EFFECTS)");
        EffectId((N_EFFECTS - 1) as u16)
    }
}

/// Look up the effect that backs a Text node and run it with dependency tracking on.
/// The closure is moved OUT of the lock before calling, so its inner `get()`/`set()`
/// (which re-lock) never nest inside this `with_runtime`.
pub(crate) fn run_effect_of(node: NodeId) {
    let taken = with_runtime(|rt| {
        let eid = effect_of_text(rt, node)?;
        rt.effects[eid.0 as usize].func.take().map(|f| {
            // Save the previous tracking cursor and install ours; we restore it (not blindly
            // clear to None) on put-back so a nested effect (Layer #2 derived/memo effects)
            // can't clobber an outer effect's subscription. We only touch current_effect once
            // the func is actually taken, so a re-entrant skip (func already None) is a no-op.
            let prev = rt.current_effect;
            rt.current_effect = Some(node);
            (eid, prev, f)
        })
    });
    if let Some((eid, prev, mut f)) = taken {
        f.call(); // runs OUTSIDE the lock; its get()s re-lock briefly
        with_runtime(|rt| {
            rt.current_effect = prev;
            rt.effects[eid.0 as usize].func = Some(f);
        });
    }
}

pub(crate) fn invoke_handler_of(node: NodeId) {
    let taken = with_runtime(|rt| {
        let eid = handler_of_button(rt, node)?;
        rt.effects[eid.0 as usize].func.take().map(|f| (eid, f))
    });
    if let Some((eid, mut f)) = taken {
        f.call();
        with_runtime(|rt| rt.effects[eid.0 as usize].func = Some(f));
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
