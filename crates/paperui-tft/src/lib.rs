#![no_std]
//! PaperUI M5StickC Plus2 backend: GPIO button-gesture input.
//! Device crate (esp-hal) — builds for xtensa-esp32-none-elf, not host-testable.

pub mod buttons;

pub use buttons::ButtonReader;
