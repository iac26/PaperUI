#pragma once

#include <M5Unified.h>
#include <stdint.h>

namespace PaperUI {

// RGB888 color (M5GFX native)
using Color = uint32_t;

namespace Colors {
    constexpr Color WHITE      = 0xFFFFFFU;
    constexpr Color GRAY_LIGHT = 0xC0C0C0U;
    constexpr Color GRAY_MID   = 0x808080U;
    constexpr Color GRAY_DARK  = 0x404040U;
    constexpr Color BLACK      = 0x000000U;
}

// Axis-aligned bounding rectangle
struct Rect {
    int16_t x, y, w, h;

    Rect() : x(0), y(0), w(0), h(0) {}
    Rect(int16_t x, int16_t y, int16_t w, int16_t h) : x(x), y(y), w(w), h(h) {}

    bool contains(int16_t px, int16_t py) const {
        return px >= x && px < x + w && py >= y && py < y + h;
    }

    bool intersects(const Rect& o) const {
        return !(x + w <= o.x || o.x + o.w <= x ||
                 y + h <= o.y || o.y + o.h <= y);
    }

    Rect unite(const Rect& o) const {
        if (w == 0 && h == 0) return o;
        if (o.w == 0 && o.h == 0) return *this;
        int16_t nx = min(x, o.x);
        int16_t ny = min(y, o.y);
        int16_t nr = max((int16_t)(x + w), (int16_t)(o.x + o.w));
        int16_t nb = max((int16_t)(y + h), (int16_t)(o.y + o.h));
        return Rect(nx, ny, (int16_t)(nr - nx), (int16_t)(nb - ny));
    }

    bool isEmpty() const { return w == 0 && h == 0; }

    bool operator==(const Rect& o) const {
        return x == o.x && y == o.y && w == o.w && h == o.h;
    }

    bool operator!=(const Rect& o) const { return !(*this == o); }
};

// Size constraint passed during layout measure pass
struct Constraints {
    int16_t min_w, min_h, max_w, max_h;

    Constraints() : min_w(0), min_h(0), max_w(0), max_h(0) {}
    Constraints(int16_t minw, int16_t minh, int16_t maxw, int16_t maxh)
        : min_w(minw), min_h(minh), max_w(maxw), max_h(maxh) {}
};

// Computed size returned from measure()
struct Size {
    int16_t w, h;

    Size() : w(0), h(0) {}
    Size(int16_t w, int16_t h) : w(w), h(h) {}
};

// Cross-axis alignment within a layout cell
enum class Align : uint8_t {
    START,
    CENTER,
    END,
    STRETCH
};

// Main-axis arrangement for layouts
enum class Arrangement : uint8_t {
    START,
    CENTER,
    END,
    SPACE_BETWEEN,
    SPACE_EVENLY
};

// Padding/margin specification
struct EdgeInsets {
    int16_t left, top, right, bottom;

    EdgeInsets() : left(0), top(0), right(0), bottom(0) {}
    EdgeInsets(int16_t l, int16_t t, int16_t r, int16_t b)
        : left(l), top(t), right(r), bottom(b) {}

    static EdgeInsets all(int16_t v) { return EdgeInsets(v, v, v, v); }
    static EdgeInsets symmetric(int16_t h, int16_t v) { return EdgeInsets(h, v, h, v); }
    static EdgeInsets horizontal(int16_t h) { return EdgeInsets(h, 0, h, 0); }
    static EdgeInsets vertical(int16_t v) { return EdgeInsets(0, v, 0, v); }
};

// Touch event
enum class TouchAction : uint8_t {
    DOWN,
    MOVE,
    UP
};

struct TouchEvent {
    int16_t x, y;
    TouchAction action;

    TouchEvent() : x(0), y(0), action(TouchAction::DOWN) {}
};

// E-ink update mode hint per widget (ordered by quality: higher = slower but better)
enum class UpdateHint : uint8_t {
    NONE = 0,
    MONO = 1,       // DU  -- black/white, fastest
    FAST = 2,       // DU4 -- 4 gray levels, fast
    TEXT = 3,       // GL16 -- 16 grays, optimized for text
    QUALITY = 4     // GC16 -- 16 grays, best quality, flashes
};

// Callback types (function pointers -- no heap allocation)
using OnClickCallback  = void (*)(void* user_data);
using OnChangeCallback = void (*)(void* user_data, int32_t new_value);

} // namespace PaperUI
