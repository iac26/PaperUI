#![no_std]
//! PaperUI M5Paper board addon.
//!
//! Provides this board's concrete look — [`EinkTheme`], pure logic over
//! `paperui::Canvas` and host-testable — plus a self-authored IT8951 e-ink driver
//! with sleep/wake power lifecycle ([`EinkRenderer`]), a 4bpp grayscale
//! [`Gray4Canvas`], and GT911 touch. The driver/renderer/canvas are generic over
//! `embedded-hal` traits; only the GT911 touch input needs esp-hal, so it sits
//! behind the default `hal` feature. Disable default features to build/test the
//! theme (and the generic driver) on host.

mod theme;
pub use theme::EinkTheme;

pub mod canvas;
pub mod it8951;
pub mod renderer;

pub use canvas::Gray4Canvas;
pub use it8951::It8951;
pub use renderer::EinkRenderer;

#[cfg(feature = "hal")]
pub mod touch;
#[cfg(feature = "hal")]
pub use touch::TouchInput;
