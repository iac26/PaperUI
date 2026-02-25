#pragma once

#include "../widget.h"
#include "../state.h"

namespace PaperUI {

class ProgressBarWidget : public Widget {
public:
    void setValue(int16_t v) {
        v = constrain(v, (int16_t)0, _max);
        if (_value != v) { _value = v; markDirty(); }
    }

    void setMax(int16_t m) {
        _max = m;
        setValue(_value); // re-clamp
    }

    int16_t value() const { return _value; }

    // Fluent setters
    ProgressBarWidget& value(int16_t v) { setValue(v); return *this; }
    ProgressBarWidget& max(int16_t m) { setMax(m); return *this; }

    Size measure(const Constraints& c) override {
        return Size(
            (int16_t)constrain((int16_t)200, c.min_w, c.max_w),
            (int16_t)constrain(BAR_H, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h,
                     Colors::WHITE);

        int16_t bar_y = _bounds.y + (_bounds.h - BAR_H) / 2;

        // Track outline
        gfx.drawRect(_bounds.x, bar_y, _bounds.w, BAR_H, Colors::BLACK);

        // Filled portion
        float frac = (_max > 0)
            ? (float)_value / (float)_max
            : 0.0f;
        int16_t fill_w = (int16_t)(frac * (_bounds.w - 4));
        if (fill_w > 0) {
            gfx.fillRect(_bounds.x + 2, bar_y + 2, fill_w, BAR_H - 4,
                         Colors::BLACK);
        }
    }

    UpdateHint updateHint() const override { return UpdateHint::FAST; }

    // Bind to a State<float> for reactive progress updates
    ProgressBarWidget& bind(State<float>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            setValue((int16_t)_bound->get());
        }
    }

private:
    int16_t _value = 0;
    int16_t _max = 100;
    State<float>* _bound = nullptr;
    uint32_t _last_gen = 0;

    static constexpr int16_t BAR_H = 20;
};

} // namespace PaperUI
