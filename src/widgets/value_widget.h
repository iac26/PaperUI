#pragma once

#include "text_widget.h"
#include <cstdio>
#include <cstring>

namespace PaperUI {

class ValueWidget : public TextWidget {
public:
    ValueWidget() {
        text(_buf);
    }

    ValueWidget& format(const char* fmt) {
        _fmt = fmt;
        // Detect integer format specifiers (%d, %i, %u, %ld, %li, %lu)
        _int_fmt = false;
        const char* p = fmt;
        while (*p) {
            if (*p == '%') {
                p++;
                // skip flags/width/precision
                while (*p == '-' || *p == '+' || *p == ' ' || *p == '0' || *p == '#') p++;
                while (*p >= '0' && *p <= '9') p++;
                if (*p == '.') { p++; while (*p >= '0' && *p <= '9') p++; }
                // skip length modifiers
                while (*p == 'l' || *p == 'h') p++;
                if (*p == 'd' || *p == 'i' || *p == 'u') {
                    _int_fmt = true;
                }
                break;
            }
            p++;
        }
        return *this;
    }

    ValueWidget& operator=(float v) {
        if (v == _val && _buf[0] != '\0') return *this;
        _val = v;
        if (_int_fmt) {
            snprintf(_buf, sizeof(_buf), _fmt, (int32_t)v);
        } else {
            snprintf(_buf, sizeof(_buf), _fmt, v);
        }
        markDirty();
        return *this;
    }

    float get() const { return _val; }
    operator float() const { return _val; }

    // Measure based on _min_chars so bounds don't shrink/grow with content
    Size measure(const Constraints& c) override {
        int16_t chars = (int16_t)_min_chars;
        int16_t tw = CHAR_W * chars * fontSize_();
        int16_t th = CHAR_H * fontSize_();
        return Size(
            (int16_t)constrain(tw, c.min_w, c.max_w),
            (int16_t)constrain(th, c.min_h, c.max_h)
        );
    }

    // Value overwrites text in place — needs full refresh to avoid ghosting
    UpdateHint updateHint() const override { return UpdateHint::QUALITY; }

    // Set minimum character width for stable bounds
    ValueWidget& minChars(uint8_t n) { _min_chars = n; return *this; }

    // Forward fluent setters to keep chaining with ValueWidget& return type
    ValueWidget& fontSize(uint8_t sz) { TextWidget::fontSize(sz); return *this; }
    ValueWidget& color(Color c) { TextWidget::color(c); return *this; }
    ValueWidget& bgColor(Color c) { TextWidget::bgColor(c); return *this; }

    // Bind to a State<float> for reactive value updates
    ValueWidget& bind(State<float>& s) { _bound_val = &s; _last_val_gen = 0; return *this; }

    void sync() override {
        if (_bound_val && _bound_val->generation() != _last_val_gen) {
            _last_val_gen = _bound_val->generation();
            *this = _bound_val->get();
        }
    }

private:
    float _val = 0;
    const char* _fmt = "%.1f";
    char _buf[16] = "";
    bool _int_fmt = false;
    uint8_t _min_chars = 6;
    State<float>* _bound_val = nullptr;
    uint32_t _last_val_gen = 0;
};

} // namespace PaperUI
