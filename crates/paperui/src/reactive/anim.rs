//! Time-driven reactive values ("tweens") riding on `Signal<T>`. The driver interpolates a
//! `from`→`to` value over a duration and writes it through the normal signal path, dirtying
//! subscribers (and `dirty_node`, for non-effect readers) so the existing surgical repaint runs.

// `Animatable` is a public bound (on `Scope::tween` / `Animated`), but `AnimValue` is a
// crate-internal bridge type — the trait is effectively sealed by `AnimValue`'s `pub(crate)`
// visibility. The resulting private-in-public on the trait's `to_av`/`from_av` is allowed by design.
#![allow(private_interfaces)]

use crate::paint::Color;
use crate::reactive::runtime::{with_runtime, AnimId, NodeId, Runtime};
use crate::reactive::signal::{set_signal_av, ReactiveValue, Signal};
use core::marker::PhantomData;

/// Easing curve. Linear only for Layer-1; the enum reserves room for eased curves
/// (integer-approximated) without an API break.
pub enum Easing {
    Linear,
}

/// Timing for one animation.
pub struct Anim {
    pub dur_ms: u16,
    pub easing: Easing,
}

/// Closed set of animatable payloads (decision: closed enum, not erased fn-hooks). Carries the
/// concrete type at runtime so the driver can interpolate and write the signal slot without being
/// generic over `T`.
#[derive(Copy, Clone, PartialEq)]
pub(crate) enum AnimValue {
    I16(i16),
    Color(Color),
    Frame(u8),
}

/// Which interpolation regime an animator runs. `FrameLoop` is specced but not advanced yet.
#[derive(Copy, Clone)]
pub(crate) enum AnimKind {
    Transition,
    #[allow(dead_code)] // built later: no frame-loop consumer in phase 1 (YAGNI).
    FrameLoop {
        n: u8,
    },
}

/// Map a typed payload to/from `AnimValue`. Implemented for the closed set only.
pub trait Animatable: ReactiveValue {
    fn to_av(self) -> AnimValue;
    fn from_av(av: AnimValue) -> Self;
}

impl Animatable for i16 {
    fn to_av(self) -> AnimValue {
        AnimValue::I16(self)
    }
    fn from_av(av: AnimValue) -> Self {
        match av {
            AnimValue::I16(v) => v,
            _ => 0,
        }
    }
}

impl Animatable for Color {
    fn to_av(self) -> AnimValue {
        AnimValue::Color(self)
    }
    fn from_av(av: AnimValue) -> Self {
        match av {
            AnimValue::Color(c) => c,
            _ => Color(0),
        }
    }
}

impl Animatable for u8 {
    fn to_av(self) -> AnimValue {
        AnimValue::Frame(self)
    }
    fn from_av(av: AnimValue) -> Self {
        match av {
            AnimValue::Frame(f) => f,
            _ => 0,
        }
    }
}

impl AnimValue {
    /// Integer interpolation `from + (to - from) * num / den` (guard `den >= 1`). `Color` lerps each
    /// RGB565 channel; `Frame` is not lerped (frame-loops index, they don't tween) and returns `to`.
    pub(crate) fn lerp(from: AnimValue, to: AnimValue, num: u32, den: u32) -> AnimValue {
        let den = den.max(1);
        match (from, to) {
            (AnimValue::I16(a), AnimValue::I16(b)) => {
                AnimValue::I16(lerp_i16(a, b, num, den))
            }
            (AnimValue::Color(a), AnimValue::Color(b)) => {
                // RRRRR_GGGGGG_BBBBB
                let ar = (a.0 >> 11) & 0x1F;
                let ag = (a.0 >> 5) & 0x3F;
                let ab = a.0 & 0x1F;
                let br = (b.0 >> 11) & 0x1F;
                let bg = (b.0 >> 5) & 0x3F;
                let bb = b.0 & 0x1F;
                let r = lerp_i16(ar as i16, br as i16, num, den) as u16;
                let g = lerp_i16(ag as i16, bg as i16, num, den) as u16;
                let bch = lerp_i16(ab as i16, bb as i16, num, den) as u16;
                AnimValue::Color(Color((r << 11) | (g << 5) | bch))
            }
            // Mixed/Frame: not tweened — snap to the target.
            _ => to,
        }
    }
}

/// `a + (b - a) * num / den`, in `i32` to avoid `i16` overflow mid-interpolation.
fn lerp_i16(a: i16, b: i16, num: u32, den: u32) -> i16 {
    let a = a as i32;
    let b = b as i32;
    (a + (b - a) * num as i32 / den as i32) as i16
}

/// A clock-driven reactive value. Reads like a `Signal<T>`; `animate_to` paces a transition that
/// the driver advances each tick.
#[derive(Copy, Clone)]
pub struct Animated<T> {
    pub(crate) sig: Signal<T>,
    pub(crate) anim: AnimId,
    _pd: PhantomData<T>,
}

impl<T: Animatable> Animated<T> {
    pub(crate) fn new(sig: Signal<T>, anim: AnimId) -> Self {
        Self { sig, anim, _pd: PhantomData }
    }

    /// Current (interpolated) value. Locks + subscribes the running effect, exactly like
    /// `Signal::get`; for effect/app code.
    pub fn get(self) -> T {
        self.sig.get()
    }

    /// GOTCHA: read the current value when you ALREADY hold the runtime (inside `with_runtime`).
    /// Does not lock or subscribe; using `get` there would re-enter the critical section.
    pub(crate) fn get_in(self, rt: &Runtime) -> T {
        self.sig.get_in(rt)
    }

    /// Jump immediately and cancel any in-flight animation.
    pub fn set_immediate(self, v: T) {
        self.sig.set(v);
        with_runtime(|rt| rt.animators[self.anim.0 as usize].active = false);
    }

    /// Start a transition: `from` = current value, `to` = target, `start_ms` = now, `active`.
    pub fn animate_to(self, target: T) {
        with_runtime(|rt| {
            let f = self.get_in(rt).to_av();
            let a = &mut rt.animators[self.anim.0 as usize];
            a.from = f;
            a.to = target.to_av();
            a.start_ms = rt.now_ms;
            a.active = true;
        });
    }

    /// Route this animator's progress to a non-effect node (e.g. the carousel), dirtied directly
    /// each tick while active. Consumed when the carousel becomes a windowed viewport (later task).
    #[allow(dead_code)]
    pub(crate) fn set_dirty_node(self, n: NodeId) {
        with_runtime(|rt| rt.animators[self.anim.0 as usize].dirty_node = Some(n));
    }
}

/// Advance every active animator to `now`: interpolate, write the signal (dirtying subscribers),
/// dirty any `dirty_node`, and deactivate + snap to `to` once the duration elapses.
pub(crate) fn advance_anims(rt: &mut Runtime, now: u32) {
    rt.now_ms = now;
    for i in 0..rt.animators.len() {
        if !rt.animators[i].active {
            continue;
        }
        let a = rt.animators[i];
        let e = now.wrapping_sub(a.start_ms).min(a.dur_ms as u32);
        set_signal_av(rt, a.signal, AnimValue::lerp(a.from, a.to, e, a.dur_ms as u32));
        if let Some(n) = a.dirty_node {
            if !rt.dirty.contains(&n) {
                let _ = rt.dirty.push(n);
            }
        }
        if e >= a.dur_ms as u32 {
            set_signal_av(rt, a.signal, a.to);
            rt.animators[i].active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::runtime::fresh_runtime;
    use crate::reactive::scope::Scope;

    fn lerp_i(from: i16, to: i16, num: u32, den: u32) -> i16 {
        i16::from_av(AnimValue::lerp(AnimValue::I16(from), AnimValue::I16(to), num, den))
    }

    #[test]
    fn lerp_i16_endpoints_and_midpoint() {
        assert_eq!(lerp_i(0, 100, 50, 100), 50);
        // num = 0 -> from; num = den -> to.
        assert_eq!(lerp_i(0, 100, 0, 100), 0);
        assert_eq!(lerp_i(0, 100, 100, 100), 100);
        // den guarded to >= 1 (no divide-by-zero panic).
        assert_eq!(lerp_i(0, 100, 0, 0), 0);
    }

    #[test]
    fn lerp_color_midpoint_is_roughly_half_each_channel() {
        let mid = AnimValue::lerp(
            AnimValue::Color(Color::BLACK),
            AnimValue::Color(Color::WHITE),
            1,
            2,
        );
        let c = match mid {
            AnimValue::Color(c) => c,
            _ => unreachable!(),
        };
        // BLACK..WHITE midpoint: R=15/31, G=31/63, B=15/31 -> (15<<11)|(31<<5)|15 = 0x7BEF.
        assert_eq!(c.0, 0x7BEF);
        let r = (c.0 >> 11) & 0x1F;
        let g = (c.0 >> 5) & 0x3F;
        let b = c.0 & 0x1F;
        assert_eq!((r, g, b), (15, 31, 15));
    }

    #[test]
    fn tween_progresses_linearly_then_deactivates() {
        fresh_runtime();
        let cx = Scope::root();
        let t = cx.tween(0i16, Anim { dur_ms: 100, easing: Easing::Linear });
        with_runtime(|rt| rt.now_ms = 0);
        t.animate_to(100);

        with_runtime(|rt| advance_anims(rt, 0));
        assert_eq!(t.get(), 0, "at t=0 the value is still `from`");
        assert!(with_runtime(|rt| rt.animators[t.anim.0 as usize].active), "still animating at t=0");

        with_runtime(|rt| advance_anims(rt, 50));
        assert_eq!(t.get(), 50, "at t=dur/2 the value is the midpoint");
        assert!(with_runtime(|rt| rt.animators[t.anim.0 as usize].active), "still animating at t=dur/2");

        with_runtime(|rt| advance_anims(rt, 100));
        assert_eq!(t.get(), 100, "at t=dur the value is `to`");
        assert!(!with_runtime(|rt| rt.animators[t.anim.0 as usize].active), "deactivated at t=dur");
        cx.dispose();
    }
}
