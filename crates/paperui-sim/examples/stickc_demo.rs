//! End-to-end desktop preview of a StickC-style UI: `TftTheme` renders a reactive counter.
//! Tab moves the focus highlight; Enter (or a mouse click on `+`) increments the count; only
//! the count region repaints. Run with:
//!   cargo run -p paperui-sim --example stickc_demo --target x86_64-unknown-linux-gnu

use paperui::reactive::{button, col, text, Scope};
use paperui_sim::{run_sim, SimConfig};
use paperui_tft::TftTheme;

fn main() {
    let cx = Scope::root();
    let count = cx.signal(0i32);

    let label = text(cx, move || {
        let mut s = heapless::String::<32>::new();
        let _ = core::fmt::write(&mut s, format_args!("Count: {}", count.get()));
        s
    });
    let inc = button(cx, "+", move || count.update(|c| *c += 1));
    let root = col(cx, (label, inc));

    run_sim(root, &TftTheme, SimConfig::stickc());
}
