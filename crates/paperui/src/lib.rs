#![no_std]
//! PaperUI — a `no_std`, allocator-free UI framework for embedded displays.
//!
//! One crate holding the reactive engine (signals + scopes + a retained node tree +
//! a sync driver, behind `feature = "reactive"`) and the shared substrate it renders
//! through: the `Canvas`/`WidgetTheme` traits and geometry. Input enters the engine as
//! `reactive::UiEvent` via the `EventSource` trait; turning raw hardware (buttons, touch)
//! into those events is the board crates' job, not this one's.
//! Concrete themes (colors + look) live in the board addon
//! crates (`paperui-tft`, `paperui-eink`) or in user crates — this crate ships
//! the `WidgetTheme` *contract*, not a concrete look. It depends on **no graphics
//! library** by default; the `embedded-graphics` adapter is behind the `eg`
//! feature, and a recording mock `Canvas` for host tests is behind `mock`.
//!
//! Device backends (esp-hal, display/IR/touch drivers) live in separate crates
//! (`paperui-tft`, `paperui-eink`) so an app only ever compiles the board it uses.

// --- value types ---
pub mod geometry; // Point, Rect, Size, Constraints
// --- drawing substrate: the Canvas surface, DrawCtx, WidgetTheme, and backends ---
pub mod paint;
// --- reactive engine: signals + scopes + node tree + sync driver ---
#[cfg(feature = "reactive")]
pub mod reactive;

// Ergonomic flat re-exports (`paperui::Color`, …); the modules above are also public.
pub use geometry::{Constraints, Point, Rect, Size};
pub use paint::{Canvas, Color, DrawCtx, FontId, UpdateHint, WidgetTheme, FONT0_H, FONT0_W};
#[cfg(feature = "mock")]
pub use paint::{DrawOp, MockCanvas};
#[cfg(feature = "eg")]
pub use paint::{to_rgb565, EgCanvas};
