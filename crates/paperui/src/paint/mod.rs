//! The drawing substrate the reactive engine renders *through*: the minimal `Canvas`
//! surface trait + font metrics, the per-draw `DrawCtx`/`UpdateHint`, the `WidgetTheme`
//! look contract, colors/fonts, and the optional `MockCanvas` (host tests) and `EgCanvas`
//! (embedded-graphics) backends. Depends on no graphics library unless the `eg` feature is on.

mod color;
mod ctx;
mod theme;
pub use color::{Color, FontId};
pub use ctx::{DrawCtx, UpdateHint};
pub use theme::WidgetTheme;

#[cfg(feature = "mock")]
mod mock;
#[cfg(feature = "mock")]
pub use mock::{DrawOp, MockCanvas};

#[cfg(feature = "eg")]
mod eg;
#[cfg(feature = "eg")]
pub use eg::{to_rgb565, EgCanvas};

use crate::geometry::{Point, Rect, Size};

/// The engine's own minimal drawing surface. The core and themes depend ONLY on this
/// trait — never on a concrete graphics library. Backends provide an impl (e.g. the `eg`
/// adapter over `embedded-graphics::DrawTarget`).
pub trait Canvas {
    fn fill_rect(&mut self, r: Rect, color: Color);
    fn stroke_rect(&mut self, r: Rect, color: Color, width: u16);
    /// Draw `s` at `at`; returns the pixel size the text occupied.
    fn text(&mut self, at: Point, s: &str, font: FontId, color: Color) -> Size;
    /// Confine subsequent draws to `clip` (scissor). `None` = whole surface.
    fn set_clip(&mut self, clip: Option<Rect>);
}

/// Glyph cell size for FontId(0): 6x8 px (matches a classic 5x7+gap bitmap font).
pub const FONT0_W: i16 = 6;
pub const FONT0_H: i16 = 8;
