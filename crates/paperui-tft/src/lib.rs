#![no_std]
//! PaperUI M5StickC Plus2 board addon.
//!
//! Provides this board's concrete look — [`TftTheme`], a full-color style that is
//! pure logic over `paperui::Canvas` and therefore host-testable — plus the GPIO
//! button-gesture reader [`ButtonReader`], which needs esp-hal and so sits behind
//! the default `hal` feature (xtensa-only). Disable default features to build/test
//! just the theme on host.

mod theme;
pub use theme::TftTheme;

#[cfg(feature = "hal")]
mod buttons;
#[cfg(feature = "hal")]
pub use buttons::ButtonReader;
