#![no_std]
//! PaperUI — a `no_std`, allocator-free UI framework for embedded displays.
//!
//! One crate holding the engine (the `Canvas`/`Widget`/`Theme`/`WidgetTheme`/
//! `Renderer`/`InputSource` traits, geometry, `State`, button gestures) and the
//! pure-logic widgets. Concrete themes (colors + look) live in the board addon
//! crates (`paperui-tft`, `paperui-eink`) or in user crates — this crate ships
//! the `WidgetTheme` *contract*, not a concrete look. It depends on **no graphics
//! library** by default; the `embedded-graphics` adapter is behind the `eg`
//! feature, and a recording mock `Canvas` for host tests is behind `mock`.
//!
//! Device backends (esp-hal, display/IR/touch drivers) live in separate crates
//! (`paperui-tft`, `paperui-eink`) so an app only ever compiles the board it uses.

// --- engine ---
mod canvas;
mod draw;
mod geometry;
mod input;
mod render;
mod state;
mod types;
mod widget;
// --- widgets (always on; pure logic, no extra deps) ---
mod button;
mod widget_theme;
// --- optional embedded-graphics adapter ---
#[cfg(feature = "eg")]
mod eg;

pub use canvas::{Canvas, FONT0_H, FONT0_W};
#[cfg(feature = "mock")]
pub use canvas::{DrawOp, MockCanvas};
pub use draw::{DrawCtx, Theme};
pub use geometry::{Point, Rect, Size};
pub use input::{GestureState, InputSource, DOUBLE_MS, HOLD_MS};
pub use render::Renderer;
pub use state::State;
pub use types::{ButtonEvent, ButtonId, Color, Constraints, FontId, UpdateHint};
pub use widget::Widget;

pub use button::Button;
pub use widget_theme::WidgetTheme;

#[cfg(feature = "eg")]
pub use eg::{to_rgb565, EgCanvas};
