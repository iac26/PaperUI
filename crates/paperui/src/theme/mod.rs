//! Default widget themes. Both implement `WidgetTheme` over any `Canvas`, so the
//! same widget tree reskins by swapping the theme — no widget-logic change.

mod eink;
mod tft;

pub use eink::EinkTheme;
pub use tft::DefaultTheme;
