#pragma once

#include "../widget.h"
#include "../state.h"
#include <cstdio>

namespace PaperUI {

class BatteryWidget : public Widget {
public:
    void setVoltage(int16_t mv) {
        if (_mv != mv) { _mv = mv; markDirty(); }
    }

    // Fluent setters
    BatteryWidget& voltage(int16_t mv) { setVoltage(mv); return *this; }

    // Bind to State<float> (millivolts)
    BatteryWidget& bind(State<float>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            setVoltage((int16_t)_bound->get());
        }
    }

    Size measure(const Constraints& c) override {
        // Icon (ICON_W x ICON_H + nub) + gap + voltage text (~6 chars at size 2)
        int16_t w = ICON_W + NUB_W + GAP + 6 * CHAR_W * 2;
        return Size(
            (int16_t)constrain(w, c.min_w, c.max_w),
            (int16_t)constrain(ICON_H, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, Colors::WHITE);

        int16_t ix = _bounds.x;
        int16_t iy = _bounds.y + (_bounds.h - ICON_H) / 2;

        // Battery body outline
        gfx.drawRect(ix, iy, ICON_W, ICON_H, Colors::BLACK);
        // Positive terminal nub
        gfx.fillRect(ix + ICON_W, iy + (ICON_H - NUB_H) / 2, NUB_W, NUB_H, Colors::BLACK);

        // Fill level: 4.2V=100%, 3.5V=0%
        float pct = (_mv > 0) ? (float)(_mv - 3500) / 700.0f : 0.0f;
        if (pct < 0.0f) pct = 0.0f;
        if (pct > 1.0f) pct = 1.0f;
        int16_t fill_w = (int16_t)(pct * (ICON_W - 4));
        if (fill_w > 0) {
            gfx.fillRect(ix + 2, iy + 2, fill_w, ICON_H - 4, Colors::BLACK);
        }

        // Voltage text to the right of icon
        char buf[8];
        snprintf(buf, sizeof(buf), "%.2fV", _mv / 1000.0f);
        gfx.setTextSize(2);
        gfx.setTextColor(Colors::BLACK);
        gfx.setTextDatum(0);
        int16_t tx = ix + ICON_W + NUB_W + GAP;
        int16_t ty = _bounds.y + (_bounds.h - CHAR_H * 2) / 2;
        gfx.drawString(buf, tx, ty);
    }

    UpdateHint updateHint() const override { return UpdateHint::FAST; }

private:
    int16_t _mv = 0;
    State<float>* _bound = nullptr;
    uint32_t _last_gen = 0;

    enum : int16_t {
        ICON_W = 40, ICON_H = 20,
        NUB_W = 4, NUB_H = 10,
        GAP = 6,
        CHAR_W = 6, CHAR_H = 8
    };
};

} // namespace PaperUI
