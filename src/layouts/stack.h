#pragma once

#include "../layout.h"

namespace PaperUI {

class Stack : public Layout {
public:
    // Fluent setters (covariant)
    Stack& add(Widget* child) { Layout::add(child); return *this; }
    Stack& spacing(int16_t s) { setSpacing(s); return *this; }
    Stack& padding(EdgeInsets p) { setPadding(p); return *this; }
    Stack& padding(int16_t all) { setPadding(EdgeInsets::all(all)); return *this; }
    Stack& padding(int16_t h, int16_t v) { setPadding(EdgeInsets::symmetric(h, v)); return *this; }
    Stack& bg(Color c) { setBackground(c); return *this; }

    Size measure(const Constraints& c) override {
        int16_t max_w = 0, max_h = 0;
        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;
            Size s = _children[i]->measure(c);
            _measured[i] = s;
            if (s.w > max_w) max_w = s.w;
            if (s.h > max_h) max_h = s.h;
        }
        return Size(
            (int16_t)constrain(max_w + _padding.left + _padding.right,
                               c.min_w, c.max_w),
            (int16_t)constrain(max_h + _padding.top + _padding.bottom,
                               c.min_h, c.max_h)
        );
    }

    void layout() override {
        int16_t cx = _bounds.x + _padding.left;
        int16_t cy = _bounds.y + _padding.top;
        int16_t cw = _bounds.w - _padding.left - _padding.right;
        int16_t ch = _bounds.h - _padding.top - _padding.bottom;

        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;
            _children[i]->place(cx, cy, cw, ch);
            if (_children[i]->isLayout()) {
                static_cast<Layout*>(_children[i])->layout();
            }
        }
    }

private:
    Size _measured[MAX_CHILDREN];
};

} // namespace PaperUI
