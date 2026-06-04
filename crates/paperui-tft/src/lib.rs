#![no_std]
//! PaperUI M5StickC Plus2 board addon.
//!
//! Provides this board's concrete look — [`TftTheme`], a full-color style that is
//! pure logic over `paperui::Canvas` and therefore host-testable — its physical-button
//! gesture detection ([`GestureState`] → [`ButtonEvent`]/[`ButtonId`]), and the GPIO
//! reader [`ButtonReader`] that drives it, which needs esp-hal and so sits behind the
//! default `hal` feature (xtensa-only). Disable default features to build/test the
//! theme and gesture logic on host.

mod theme;
pub use theme::TftTheme;

// Button gestures are pure (no esp-hal), so the module stays ungated and host-testable; only
// the GPIO reader that consumes them sits behind `hal`.
mod input;
pub use input::{ButtonEvent, ButtonId, GestureState, DOUBLE_MS, HOLD_MS};

#[cfg(feature = "hal")]
mod buttons;
#[cfg(feature = "hal")]
pub use buttons::ButtonReader;
