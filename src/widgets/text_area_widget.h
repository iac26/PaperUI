#pragma once

#include "../widget.h"
#include <cstring>

namespace PaperUI {

class TextAreaWidget : public Widget {
public:
    TextAreaWidget() { _buf[0] = '\0'; }

    void appendChar(char c) {
        if (_len < sizeof(_buf) - 1) {
            _buf[_len++] = c;
            _buf[_len] = '\0';
            markDirty();
        }
    }

    void deleteChar() {
        if (_len > 0) {
            _len--;
            _buf[_len] = '\0';
            markDirty();
        }
    }

    void clear() {
        if (_len > 0) {
            _len = 0;
            _buf[0] = '\0';
            markDirty();
        }
    }

    const char* text() const { return _buf; }
    uint16_t length() const { return _len; }

    // Fluent setters
    TextAreaWidget& fontSize(uint8_t sz) { _font_size = sz; return *this; }
    TextAreaWidget& color(Color c) { _fg = c; return *this; }
    TextAreaWidget& bgColor(Color c) { _bg = c; return *this; }
    TextAreaWidget& height(int16_t h) { _height = h; return *this; }

    Size measure(const Constraints& c) override {
        return Size(
            c.max_w,
            (int16_t)constrain(_height, c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        // Background
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, _bg);
        // Border
        gfx.drawRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, Colors::BLACK);

        int16_t char_w = CHAR_W * _font_size;
        int16_t char_h = CHAR_H * _font_size;
        int16_t inner_w = _bounds.w - 2 * PAD;
        int16_t chars_per_line = (inner_w > 0 && char_w > 0) ? inner_w / char_w : 1;
        if (chars_per_line < 1) chars_per_line = 1;

        gfx.setTextSize(_font_size);
        gfx.setTextColor(_fg);
        gfx.setTextDatum(0);

        int16_t tx = _bounds.x + PAD;
        int16_t ty = _bounds.y + PAD;
        int16_t max_y = _bounds.y + _bounds.h - PAD - char_h;

        // Word-wrap: draw chars_per_line characters per line
        uint16_t pos = 0;
        while (pos < _len && ty <= max_y) {
            uint16_t line_len = _len - pos;
            if (line_len > (uint16_t)chars_per_line) line_len = chars_per_line;

            // Draw one line
            char tmp[64];
            if (line_len > sizeof(tmp) - 1) line_len = sizeof(tmp) - 1;
            memcpy(tmp, _buf + pos, line_len);
            tmp[line_len] = '\0';
            gfx.drawString(tmp, tx, ty);

            pos += line_len;
            ty += char_h + 2;
        }

        // Draw cursor underscore after last character
        int16_t cursor_col = _len % chars_per_line;
        int16_t cursor_row = _len / chars_per_line;
        int16_t cx = _bounds.x + PAD + cursor_col * char_w;
        int16_t cy = _bounds.y + PAD + cursor_row * (char_h + 2);
        if (cy <= max_y) {
            gfx.fillRect(cx, cy + char_h - 2, char_w, 2, _fg);
        }
    }

    UpdateHint updateHint() const override { return UpdateHint::TEXT; }

private:
    char _buf[256];
    uint16_t _len = 0;
    uint8_t _font_size = 2;
    Color _fg = Colors::BLACK;
    Color _bg = Colors::WHITE;
    int16_t _height = 120;

    static constexpr int16_t PAD = 6;
    static constexpr int16_t CHAR_W = 6;
    static constexpr int16_t CHAR_H = 8;
};

} // namespace PaperUI
