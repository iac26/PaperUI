#pragma once

#include "../widget.h"
#include "../state.h"

namespace PaperUI {

class SliderWidget : public Widget {
public:
    int16_t value() const { return _value; }

    void setValue(int16_t v) {
        v = constrain(v, _min, _max);
        if (_value != v) { _value = v; markDirty(); }
    }

    void setRange(int16_t min_val, int16_t max_val) {
        _min = min_val;
        _max = max_val;
        setValue(_value); // re-clamp
    }

    void setOnChange(OnChangeCallback cb, void* data = nullptr) {
        _on_change = cb;
        _user_data = data;
    }

    // Fluent setters
    SliderWidget& value(int16_t v) { setValue(v); return *this; }
    SliderWidget& range(int16_t lo, int16_t hi) { setRange(lo, hi); return *this; }
    SliderWidget& onChange(OnChangeCallback cb, void* data = nullptr) { setOnChange(cb, data); return *this; }

    Size measure(const Constraints& c) override {
        return Size(
            (int16_t)constrain((int16_t)200, c.min_w, c.max_w),
            (int16_t)constrain((int16_t)40, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h,
                     Colors::WHITE);

        int16_t track_y = _bounds.y + _bounds.h / 2;
        int16_t x0 = _bounds.x + THUMB_R;
        int16_t x1 = _bounds.x + _bounds.w - THUMB_R;
        int16_t track_len = x1 - x0;

        // Track background (2px thick line)
        gfx.fillRect(x0, track_y - 1, track_len, 2, Colors::GRAY_MID);

        // Filled portion
        float frac = (_max > _min)
            ? (float)(_value - _min) / (float)(_max - _min)
            : 0.0f;
        int16_t fill_w = (int16_t)(frac * track_len);
        gfx.fillRect(x0, track_y - 1, fill_w, 2, Colors::BLACK);

        // Thumb
        int16_t thumb_cx = x0 + fill_w;
        gfx.fillCircle(thumb_cx, track_y, THUMB_R, Colors::WHITE);
        gfx.drawCircle(thumb_cx, track_y, THUMB_R, Colors::BLACK);
    }

    bool onTouch(const TouchEvent& event) override {
        if (event.action == TouchAction::DOWN &&
            _bounds.contains(event.x, event.y)) {
            _dragging = true;
        }

        if (_dragging && (event.action == TouchAction::DOWN ||
                          event.action == TouchAction::MOVE)) {
            int16_t x0 = _bounds.x + THUMB_R;
            int16_t x1 = _bounds.x + _bounds.w - THUMB_R;
            float frac = (float)(event.x - x0) / (float)(x1 - x0);
            frac = constrain(frac, 0.0f, 1.0f);
            int16_t nv = _min + (int16_t)(frac * (_max - _min));
            if (nv != _value) {
                setValue(nv);
                if (_bound) _bound->set((float)_value);
                if (_on_change) _on_change(_user_data, _value);
            }
            return true;
        }

        if (event.action == TouchAction::UP) {
            _dragging = false;
            return true;
        }

        return _bounds.contains(event.x, event.y);
    }

    UpdateHint updateHint() const override { return UpdateHint::FAST; }

    // Two-way bind to a State<float>
    SliderWidget& bind(State<float>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            setValue((int16_t)_bound->get());
        }
    }

private:
    int16_t _value = 0;
    int16_t _min = 0;
    int16_t _max = 100;
    bool _dragging = false;
    OnChangeCallback _on_change = nullptr;
    void* _user_data = nullptr;
    State<float>* _bound = nullptr;
    uint32_t _last_gen = 0;

    static constexpr int16_t THUMB_R = 12;
};

} // namespace PaperUI
