#![no_std]
//! PaperUI e-ink theme: high-contrast, mono-friendly rendering for IT8951 panels.
//! Depends only on core + widgets (no device deps) — host-testable.

mod eink_theme;
pub use eink_theme::EinkTheme;
