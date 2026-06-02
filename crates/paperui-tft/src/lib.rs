#![no_std]
//! PaperUI M5StickC Plus2 backend: RMT IR transmit + GPIO button gestures.
//! Device crate (esp-hal) — builds for xtensa-esp32-none-elf, not host-testable.

pub mod buttons;
pub mod ir;

pub use buttons::ButtonReader;
pub use ir::IrTx;
