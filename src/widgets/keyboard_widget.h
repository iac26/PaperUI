#pragma once

#include "../widget.h"

namespace PaperUI {

class KeyboardWidget : public Widget {
public:
    void setOnKey(OnKeyCallback cb, void* data = nullptr) {
        _on_key = cb;
        _user_data = data;
    }

    // Fluent setters
    KeyboardWidget& onKey(OnKeyCallback cb, void* data = nullptr) {
        setOnKey(cb, data); return *this;
    }

    Size measure(const Constraints& c) override {
        return Size(
            c.max_w,
            (int16_t)constrain((int16_t)(NUM_ROWS * ROW_H), c.min_h, c.max_h)
        );
    }

    void draw(M5GFX& gfx) override {
        gfx.fillRect(_bounds.x, _bounds.y, _bounds.w, _bounds.h, Colors::WHITE);

        int16_t cw = colWidth();
        for (uint8_t r = 0; r < NUM_ROWS; r++) {
            int16_t num_keys = rowCount(r);
            int16_t ky = _bounds.y + r * ROW_H;

            int16_t col = 0;
            for (int16_t k = 0; k < num_keys; k++) {
                int16_t span = getSpan(r, k);
                int16_t kx = _bounds.x + col * cw;
                int16_t kw = span * cw - KEY_GAP;
                int16_t kh = ROW_H - KEY_GAP;

                bool pressed = (_press_row == r && _press_key == k);
                Color bg = pressed ? Colors::BLACK : Colors::WHITE;
                Color fg = pressed ? Colors::WHITE : Colors::BLACK;

                gfx.fillRect(kx, ky, kw, kh, bg);
                gfx.drawRect(kx, ky, kw, kh, Colors::BLACK);

                const char* lbl = getLabel(r, k);
                gfx.setTextSize(2);
                gfx.setTextColor(fg);
                int16_t lw = strlen(lbl) * CHAR_W * 2;
                int16_t tx = kx + (kw - lw) / 2;
                int16_t ty = ky + (kh - CHAR_H * 2) / 2;
                gfx.drawString(lbl, tx, ty);

                col += span;
            }
        }
    }

    bool onTouch(const TouchEvent& event) override {
        if (!_bounds.contains(event.x, event.y)) {
            if (_press_row >= 0) { _press_row = -1; _press_key = -1; markDirty(); }
            return false;
        }

        switch (event.action) {
            case TouchAction::DOWN: {
                int8_t r, k;
                if (hitTest(event.x, event.y, r, k)) {
                    _press_row = r;
                    _press_key = k;
                    markDirty();
                }
                return true;
            }
            case TouchAction::UP: {
                if (_press_row >= 0 && _press_key >= 0) {
                    char ch = getChar(_press_row, _press_key);
                    _press_row = -1;
                    _press_key = -1;
                    markDirty();
                    if (_on_key && ch) _on_key(_user_data, ch);
                }
                return true;
            }
            default:
                return true;
        }
    }

    UpdateHint updateHint() const override { return UpdateHint::MONO; }

private:
    OnKeyCallback _on_key = nullptr;
    void* _user_data = nullptr;
    int8_t _press_row = -1;
    int8_t _press_key = -1;

    enum : int16_t { NUM_ROWS = 5, NUM_COLS = 10, ROW_H = 48, KEY_GAP = 2,
                     CHAR_W = 6, CHAR_H = 8 };

    int16_t colWidth() const { return _bounds.w / NUM_COLS; }

    // Row 0: Q W E R T Y U I O P       (10 keys)
    // Row 1: A S D F G H J K L <-      (10 keys)
    // Row 2: Z X C V B N M [spc] CLR   (9 keys, CLR span=2)
    // Row 3: [________SPACE________]   (1 key, span=10)
    // Row 4: 1 2 3 4 5 6 7 8 9 0       (10 keys)

    static const char* row0()   { return "QWERTYUIOP"; }
    static const char* row1()   { return "ASDFGHJKL"; }
    static const char* row2()   { return "ZXCVBNM"; }
    static const char* row4()   { return "1234567890"; }

    static int16_t rowCount(uint8_t r) {
        static const int16_t counts[] = {10, 10, 9, 1, 10};
        return (r < NUM_ROWS) ? counts[r] : 0;
    }

    static int16_t getSpan(uint8_t row, int16_t key) {
        if (row == 3) return NUM_COLS;
        if (row == 2 && key == 8) return 2;
        return 1;
    }

    static const char* getLabel(uint8_t row, int16_t key) {
        static char single[2] = {0, 0};
        switch (row) {
            case 0: single[0] = row0()[key]; return single;
            case 1:
                if (key < 9) { single[0] = row1()[key]; return single; }
                return "<-";
            case 2:
                if (key < 7) { single[0] = row2()[key]; return single; }
                if (key == 7) return " ";
                return "CLR";
            case 3: return "SPACE";
            case 4: single[0] = row4()[key]; return single;
            default: return "";
        }
    }

    static char getChar(int8_t row, int8_t key) {
        switch (row) {
            case 0: return (key < 10) ? row0()[key] : 0;
            case 1:
                if (key < 9) return row1()[key];
                return '\b';
            case 2:
                if (key < 7) return row2()[key];
                if (key == 7) return ' ';
                return '\0';
            case 3: return ' ';
            case 4: return (key < 10) ? row4()[key] : 0;
            default: return 0;
        }
    }

    bool hitTest(int16_t px, int16_t py, int8_t& out_row, int8_t& out_key) const {
        int16_t ry = py - _bounds.y;
        int8_t r = (int8_t)(ry / ROW_H);
        if (r < 0 || r >= NUM_ROWS) return false;

        int16_t cw = colWidth();
        int16_t rx = px - _bounds.x;
        int16_t col = 0;
        int16_t nk = rowCount(r);
        for (int16_t k = 0; k < nk; k++) {
            int16_t span = getSpan(r, k);
            int16_t kx = col * cw;
            int16_t kw = span * cw;
            if (rx >= kx && rx < kx + kw) {
                out_row = r;
                out_key = k;
                return true;
            }
            col += span;
        }
        return false;
    }
};

} // namespace PaperUI
