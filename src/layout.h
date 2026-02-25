#pragma once

#include "widget.h"

namespace PaperUI {

constexpr uint8_t MAX_CHILDREN = 16;

class Layout : public Widget {
public:
    Layout() = default;

    Layout& add(Widget* child) {
        if (_child_count < MAX_CHILDREN) {
            _children[_child_count++] = child;
            child->setParent(this);
        }
        return *this;
    }

    uint8_t childCount() const { return _child_count; }
    Widget* child(uint8_t i) const { return _children[i]; }

    // --- Widget overrides ---

    bool isLayout() const override { return true; }

    void draw(M5GFX& gfx) override {
        if (_bg != Colors::WHITE) {
            gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, _bg);
        }
        for (uint8_t i = 0; i < _child_count; i++) {
            if (_children[i]->isVisible()) {
                _children[i]->draw(gfx);
            }
        }
    }

    // Dispatch touch to children in reverse order (topmost first)
    bool onTouch(const TouchEvent& event) override {
        for (int8_t i = _child_count - 1; i >= 0; i--) {
            if (_children[i]->isVisible() &&
                _children[i]->bounds().contains(event.x, event.y)) {
                if (_children[i]->onTouch(event)) return true;
            }
        }
        return false;
    }

    // Called by children when they become dirty
    void onChildDirty(Widget* child) {
        _dirty = true;
        if (_parent) _parent->onChildDirty(this);
    }

    // Position children within our bounds. Called after place().
    virtual void layout() = 0;

    // --- Configuration ---

    Color background() const { return _bg; }
    void setBackground(Color c) { _bg = c; markDirty(); }
    void setPadding(EdgeInsets p) { _padding = p; markDirty(); }
    void setSpacing(int16_t s) { _spacing = s; markDirty(); }

    // Fluent setters (return Layout&; subclasses provide covariant overrides)
    Layout& spacing(int16_t s) { setSpacing(s); return *this; }
    Layout& padding(EdgeInsets p) { setPadding(p); return *this; }
    Layout& padding(int16_t all) { setPadding(EdgeInsets::all(all)); return *this; }
    Layout& padding(int16_t h, int16_t v) { setPadding(EdgeInsets::symmetric(h, v)); return *this; }
    Layout& bg(Color c) { setBackground(c); return *this; }

protected:
    Widget* _children[MAX_CHILDREN] = {};
    uint8_t _child_count = 0;
    int16_t _spacing = 4;
    EdgeInsets _padding = {};
    Color _bg = Colors::WHITE;
};

} // namespace PaperUI
