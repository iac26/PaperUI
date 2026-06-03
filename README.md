# PaperUI

A small, `no_std`, allocator-free **UI framework for embedded displays**, written in
Rust for the `esp-rs` ecosystem. The engine depends on **no graphics library** by default —
it defines its own minimal `Canvas` trait; the `embedded-graphics` adapter and the device
drivers are opt-in (a feature, and separate board crates).

> Rewritten from the original C++/Arduino/M5Unified library. The C++ version is archived at
> the `v0-cpp` tag.

## Crates

| Crate | Responsibility | Target |
|-------|----------------|--------|
| `paperui` | The framework: `Canvas`/`Widget`/`Theme`/`Renderer`/`InputSource` traits, `State`, geometry, button gestures, the logic widgets (`Button`), and the default themes (`DefaultTheme`, `EinkTheme`). | host-testable |
| `paperui-tft` | M5StickC Plus2 backend: GPIO button-gesture input (esp-hal). | ESP32 / Xtensa |
| `paperui-eink` | M5Paper backend: self-authored IT8951 e-ink driver + sleep/wake `Renderer` + GT911 touch (esp-hal). | ESP32 / Xtensa |

`paperui` features:
- **`eg`** — the `Canvas` → `embedded-graphics` `DrawTarget` adapter (`EgCanvas`). Pulls
  `embedded-graphics`; off by default so the engine stays graphics-library-free.
- **`mock`** — a recording mock `Canvas` for host tests.

The two board crates are kept separate (not features) on purpose: they pull `esp-hal` and
mutually-exclusive display drivers, so an app only ever compiles the board it names, and the
`paperui` crate never drags in `esp-hal`.

**Principles:** logic ⟂ rendering (widgets are logic; themes render); static memory only (no
`alloc`); device backends own panel presentation + power lifecycle. The same widget tree
reskins across color-TFT and e-ink by swapping the theme — no widget-logic change.

## Building

```bash
source ~/export-esp.sh   # esp toolchain (espup)

# Host (engine + widgets + themes + the eg adapter):
cargo test -p paperui --features mock,eg --target x86_64-unknown-linux-gnu

# Device backends (ESP32 / Xtensa):
cargo +esp build -p paperui-tft -p paperui-eink \
  -Zbuild-std=core --target xtensa-esp32-none-elf
```

## Example consumer

The [`electrolux-remote`](https://github.com/iac26/electrolux-remote) app uses PaperUI
(`paperui` + `eg` feature + `paperui-tft`) to build an M5StickC Plus2 IR remote that turns
off an Electrolux AC.
