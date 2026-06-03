# paperui-eink

M5Paper (ESP32 + IT8951 e-ink + GT911 touch) backend for PaperUI. A self-authored,
`no_std`, alloc-free IT8951 SPI driver with the **sleep/wake power lifecycle** wired
through `EinkRenderer` (the engine wakes the panel in `before_draw` and sleeps it in
`after_draw`), a 4bpp `Gray4Canvas`, an `UpdateHint`→waveform mapping, and GT911 touch.

## Build
```bash
source ~/export-esp.sh
cargo +esp build -p paperui-eink -Zbuild-std=core --target xtensa-esp32-none-elf
```

## Verified vs. needs-hardware
- **Verified (Xtensa compile):** the crate type-checks and links against real esp-hal
  1.1 SPI/I2C/GPIO + `gt911` 0.3 APIs; the engine `Renderer` lifecycle is wired
  (`before_draw`→wake, `present`→load+display, `after_draw`→sleep).
- **NEEDS HARDWARE (spec risk #1 — do these on an M5Paper):**
  1. **IT8951 register/command constants + SPI preamble timing** — confirm against your
     panel's datasheet/firmware; the values here are from the public app note.
  2. **Sleep current** — measure idle draw after `after_draw()` sleeps the IT8951; the
     whole point is the ~100–200 mA → near-0 drop. If it doesn't drop, the `CMD_SLEEP`
     path / power-enable rail need adjustment.
  3. **Image-load address (`target_addr`)** — read it from the IT8951 `get_device_info`
     on the real panel instead of hardcoding.
  4. **`Gray4Canvas::text()`** currently fills glyph cells (correct metrics + ink, not
     letterforms). Replace with an `embedded-graphics` `MonoFont` rasterizer onto the gray
     buffer for readable text.
  5. **Windowed framebuffer** — `Gray4Canvas` is a 320×240 window (a full 960×540
     one-byte buffer overflows ESP32 internal RAM); a larger panel area needs a PSRAM
     framebuffer. `present()` itself splits any region into ≤16384-px horizontal bands
     internally (one `load_image_area` per band, then a single `display_area`), so it no
     longer drops pixels — but the canvas window still bounds how much you can draw.
  6. **GT911 wiring** — I2C address (default 0x5D), INT/RST pins, and coordinate
     orientation vs. the panel.

## M5Paper pin map (VERIFY against the schematic)
IT8951: SPI (SCK/MOSI/MISO) + CS + HRDY (host-ready, input) + RST; plus a panel
power-enable GPIO (drive it before `init`). GT911: I2C (SDA/SCL) + INT + RST. Fill these
in from your board's schematic during bring-up.

## How it fits the engine
`EinkRenderer` is the concrete `paperui::Renderer` for e-ink: the same widget tree +
the same engine loop that drives the StickC TFT also drives the M5Paper — only the renderer
(power lifecycle + waveform) and theme (`EinkTheme`) differ. Swapping `DefaultTheme`↔
`EinkTheme` reskins with zero widget-logic change.
