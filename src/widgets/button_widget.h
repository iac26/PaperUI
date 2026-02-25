#pragma once

#include "../widget.h"

namespace PaperUI {

class ButtonWidget : public Widget {
public:
    void setLabel(const char* label) {
        if (_label != label) { _label = label; markDirty(); }
    }

    void setOnClick(OnClickCallback cb, void* data = nullptr) {
        _on_click = cb;
        _user_data = data;
    }

    void setCornerRadius(int16_t r) { _radius = r; }

    void setPadding(EdgeInsets p) { _pad = p; }

    // Fluent setters
    ButtonWidget& label(const char* l) { setLabel(l); return *this; }
    ButtonWidget& onClick(OnClickCallback cb, void* data = nullptr) { setOnClick(cb, data); return *this; }
    ButtonWidget& radius(int16_t r) { setCornerRadius(r); return *this; }
    ButtonWidget& padding(EdgeInsets p) { setPadding(p); return *this; }
    ButtonWidget& padding(int16_t h, int16_t v) { setPadding(EdgeInsets::symmetric(h, v)); return *this; }

    Size measure(const Constraints& c) override {
        int16_t tw = CHAR_W * strlen(_label) * 2 + _pad.left + _pad.right;
        int16_t th = CHAR_H * 2 + _pad.top + _pad.bottom;
        return Size(
            (int16_t)constrain(tw, c.min_w, c.max_w),
            (int16_t)constrain(th, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        Color bg = _pressed ? Colors::BLACK : Colors::WHITE;
        Color fg = _pressed ? Colors::WHITE : Colors::BLACK;

        gfx.fillRoundRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h,
                           _radius, bg);
        gfx.drawRoundRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h,
                           _radius, Colors::BLACK);

        // Center label
        gfx.setTextSize(2);
        gfx.setTextColor(fg);
        int16_t lw = CHAR_W * strlen(_label) * 2;
        int16_t tx = _bounds.x + (_bounds.w - lw) / 2;
        int16_t ty = _bounds.y + (_bounds.h - CHAR_H * 2) / 2;
        gfx.drawString(_label, tx, ty);
    }

    bool onTouch(const TouchEvent& event) override {
        if (!_bounds.contains(event.x, event.y)) {
            if (_pressed) { _pressed = false; markDirty(); }
            return false;
        }
        switch (event.action) {
            case TouchAction::DOWN:
                _pressed = true;
                markDirty();
                return true;
            case TouchAction::UP:
                if (_pressed) {
                    _pressed = false;
                    markDirty();
                    if (_on_click) _on_click(_user_data);
                }
                return true;
            default:
                return true;
        }
    }

    UpdateHint updateHint() const override { return UpdateHint::MONO; }

private:
    const char* _label = "";
    bool _pressed = false;
    int16_t _radius = 8;
    EdgeInsets _pad = EdgeInsets::symmetric(16, 8);
    OnClickCallback _on_click = nullptr;
    void* _user_data = nullptr;

    static constexpr int16_t CHAR_W = 6;
    static constexpr int16_t CHAR_H = 8;
};

} // namespace PaperUI
