#![no_std]
//! PaperUI M5Paper backend: self-authored IT8951 e-ink driver with sleep/wake
//! power lifecycle, a 4bpp grayscale Canvas, an UpdateHint→waveform EinkRenderer,
//! and GT911 touch. Device crate (esp-hal) — builds for xtensa, not host-testable.

pub mod canvas;
pub mod it8951;
pub mod renderer;
pub mod touch;

pub use canvas::Gray4Canvas;
pub use it8951::It8951;
pub use renderer::EinkRenderer;
pub use touch::TouchInput;
