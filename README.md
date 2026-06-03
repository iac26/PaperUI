# PaperUI

A small, `no_std`, allocator-free **UI framework for embedded displays**, written in
Rust for the `esp-rs` ecosystem. The engine core depends on **no graphics library** — it
defines its own minimal `Canvas` trait; concrete graphics (`embedded-graphics`) and device
drivers live only in optional adapter/backend crates.

> Rewritten from the original C++/Arduino/M5Unified library. The C++ version is archived at
> the `v0-cpp` tag.

## Architecture (layered workspace)

| Crate | Layer | Responsibility | Host-testable |
|-------|-------|----------------|:---:|
| `paperui-core` | L1 | `Canvas`/`Widget`/`Theme`/`Renderer`/`InputSource` traits, `State`, geometry, gestures. No graphics dep, no widgets. | ✅ |
| `paperui-widgets` | L2 | Pure-logic widgets (`Button`) + the `WidgetTheme` render contract. | ✅ |
| `paperui-theme-tft` | L3 | `DefaultTheme` — color look. | ✅ |
| `paperui-theme-eink` | L3 | `EinkTheme` — high-contrast mono look + e-ink update hints. | ✅ |
| `paperui-eg` | L4 | `Canvas` → `embedded-graphics` `DrawTarget` adapter. | ✅ |
| `paperui-tft` | L4 | M5StickC Plus2 backend: GPIO button-gesture input (esp-hal). | compile-only |
| `paperui-eink` | L4 | M5Paper backend: IT8951 e-ink driver + sleep/wake `Renderer` + GT911 touch (esp-hal). | compile-only |

**Principles:** logic ⟂ rendering (widgets are logic; themes render); static memory only (no
`alloc`); device backends own panel presentation + power lifecycle. The same widget tree
reskins across color-TFT and e-ink by swapping the theme — no widget-logic change.

## Building

```bash
source ~/export-esp.sh   # esp toolchain (espup)

# Host-testable layers (L1–L4 adapter):
cargo test --workspace \
  --exclude paperui-tft --exclude paperui-eink \
  --target x86_64-unknown-linux-gnu \
  --features paperui-core/mock,paperui-theme-tft/mock,paperui-theme-eink/mock

# Device backends (ESP32 / Xtensa):
cargo +esp build -p paperui-tft -p paperui-eink \
  -Zbuild-std=core --target xtensa-esp32-none-elf
```

## Example consumer

The `electrolux-remote` app uses PaperUI to build an M5StickC Plus2 IR remote that turns
off an Electrolux AC.
