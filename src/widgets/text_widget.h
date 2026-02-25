#pragma once

#include "../widget.h"
#include "../state.h"

namespace PaperUI {

class TextWidget : public Widget {
public:
    void setText(const char* text) {
        if (_text != text) {
            _text = text;
            markDirty();
        }
    }

    void setFontSize(uint8_t sz) {
        if (_font_size != sz) { _font_size = sz; markDirty(); }
    }

    void setColor(Color c) {
        if (_fg != c) { _fg = c; markDirty(); }
    }

    void setBgColor(Color c) {
        if (_bg != c) { _bg = c; markDirty(); }
    }

    const char* text() const { return _text; }

    // Fluent setters
    TextWidget& text(const char* t) { setText(t); return *this; }
    TextWidget& fontSize(uint8_t sz) { setFontSize(sz); return *this; }
    TextWidget& color(Color c) { setColor(c); return *this; }
    TextWidget& bgColor(Color c) { setBgColor(c); return *this; }

    Size measure(const Constraints& c) override {
        int16_t tw = CHAR_W * strlen(_text) * _font_size;
        int16_t th = CHAR_H * _font_size;
        return Size(
            (int16_t)constrain(tw, c.min_w, c.max_w),
            (int16_t)constrain(th, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, _bg);
        gfx.setTextSize(_font_size);
        gfx.setTextColor(_fg);
        gfx.setTextDatum(0);
        gfx.drawString(_text, _bounds.x, _bounds.y);
    }

    UpdateHint updateHint() const override { return UpdateHint::TEXT; }

    // Bind to a State<const char*> for reactive text updates
    TextWidget& bind(State<const char*>& s) { _bound = &s; _last_gen = 0; return *this; }

    void sync() override {
        if (_bound && _bound->generation() != _last_gen) {
            _last_gen = _bound->generation();
            setText(_bound->get());
        }
    }

protected:
    uint8_t fontSize_() const { return _font_size; }
    static constexpr int16_t CHAR_W = 6;
    static constexpr int16_t CHAR_H = 8;

private:
    const char* _text = "";
    Color _fg = Colors::BLACK;
    Color _bg = Colors::WHITE;
    uint8_t _font_size = 2;
    State<const char*>* _bound = nullptr;
    uint32_t _last_gen = 0;
};

} // namespace PaperUI
