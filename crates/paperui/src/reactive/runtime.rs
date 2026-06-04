//! The single global owner of all reactive storage. Accessed via `with_runtime`, which
//! takes ONE non-reentrant critical section. Never call `with_runtime` from inside another.

use crate::reactive::bounded_fn::BoundedFn;
use crate::reactive::node::{effect_of_text, handler_of_button, Node};
use crate::reactive::{Arena, CLOSURE_WORDS, FANOUT, SIGNAL_SLOT_WORDS};
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

pub(crate) const EMPTY_EFFECT: EffectSlot = EffectSlot { func: None, owner: OwnerId(0), in_use: false };

pub(crate) struct Runtime<'a> {
    pub signals: &'a mut [SignalSlot],
    pub effects: &'a mut [EffectSlot],
    pub owners_used: &'a mut [bool],
    pub nodes: Arena<'a, Node>,
    pub dirty: Arena<'a, NodeId>,
    pub current_effect: Option<NodeId>,
    pub focus: Option<NodeId>,
    pub epoch: u32,
}

pub(crate) const EMPTY_SIGNAL: SignalSlot = SignalSlot {
    value: [MaybeUninit::uninit(); SIGNAL_SLOT_WORDS],
    type_id: None,
    subs: Vec::new(),
    owner: OwnerId(0),
    in_use: false,
};

static RUNTIME: Mutex<RefCell<Option<Runtime<'static>>>> = Mutex::new(RefCell::new(None));

/// Install the engine's view of app-owned storage. Called once by `Storage::install`.
pub(crate) fn install_view(rt: Runtime<'static>) {
    critical_section::with(|cs| {
        let mut guard = RUNTIME.borrow_ref_mut(cs);
        debug_assert!(guard.is_none(), "paperui::reactive::install() called twice");
        *guard = Some(rt);
    });
}

/// Run `f` with exclusive access to the runtime. NON-REENTRANT: never nest.
/// Panics if `paperui::reactive::install` has not been called.
pub(crate) fn with_runtime<R>(f: impl FnOnce(&mut Runtime<'_>) -> R) -> R {
    critical_section::with(|cs| {
        let mut guard = RUNTIME.borrow_ref_mut(cs);
        let rt = guard
            .as_mut()
            .expect("paperui::reactive::install() must be called before building the UI");
        f(rt)
    })
}

impl Runtime<'_> {
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
        debug_assert!(false, "owner arena exhausted (raise the app's Storage)");
        OwnerId((self.owners_used.len() - 1) as u16)
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

    /// Return all pools to empty and clear cursors. Tests use this to isolate each case so the
    /// append-only node arena never accumulates across the suite.
    #[cfg(test)]
    pub(crate) fn reset(&mut self) {
        for s in self.signals.iter_mut() {
            s.in_use = false;
            s.subs.clear();
            s.type_id = None;
            s.owner = OwnerId(0);
        }
        for e in self.effects.iter_mut() {
            e.in_use = false;
            e.func = None;
            e.owner = OwnerId(0);
        }
        for u in self.owners_used.iter_mut() {
            *u = false;
        }
        self.nodes.clear();
        self.dirty.clear();
        self.current_effect = None;
        self.focus = None;
        self.epoch = 0;
    }
}

/// A `TypeId` wrapper so `signal.rs` can hand the stored type into `alloc_signal`.
pub(crate) struct TypeIdShim(pub core::any::TypeId);

impl Runtime<'_> {
    pub(crate) fn push_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u16);
        if !self.nodes.push(node) {
            debug_assert!(false, "node arena exhausted (raise the app's Storage NN)");
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
        debug_assert!(false, "signal arena exhausted (raise the app's Storage)");
        SignalId((self.signals.len() - 1) as u16)
    }
}

impl Runtime<'_> {
    pub(crate) fn alloc_effect(&mut self, owner: OwnerId, f: BoundedFn<CLOSURE_WORDS, ()>) -> EffectId {
        for (i, e) in self.effects.iter_mut().enumerate() {
            if !e.in_use {
                e.in_use = true;
                e.owner = owner;
                e.func = Some(f);
                return EffectId(i as u16);
            }
        }
        debug_assert!(false, "effect arena exhausted (raise the app's Storage)");
        EffectId((self.effects.len() - 1) as u16)
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

/// Test-only: install a fixed, modestly-sized runtime ONCE for the whole (single-threaded) test
/// binary, then reset it so each test starts empty. These are TEST sizes — they live here, not in
/// the library, and only need to fit the largest single test (not the suite's sum).
#[cfg(test)]
pub(crate) fn fresh_runtime() {
    use crate::reactive::storage::{install, Storage};
    use static_cell::StaticCell;
    static CELL: StaticCell<Storage<16, 64, 32, 16>> = StaticCell::new();
    if let Some(storage) = CELL.try_init(Storage::new()) {
        install(storage);
    }
    with_runtime(|rt| rt.reset());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alloc_and_free_owner_round_trips() {
        fresh_runtime();
        with_runtime(|rt| {
            let o = rt.alloc_owner();
            assert!(rt.owners_used[o.0 as usize]);
            rt.free_owner(o);
            assert!(!rt.owners_used[o.0 as usize]);
        });
    }
}
