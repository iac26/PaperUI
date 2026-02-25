#pragma once

#include "../layout.h"

namespace PaperUI {

class Column : public Layout {
public:
    void setCrossAlign(Align a) { _cross_align = a; }
    void setMainArrangement(Arrangement a) { _arrangement = a; }

    // Fluent setters (covariant)
    Column& crossAlign(Align a) { _cross_align = a; return *this; }
    Column& arrange(Arrangement a) { _arrangement = a; return *this; }
    Column& add(Widget* child) { Layout::add(child); return *this; }
    Column& spacing(int16_t s) { setSpacing(s); return *this; }
    Column& padding(EdgeInsets p) { setPadding(p); return *this; }
    Column& padding(int16_t all) { setPadding(EdgeInsets::all(all)); return *this; }
    Column& padding(int16_t h, int16_t v) { setPadding(EdgeInsets::symmetric(h, v)); return *this; }
    Column& bg(Color c) { setBackground(c); return *this; }

    Size measure(const Constraints& c) override {
        int16_t total_h = _padding.top + _padding.bottom;
        int16_t max_w = 0;
        int16_t content_w = c.max_w - _padding.left - _padding.right;

        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;
            Constraints cc(0, 0, content_w, (int16_t)(c.max_h - total_h));
            Size cs = _children[i]->measure(cc);
            _measured[i] = cs;
            if (i > 0) total_h += _spacing;
            total_h += cs.h;
            if (cs.w > max_w) max_w = cs.w;
        }
        max_w += _padding.left + _padding.right;
        return Size(
            (int16_t)constrain(max_w, c.min_w, c.max_w),
            (int16_t)constrain(total_h, c.min_h, c.max_h)
        );
    }

    void layout() override {
        int16_t avail_w = _bounds.w - _padding.left - _padding.right;

        // Count visible children and their total height
        int16_t total_child_h = 0;
        uint8_t visible = 0;
        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;
            total_child_h += _measured[i].h;
            visible++;
        }
        int16_t total_spacing = (visible > 1) ? _spacing * (visible - 1) : 0;
        int16_t extra = _bounds.h - _padding.top - _padding.bottom
                        - total_child_h - total_spacing;
        if (extra < 0) extra = 0;

        // Compute starting Y and gap based on arrangement
        int16_t cursor_y = _bounds.y + _padding.top;
        int16_t gap = _spacing;

        switch (_arrangement) {
            case Arrangement::CENTER:
                cursor_y += extra / 2;
                break;
            case Arrangement::END:
                cursor_y += extra;
                break;
            case Arrangement::SPACE_BETWEEN:
                if (visible > 1) gap = (extra + total_spacing) / (visible - 1);
                break;
            case Arrangement::SPACE_EVENLY:
                if (visible > 0) {
                    int16_t eg = (extra + total_spacing) / (visible + 1);
                    cursor_y += eg;
                    gap = eg;
                }
                break;
            default: break; // START
        }

        for (uint8_t i = 0; i < _child_count; i++) {
            if (!_children[i]->isVisible()) continue;

            int16_t child_w = _measured[i].w;
            int16_t child_x = _bounds.x + _padding.left;

            switch (_cross_align) {
                case Align::CENTER:  child_x += (avail_w - child_w) / 2; break;
                case Align::END:     child_x += avail_w - child_w; break;
                case Align::STRETCH: child_w = avail_w; break;
                default: break; // START
            }

            _children[i]->place(child_x, cursor_y, child_w, _measured[i].h);

            if (_children[i]->isLayout()) {
                static_cast<Layout*>(_children[i])->layout();
            }

            cursor_y += _measured[i].h + gap;
        }
    }

private:
    Size _measured[MAX_CHILDREN];
    Align _cross_align = Align::START;
    Arrangement _arrangement = Arrangement::START;
};

} // namespace PaperUI
