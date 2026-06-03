# paperui-sim

A **host-only desktop preview** backend for PaperUI. It hosts the real reactive engine and an
existing device theme in an [`embedded-graphics-simulator`](https://crates.io/crates/embedded-graphics-simulator)
window so you can exercise a UI end-to-end — keyboard focus-nav and mouse-as-touch — without
flashing hardware. It is **not** a new theme.

## Prerequisites
- SDL2: `sudo apt-get install -y libsdl2-dev` (or your distro's equivalent).

## Run the demo
```bash
cargo run -p paperui-sim --example stickc_demo --target x86_64-unknown-linux-gnu
```
- `Tab` / `→` / `↓` — move focus (visible highlight)
- `Enter` / `Space` — activate the focused button
- mouse click — activate the button under the cursor (emulates touch)
- `Esc` / close — quit

## Use it for your own tree
```rust
use paperui::reactive::Scope;
use paperui_sim::{run_sim, SimConfig};
use paperui_tft::TftTheme;

let cx = Scope::root();
// ... build your reactive tree, get the root NodeId ...
run_sim(root, &TftTheme, SimConfig::stickc()); // or SimConfig::m5paper()
```

## Notes
- `run_sim` is generic over the theme, so it also previews `paperui_eink::EinkTheme` in grayscale.
- Reactive text currently renders placeholder black-on-white (a Layer-#1 limitation), faithfully shown here.
