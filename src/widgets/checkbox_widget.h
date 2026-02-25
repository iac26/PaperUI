#pragma once

#include "../widget.h"
#include "../state.h"

namespace PaperUI {

class CheckboxWidget : public Widget {
public:
    bool isChecked() const { return _checked; }

    void setChecked(bool c) {
        if (_checked != c) { _checked = c; markDirty(); }
    }

    void setLabel(const char* label) {
        if (_label != label) { _label = label; markDirty(); }
    }

    void setOnChange(OnChangeCallback cb, void* data = nullptr) {
        _on_change = cb;
        _user_data = data;
    }

    // Fluent setters
    CheckboxWidget& label(const char* l) { setLabel(l); return *this; }
    CheckboxWidget& checked(bool c) { setChecked(c); return *this; }
    CheckboxWidget& onChange(OnChangeCallback cb, void* data = nullptr) { setOnChange(cb, data); return *this; }

    Size measure(const Constraints& c) override {
        int16_t label_w = _label ? strlen(_label) * CHAR_W * 2 : 0;
        int16_t tw = BOX_SIZE + GAP + label_w;
        return Size(
            (int16_t)constrain(tw, c.min_w, c.max_w),
            (int16_t)constrain(BOX_SIZE, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h,
                     Colors::WHITE);

        int16_t bx = _bounds.x;
        int16_t by = _bounds.y + (_bounds.h - BOX_SIZE) / 2;

        // Box outline
        gfx.drawRect(bx, by, BOX_SIZE, BOX_SIZE, Colors::BLACK);

        // Fill when checked
        if (_checked) {
            gfx.fillRect(bx + 4, by + 4, BOX_SIZE - 8, BOX_SIZE - 8,
                         Colors::BLACK);
        }

        // Label
        if (_label) {
            gfx.setTextSize(2);
            gfx.setTextColor(Colors::BLACK);
            gfx.drawString(_label, bx + BOX_SIZE + GAP,
                           _bounds.y + (_bounds.h - CHAR_H * 2) / 2);
        }
    }

    bool onTouch(const TouchEvent& event) override {
        if (event.action == TouchAction::UP &&
            _bounds.contains(event.x, event.y)) {
            _checked = !_checked;
            markDirty();
            if (_bound) _bound->set(_checked);
            if (_on_change) _on_change(_user_data, _checked ? 1 : 0);
            return true;
        }
        return _bounds.contains(event.x, event.y);
    }

    UpdateHint updateHint() const override { return UpdateHint::MONO; }

    // Two-way bind to a State<bool>
    CheckboxWidget& bind(State<bool>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            setChecked(_bound->get());
        }
    }

private:
    bool _checked = false;
    const char* _label = nullptr;
    OnChangeCallback _on_change = nullptr;
    void* _user_data = nullptr;
    State<bool>* _bound = nullptr;
    uint32_t _last_gen = 0;

    static constexpr int16_t BOX_SIZE = 28;
    static constexpr int16_t GAP = 8;
    static constexpr int16_t CHAR_W = 6;
    static constexpr int16_t CHAR_H = 8;
};

} // namespace PaperUI
