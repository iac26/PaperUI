#pragma once

#include "../layout.h"

namespace PaperUI {

class Row : public Layout {
public:
    void setCrossAlign(Align a) { _cross_align = a; }
    void setMainArrangement(Arrangement a) { _arrangement = a; }

    // Fluent setters (covariant)
    Row& crossAlign(Align a) { _cross_align = a; return *this; }
    Row& arrange(Arrangement a) { _arrangement = a; return *this; }
    Row& add(Widget* child) { Layout::add(child); return *this; }
    Row& spacing(int16_t s) { setSpacing(s); return *this; }
    Row& padding(EdgeInsets p) { setPadding(p); return *this; }
    Row& padding(int16_t all) { setPadding(EdgeInsets::all(all)); return *this; }
    Row& padding(int16_t h, int16_t v) { setPadding(EdgeInsets::symmetric(h, v)); return *this; }
    Row& bg(Color c) { setBackground(c); return *this; }

    Size measure(const Constraints& c) override {
        int16_t total_w = _padding.left + _padding.right;
        int16_t max_h = 0;
        int16_t content_h = c.max_h - _padding.top - _padding.bottom;

        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;
            Constraints cc(0, 0, (int16_t)(c.max_w - total_w), content_h);
            Size cs = _children[i]->measure(cc);
            _measured[i] = cs;
            if (i > 0) total_w += _spacing;
            total_w += cs.w;
            if (cs.h > max_h) max_h = cs.h;
        }
        max_h += _padding.top + _padding.bottom;
        return Size(
            (int16_t)constrain(total_w, c.min_w, c.max_w),
            (int16_t)constrain(max_h, c.min_h, c.max_h)
        );
    }

    void layout() override {
        int16_t avail_h = _bounds.h - _padding.top - _padding.bottom;

        int16_t total_child_w = 0;
        uint8_t visible = 0;
        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;
            total_child_w += _measured[i].w;
            visible++;
        }
        int16_t total_spacing = (visible > 1) ? _spacing * (visible - 1) : 0;
        int16_t extra = _bounds.w - _padding.left - _padding.right
                        - total_child_w - total_spacing;
        if (extra < 0) extra = 0;

        int16_t cursor_x = _bounds.x + _padding.left;
        int16_t gap = _spacing;

        switch (_arrangement) {
            case Arrangement::CENTER:
                cursor_x += extra / 2;
                break;
            case Arrangement::END:
                cursor_x += extra;
                break;
            case Arrangement::SPACE_BETWEEN:
                if (visible > 1) gap = (extra + total_spacing) / (visible - 1);
                break;
            case Arrangement::SPACE_EVENLY:
                if (visible > 0) {
                    int16_t eg = (extra + total_spacing) / (visible + 1);
                    cursor_x += eg;
                    gap = eg;
                }
                break;
            default: break;
        }

        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;

            int16_t child_h = _measured[i].h;
            int16_t child_y = _bounds.y + _padding.top;

            switch (_cross_align) {
                case Align::CENTER:  child_y += (avail_h - child_h) / 2; break;
                case Align::END:     child_y += avail_h - child_h; break;
                case Align::STRETCH: child_h = avail_h; break;
                default: break;
            }

            _children[i]->place(cursor_x, child_y, _measured[i].w, child_h);

            if (_children[i]->isLayout()) {
                static_cast<Layout*>(_children[i])->layout();
            }

            cursor_x += _measured[i].w + gap;
        }
    }

private:
    Size _measured[MAX_CHILDREN];
    Align _cross_align = Align::START;
    Arrangement _arrangement = Arrangement::START;
};

} // namespace PaperUI
