# PaperUI

Declarative UI framework for M5Paper (ESP32 + IT8951 e-ink), built on M5Unified/M5GFX.

## Quick Start

```cpp
#include <M5Unified.h>
#include <PaperUI.h>
using namespace PaperUI;

Screen screen;
State<float> counter(0);

static void onIncrement(void*) { counter.set(counter.get() + 1); }

void setup() {
    auto cfg = M5.config();
    M5.begin(cfg);
    screen.begin();

    auto& root = ui::col(6,
        ui::text("Hello PaperUI", 3),
        ui::row(Arrangement::START, Align::CENTER, 8,
            ui::text("Count:"),
            ui::value("%d").bind(counter)
        ),
        ui::button("+1").onClick(onIncrement)
    );
    root.padding(20);
    root.crossAlign(Align::STRETCH);
    screen.root(root);
}

void loop() {
    M5.update();
    screen.update();
    delay(50);
}
```

## Design Principles

1. **Static allocation only** -- no `new`/`delete`, no heap fragmentation. All widgets live in fixed-size `StaticPool<T, N>` arrays. Pool sizes are configurable via `#define` before including `<PaperUI.h>`.

2. **Compose-like API** -- factory functions (`ui::text()`, `ui::col()`, `ui::button()`, etc.) allocate from pools and return references. Build the widget tree declaratively in `setup()`.

3. **Reactive state** -- `State<T>` tracks changes via generation counters. Widgets bound to state auto-update via `sync()`. The screen only walks the tree when `StateBase::global_gen()` changes.

4. **E-ink aware** -- each widget declares an `UpdateHint` so the screen picks the appropriate EPD refresh mode per dirty region. No full-screen refreshes unless explicitly requested.

5. **Direct drawing** -- renders to `M5.Display` (M5GFX) without a full-screen sprite buffer. The IT8951 controller has its own framebuffer. Partial refresh via `display(x, y, w, h)`.

## Architecture

### Layout Pipeline

`Screen::performLayout()` runs the full pipeline:

```
measure(constraints)  -->  Constraints flow DOWN the tree
place(x, y, w, h)    -->  Positions flow DOWN
layout()              -->  Children positioned within parent
draw(gfx)            -->  Paint to M5GFX
display()             -->  Push to e-ink (epd_quality)
```

Incremental updates via `Screen::update()`:
1. Sync all state bindings
2. Process touch/button input
3. Collect dirty leaf widget rects
4. Merge if fragmented (>4 rects)
5. Clear & redraw overlapping regions
6. Partial e-ink refresh per dirty rect with appropriate EPD mode

### E-ink Update Modes

Each widget declares an `UpdateHint`. The screen picks the slowest (highest quality) hint among dirty widgets in a region:

| UpdateHint | EPD Mode | Grays | Use |
|------------|----------|-------|-----|
| `NONE` | -- | -- | Invisible widgets (spacers) |
| `MONO` | `epd_fastest` | 2 (B/W) | Buttons, switches, keyboard |
| `FAST` | `epd_fast` | 4 | Sliders, progress bars, battery |
| `TEXT` | `epd_text` | 16 | Text, text areas |
| `QUALITY` | `epd_quality` | 16 | Value changes, toggles (flashes) |

### Dirty Tracking

Calling `markDirty()` on a widget bubbles up to its parent layout via `onChildDirty()`. The screen collects dirty leaf widgets' bounds on each `update()` cycle. Only changed regions are redrawn and pushed to the e-ink display.

## Widgets

### TextWidget

Static text label.

```cpp
auto& t = ui::text("Hello", 3);  // text, fontSize
t.color(Colors::GRAY_MID);
t.bgColor(Colors::WHITE);

// Bind to reactive state
State<const char*> label("Ready");
ui::text("").bind(label);
```

### ValueWidget

Formatted numeric display. Extends `TextWidget`.

```cpp
State<float> temp(23.5);
auto& v = ui::value("%.1f C").bind(temp);
v.minChars(8);  // stable width (won't shrink/grow)
```

Format detection: `%d`/`%i`/`%u` → integer cast, `%f`/`%e` → float.

### ButtonWidget

Tappable button with press state.

```cpp
auto& b = ui::button("Click me")
    .onClick(myCallback, optionalUserData)
    .padding(16, 8)    // horizontal, vertical
    .radius(8);        // corner radius
```

Callback signature: `void (*)(void* user_data)`.

### SwitchWidget

Toggle switch with track and thumb.

```cpp
State<bool> enabled(false);
auto& sw = ui::toggle()
    .bind(enabled)      // two-way binding
    .onChange(onChanged);
```

Callback: `void (*)(void* user_data, int32_t new_value)` where value is 0 or 1.

### SliderWidget

Horizontal draggable slider.

```cpp
State<float> volume(50);
auto& s = ui::slider(0, 100)
    .bind(volume)
    .onChange(onSliderChange);
```

### CheckboxWidget

Checkbox with text label. Two-way binding to `State<bool>`.

```cpp
State<bool> agreed(false);
auto& cb = ui::checkbox("I agree")
    .bind(agreed)
    .onChange(onCheckChange);
```

### ProgressBarWidget

Horizontal progress bar.

```cpp
State<float> progress(0);
auto& p = ui::progress(0, 100).bind(progress);
```

### KeyboardWidget

Full on-screen QWERTY keyboard. Monolithic widget (draws its own key grid, no child ButtonWidgets).

```
Row 0: Q W E R T Y U I O P
Row 1: A S D F G H J K L <-
Row 2: Z X C V B N M [spc] CLR
Row 3: [________SPACE________]
Row 4: 1 2 3 4 5 6 7 8 9 0
```

```cpp
auto& kb = ui::keyboard().onKey(myKeyHandler, userData);
```

Callback: `void (*)(void* user_data, char key)` where key is:
- `'A'-'Z'`, `'0'-'9'`, `' '` for regular keys
- `'\b'` for backspace
- `'\0'` for clear

### TextAreaWidget

Fixed-buffer (256 char) text display with word wrapping and cursor.

```cpp
auto& ta = ui::textArea().height(140).fontSize(2);
ta.appendChar('H');
ta.deleteChar();
ta.clear();
```

Typically wired to a KeyboardWidget:

```cpp
static void onKey(void* ud, char key) {
    auto* ta = static_cast<TextAreaWidget*>(ud);
    if (key == '\b')      ta->deleteChar();
    else if (key == '\0') ta->clear();
    else                  ta->appendChar(key);
}
ui::keyboard().onKey(onKey, &ta);
```

### BatteryWidget

Battery icon with voltage text. Displays a graphical battery (fill proportional to 3.5V-4.2V range) plus voltage reading.

```cpp
State<float> bat_mv(0);
auto& bat = ui::battery().bind(bat_mv);

// In loop:
bat_mv.set((float)M5.Power.getBatteryVoltage());
```

## Layouts

### Column

Vertical layout. Children stacked top-to-bottom.

```cpp
auto& c = ui::col(6,         // spacing between children
    ui::text("Title", 3),
    ui::button("OK")
);
c.crossAlign(Align::STRETCH); // STRETCH, START, CENTER, END
c.arrange(Arrangement::START); // START, CENTER, END, SPACE_BETWEEN, SPACE_EVENLY
c.padding(12);
```

### Row

Horizontal layout. Children laid out left-to-right.

```cpp
auto& r = ui::row(Arrangement::SPACE_BETWEEN, Align::CENTER, 8,
    ui::text("Label:"),
    ui::value("%.1f").bind(state)
);
```

### Stack

Overlapping children. All children occupy the same bounds. Use `setVisible()` to show one at a time (tab system).

```cpp
auto& s = ui::stack(page1, page2, page3);
page2.setVisible(false);
page3.setVisible(false);
```

### Spacer

Invisible fixed-size widget for spacing control.

```cpp
ui::spacer(0, 8);   // width=0, height=8 (vertical gap)
ui::spacer(16, 0);  // horizontal gap
```

## State Binding

`State<T>` is a lightweight reactive container with generation tracking.

```cpp
State<float> temperature(22.5);

// Read
float t = temperature.get();

// Write (increments generation counter, triggers widget sync)
temperature.set(23.0);

// Bind widgets
ui::value("%.1f").bind(temperature);     // one-way: state -> widget
ui::slider(0, 50).bind(temperature);     // two-way: state <-> widget
ui::toggle().bind(boolState);            // two-way
ui::checkbox("Label").bind(boolState);   // two-way
```

Multiple widgets can bind to the same state. Changes propagate automatically.

## Pool Configuration

Widgets are allocated from fixed-size pools. Override pool sizes before including PaperUI:

```cpp
#define PAPERUI_POOL_TEXT     20   // default: 12
#define PAPERUI_POOL_VALUE     6   // default: 4
#define PAPERUI_POOL_BUTTON    8   // default: 4
#define PAPERUI_POOL_SWITCH    2   // default: 2
#define PAPERUI_POOL_SLIDER    2   // default: 2
#define PAPERUI_POOL_CHECKBOX  2   // default: 2
#define PAPERUI_POOL_PROGRESS  4   // default: 2
#define PAPERUI_POOL_COLUMN   12   // default: 8
#define PAPERUI_POOL_ROW      10   // default: 8
#define PAPERUI_POOL_STACK     2   // default: 2
#define PAPERUI_POOL_SPACER   12   // default: 4
#define PAPERUI_POOL_KEYBOARD  1   // default: 1
#define PAPERUI_POOL_TEXTAREA  1   // default: 1
#define PAPERUI_POOL_BATTERY   1   // default: 1

#include <PaperUI.h>
```

If a pool is exhausted, `alloc()` reuses the last slot (better than crashing, but produces incorrect behavior). Size your pools for your UI.

Maximum children per layout: `MAX_CHILDREN = 16`.

## Caveats

### E-ink Specific

- **No animation**: e-ink refresh takes 100-1000ms. Design for static layouts with incremental updates.
- **Ghosting**: partial refreshes accumulate ghosting. The screen does a full refresh every 10 partial updates by default (`setFullRefreshInterval(n)`, 0 to disable).
- **QUALITY mode flashes**: `epd_quality` causes a full black-white-black flash. Avoid `ValueWidget` in rapidly-changing scenarios or accept the flash.
- **No sprite buffer**: drawing goes directly to M5GFX. If two widgets overlap, the second draw wins. Layouts prevent overlap for sibling widgets, but Stack children can overlap intentionally.

### Memory

- All pools are statically allocated at startup. A `StaticPool<Widget, 20>` allocates 20 Widget-sized objects in `.bss` regardless of how many you use.
- The ESP32 has 4MB of PSRAM, but pools are in regular SRAM (~320KB). Keep total pool sizes reasonable.
- `State<T>` objects are also static/global. No heap allocation anywhere.

### Touch

- Touch events are debounced (80ms cooldown after release). Rapid tapping may miss events.
- The GT911 touch controller supports multi-touch, but PaperUI only processes the first touch point.
- Touch dispatch goes to children in reverse order (last-added = topmost, checked first).

### Callbacks

- All callbacks are plain function pointers with `void* user_data`. No `std::function`, no lambdas with captures.
- Callbacks fire synchronously during `screen.update()`. Keep them fast -- don't do I/O or blocking work.

### Layout

- `measure()` is called once per layout pass. Widget sizes are cached in `_measured[]` arrays within layouts.
- Calling `screen.performLayout()` recomputes the entire tree and does a full e-ink refresh. Use it for structural changes (visibility toggling, tab switching). Don't call it every frame.
- `screen.update()` handles incremental dirty-rect updates efficiently. This is what you call in `loop()`.

## Extending: Creating a Custom Widget

### 1. Create the Header

```
lib/PaperUI/src/widgets/my_widget.h
```

```cpp
#pragma once
#include "../widget.h"
#include "../state.h"  // if you need state binding

namespace PaperUI {

class MyWidget : public Widget {
public:
    // Fluent setters
    MyWidget& someProperty(int v) { _prop = v; markDirty(); return *this; }

    Size measure(const Constraints& c) override {
        // Return your desired size, clamped to constraints
        int16_t w = /* your width */;
        int16_t h = /* your height */;
        return Size(
            (int16_t)constrain(w, c.min_w, c.max_w),
            (int16_t)constrain(h, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        // Draw within _bounds (set by place())
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, Colors::WHITE);
        // ... your drawing code ...
    }

    // Optional: handle touch
    bool onTouch(const TouchEvent& event) override {
        if (!_bounds.contains(event.x, event.y)) return false;
        // Handle DOWN/MOVE/UP
        if (event.action == TouchAction::UP) {
            // Do something
            markDirty();  // request redraw
        }
        return true;  // consumed
    }

    // Choose appropriate e-ink mode
    UpdateHint updateHint() const override { return UpdateHint::FAST; }

    // Optional: bind to State<T>
    MyWidget& bind(State<float>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            // Update from state
        }
    }

private:
    int _prop = 0;
    State<float>* _bound = nullptr;
    uint32_t _last_gen = 0;
};

} // namespace PaperUI
```

### 2. Register the Pool

In `lib/PaperUI/src/ui.h`:

1. Add `#include "widgets/my_widget.h"` at the top
2. Add pool size default:
   ```cpp
   #ifndef PAPERUI_POOL_MYWIDGET
   #define PAPERUI_POOL_MYWIDGET 2
   #endif
   ```
3. Add pool member to `struct Pools`:
   ```cpp
   StaticPool<MyWidget, PAPERUI_POOL_MYWIDGET> myWidgets;
   ```
4. Add factory function:
   ```cpp
   inline MyWidget& myWidget() {
       return pools().myWidgets.alloc();
   }
   ```
5. Add to `reset()`:
   ```cpp
   pools().myWidgets.reset();
   ```

### 3. Add Include to Entry Point

In `lib/PaperUI/PaperUI.h`, add:
```cpp
#include "src/widgets/my_widget.h"
```

### Key Rules

- Call `markDirty()` whenever visual state changes. This is how the screen knows to redraw.
- Draw only within `_bounds`. The bounds are set by the layout system via `place()`.
- Use `Colors::WHITE` as the default background. The screen clears dirty regions to white before redrawing.
- Keep `draw()` fast. It runs on the main thread during `screen.update()`.
- For touch-interactive widgets, return `true` from `onTouch()` to consume the event (prevents it reaching widgets underneath).
- Character dimensions at text size N: width = `6*N` pixels, height = `8*N` pixels. This is the M5GFX default font.

## File Structure

```
lib/PaperUI/
  PaperUI.h                          # Single include entry point
  library.json                       # PlatformIO library manifest
  src/
    types.h                          # Color, Rect, Constraints, Size, enums, callback types
    pool.h                           # StaticPool<T, N> fixed-size allocator
    state.h                          # State<T> reactive value with generation counter
    widget.h                         # Base Widget class (measure/place/draw/onTouch)
    widget.cpp                       # Widget::markDirty() implementation
    layout.h                         # Base Layout class (children, draw, touch dispatch)
    screen.h                         # Screen manager (layout, dirty rects, touch, buttons)
    ui.h                             # Factory functions and pool definitions
    widgets/
      text_widget.h                  # Static text
      value_widget.h                 # Formatted numeric value (extends TextWidget)
      button_widget.h                # Tappable button with press state
      switch_widget.h                # Toggle switch
      slider_widget.h                # Draggable slider
      checkbox_widget.h              # Checkbox with label
      progress_bar_widget.h          # Progress bar
      keyboard_widget.h              # On-screen QWERTY keyboard
      text_area_widget.h             # Multi-line text display with cursor
      battery_widget.h               # Battery icon with voltage
    layouts/
      column.h                       # Vertical layout
      row.h                          # Horizontal layout
      stack.h                        # Overlapping layout (for tabs)
      spacer.h                       # Invisible fixed-size spacer
```

## Dependencies

- [M5Unified](https://github.com/m5stack/M5Unified) (0.2.x) -- hardware abstraction
- [M5GFX](https://github.com/m5stack/M5GFX) -- graphics (pulled in by M5Unified)
- PlatformIO `espressif32` platform (6.x)
- Standard Arduino framework (**not** the custom M5Stack framework zip)
