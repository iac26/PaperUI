//! PaperUI desktop **preview** backend.
//!
//! Hosts the real PaperUI reactive engine + an existing device theme (e.g.
//! `paperui_tft::TftTheme`) in an `embedded-graphics-simulator` window, driven by keyboard
//! (focus nav) and mouse (touch). It is a host-only test/preview tool — not a new theme.
//!
//! Run the demo:
//! `cargo run -p paperui-sim --example stickc_demo --target x86_64-unknown-linux-gnu`
