#pragma once

#include "../widget.h"

namespace PaperUI {

class Spacer : public Widget {
public:
    Spacer() = default;
    Spacer(int16_t w, int16_t h) : _fixed_w(w), _fixed_h(h) {}

    // Fluent setter
    Spacer& size(int16_t w, int16_t h) { _fixed_w = w; _fixed_h = h; markDirty(); return *this; }

    Size measure(const Constraints& c) override {
        return {
            (int16_t)constrain(_fixed_w, c.min_w, c.max_w),
            (int16_t)constrain(_fixed_h, c.min_h, c.max_h)
        };
    }

    void draw(M5GFX&) override {} // invisible

    UpdateHint updateHint() const override { return UpdateHint::NONE; }

private:
    int16_t _fixed_w = 0;
    int16_t _fixed_h = 0;
};

} // namespace PaperUI
