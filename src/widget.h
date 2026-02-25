#pragma once

#include "types.h"

namespace PaperUI {

class Layout; // forward declare

class Widget {
public:
    Widget() = default;
    virtual ~Widget() = default;

    // --- Layout protocol ---

    // Determine desired size given parent constraints.
    virtual Size measure(const Constraints& constraints) = 0;

    // Assign final absolute position. Marks dirty if bounds changed.
    void place(int16_t x, int16_t y, int16_t w, int16_t h) {
        Rect nb(x, y, w, h);
        if (nb != _bounds) {
            markDirty();
            _bounds = nb;
            markDirty();
        }
    }

    // --- Rendering ---

    // Draw this widget into the display at its _bounds position.
    virtual void draw(M5GFX& gfx) = 0;

    // What e-ink update mode this widget prefers.
    virtual UpdateHint updateHint() const { return UpdateHint::FAST; }

    // --- Dirty tracking ---

    bool isDirty() const { return _dirty; }
    void markDirty();
    void clearDirty() { _dirty = false; }

    // --- State binding ---

    // Pull new value from bound State (if any). Called by Screen::syncAll().
    virtual void sync() {}

    // --- Touch/input ---

    // Return true if this widget consumed the event.
    virtual bool onTouch(const TouchEvent& event) { return false; }

    // --- Hierarchy ---

    virtual bool isLayout() const { return false; }

    const Rect& bounds() const { return _bounds; }

    bool isVisible() const { return _visible; }
    void setVisible(bool v) {
        if (_visible != v) { _visible = v; markDirty(); }
    }

    void setParent(Layout* p) { _parent = p; }
    Layout* parent() const { return _parent; }

    // Optional user-assigned ID for widget lookup
    uint16_t id = 0;

protected:
    Rect _bounds = {};
    Layout* _parent = nullptr;
    bool _dirty = true;   // starts dirty so first frame draws everything
    bool _visible = true;
};

} // namespace PaperUI
