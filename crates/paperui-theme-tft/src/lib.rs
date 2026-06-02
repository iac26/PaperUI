#![no_std]
//! PaperUI default (color/TFT) theme: implements WidgetTheme to render widgets.
//! Depends only on core + widgets — NO device dependencies (host-testable).

mod default_theme;
pub use default_theme::DefaultTheme;
