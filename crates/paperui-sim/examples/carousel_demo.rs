//! Desktop preview of the carousel + crude slide animation. `TftTheme` renders a vertical
//! carousel of rooms; the selection is always centered. Tab/Down/Right moves the selection
//! down, Up moves it up — each move slides the list one row (the crude frame-stepped
//! animation), wrapping at both ends. Run with:
//!   cargo run -p paperui-sim --example carousel_demo --target x86_64-unknown-linux-gnu

use paperui::reactive::{button, carousel, carousel_select_first, col, install, text_static, Scope, Storage};
use paperui_sim::{run_sim, SimConfig};
use paperui_tft::TftTheme;
use static_cell::StaticCell;

fn main() {
    // 9 nodes (6 buttons + carousel + title + col), 6 effects (button handlers), 1 owner;
    // signals:1 is harmless headroom (this demo has none).
    static STORAGE: StaticCell<Storage<1, 9, 6, 1>> = StaticCell::new();
    install(STORAGE.init(Storage::new()));
    let cx = Scope::root();

    // A carousel of selectable rooms. Empty handlers — this demo is about navigation + the slide.
    let items = [
        button(cx, "Living Room", || {}),
        button(cx, "Kitchen", || {}),
        button(cx, "Bedroom", || {}),
        button(cx, "Office", || {}),
        button(cx, "Garage", || {}),
        button(cx, "Garden", || {}),
    ];
    let rooms = carousel(cx, &items);

    // Root is a column: a fixed title on top, the carousel below. The animation pass repaints the
    // title as an overlay mask each frame, so it stays crisp while the list slides.
    let root = col(cx, (text_static(cx, "Rooms"), rooms));

    // Center the first item and focus it before the first paint.
    carousel_select_first(rooms);

    run_sim(root, &TftTheme, SimConfig::stickc());
}
