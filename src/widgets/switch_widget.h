#pragma once

#include "../widget.h"
#include "../state.h"

namespace PaperUI {

class SwitchWidget : public Widget {
public:
    bool isOn() const { return _on; }

    void setOn(bool on) {
        if (_on != on) { _on = on; markDirty(); }
    }

    void toggle() { setOn(!_on); }

    void setOnChange(OnChangeCallback cb, void* data = nullptr) {
        _on_change = cb;
        _user_data = data;
    }

    // Fluent setters
    SwitchWidget& on(bool v) { setOn(v); return *this; }
    SwitchWidget& onChange(OnChangeCallback cb, void* data = nullptr) { setOnChange(cb, data); return *this; }

    Size measure(const Constraints& c) override {
        return Size(
            (int16_t)constrain(_track_w, c.min_w, c.max_w),
            (int16_t)constrain(_track_h, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h,
                     Colors::WHITE);

        int16_t cx = _bounds.x;
        int16_t cy = _bounds.y + (_bounds.h - _track_h) / 2;
        int16_t r = _track_h / 2;

        // Track — use GRAY_MID when off so white thumb remains visible
        Color track_color = _on ? Colors::BLACK : Colors::GRAY_MID;
        gfx.fillRoundRect(cx, cy, _track_w, _track_h, r, track_color);

        // Thumb
        int16_t thumb_r = r - 2;
        int16_t thumb_cx = _on ? (cx + _track_w - r) : (cx + r);
        int16_t thumb_cy = cy + r;
        gfx.fillCircle(thumb_cx, thumb_cy, thumb_r, Colors::WHITE);
        gfx.drawCircle(thumb_cx, thumb_cy, thumb_r, Colors::BLACK);
    }

    bool onTouch(const TouchEvent& event) override {
        if (event.action == TouchAction::UP &&
            _bounds.contains(event.x, event.y)) {
            toggle();
            if (_bound) _bound->set(_on);
            if (_on_change) _on_change(_user_data, _on ? 1 : 0);
            return true;
        }
        return _bounds.contains(event.x, event.y);
    }

    // QUALITY needed to cleanly erase old thumb position
    UpdateHint updateHint() const override { return UpdateHint::QUALITY; }

    // Two-way bind to a State<bool>
    SwitchWidget& bind(State<bool>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            setOn(_bound->get());
        }
    }

private:
    bool _on = false;
    int16_t _track_w = 60;
    int16_t _track_h = 30;
    OnChangeCallback _on_change = nullptr;
    void* _user_data = nullptr;
    State<bool>* _bound = nullptr;
    uint32_t _last_gen = 0;
};

} // namespace PaperUI
