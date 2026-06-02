#![no_std]
//! PaperUI widgets: pure-logic widgets + the WidgetTheme render contract.
mod button;
mod widget_theme;
pub use button::Button;
pub use widget_theme::WidgetTheme;
