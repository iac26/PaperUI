#![no_std]
//! PaperUI engine core: device- and graphics-library-agnostic UI infrastructure.
//! Defines the Canvas/Widget/Theme/State traits. Contains NO concrete widgets,
//! NO concrete rendering, NO device code.

mod geometry;
pub use geometry::{Point, Rect, Size};
