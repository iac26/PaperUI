#![no_std]
//! PaperUI engine core: device- and graphics-library-agnostic UI infrastructure.
//! Defines the Canvas/Widget/Theme/State traits. Contains NO concrete widgets,
//! NO concrete rendering, NO device code.

mod geometry;
pub use geometry::{Point, Rect, Size};

mod types;
pub use types::{ButtonEvent, ButtonId, Color, Constraints, FontId, UpdateHint};

mod canvas;
pub use canvas::{Canvas, FONT0_H, FONT0_W};
#[cfg(feature = "mock")]
pub use canvas::{DrawOp, MockCanvas};

mod state;
pub use state::State;

mod draw;
mod widget;
pub use draw::{DrawCtx, Theme};
pub use widget::Widget;

mod render;
mod input;
pub use render::Renderer;
pub use input::{GestureState, InputSource, DOUBLE_MS, HOLD_MS};
