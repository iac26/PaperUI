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
| `paperui` | The framework plumbing: the `Canvas`/`Widget`/`Theme`/`WidgetTheme`/`Renderer`/`InputSource` traits, `State`, geometry, button gestures, and the logic widgets (`Button`). Ships the `WidgetTheme` **contract**, not a concrete look. | host-testable |
| `paperui-tft` | M5StickC Plus2 addon: the full-color `TftTheme` (host-testable) + GPIO button-gesture input (`ButtonReader`, esp-hal, behind `hal`). | host + ESP32 |
| `paperui-eink` | M5Paper addon: the `EinkTheme` (host-testable) + a self-authored IT8951 e-ink driver + sleep/wake `Renderer` + GT911 touch (esp-hal, behind `hal`). | host + ESP32 |

`paperui` features:
- **`eg`** — the `Canvas` → `embedded-graphics` `DrawTarget` adapter (`EgCanvas`). Pulls
  `embedded-graphics`; off by default so the engine stays graphics-library-free.
- **`mock`** — a recording mock `Canvas` for host tests.

**Plumbing vs. rendering.** `paperui` is the declarative plumbing: the engine, the widget
*logic*, and the trait contracts — including `WidgetTheme`, but **no concrete colors**. Each
board **addon** crate provides the rendering for its panel: the driver *and* that board's
theme (`paperui-tft::TftTheme`, `paperui-eink::EinkTheme`). A theme is pure logic over the
abstract `Canvas`, so it is host-testable; the esp-hal driver sits behind each addon's
default **`hal`** feature, so `--no-default-features` builds and tests the theme on host.

The board crates are separate (not features of `paperui`) on purpose: they pull `esp-hal` and
mutually-exclusive display drivers, so an app only ever compiles the board it names, and the
`paperui` crate never drags in `esp-hal`.

**Principles:** logic ⟂ rendering (widgets are logic; themes + drivers render); static memory
only (no `alloc`); board addons own the look + panel presentation + power lifecycle. The same
widget tree reskins by swapping the theme — no widget-logic change. Anyone can ship their own
look by implementing `WidgetTheme` over a `Canvas`.

## Building

```bash
source ~/export-esp.sh   # esp toolchain (espup)

# Host (engine + widgets + the eg adapter):
cargo test -p paperui --features mock,eg --target x86_64-unknown-linux-gnu

# Host (each board addon's theme — no esp-hal):
cargo test -p paperui-tft  --no-default-features --features mock --target x86_64-unknown-linux-gnu
cargo test -p paperui-eink --no-default-features --features mock --target x86_64-unknown-linux-gnu

# Device backends, full addon incl. drivers (ESP32 / Xtensa):
cargo +esp build -p paperui-tft -p paperui-eink \
  -Zbuild-std=core --target xtensa-esp32-none-elf
```

## Example consumer

The [`electrolux-remote`](https://github.com/iac26/electrolux-remote) app uses PaperUI
(`paperui` + `eg` feature + `paperui-tft`) to build an M5StickC Plus2 IR remote that turns
off an Electrolux AC.
