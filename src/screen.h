#pragma once

#include "layout.h"
#include "state.h"

#ifdef PAPERUI_DEBUG
#define PUI_LOG(fmt, ...) Serial.printf("[PUI] " fmt "\n", ##__VA_ARGS__)
#else
#define PUI_LOG(fmt, ...) ((void)0)
#endif

namespace PaperUI {

constexpr uint8_t MAX_DIRTY_RECTS = 8;
constexpr int16_t SCREEN_W = 540;
constexpr int16_t SCREEN_H = 960;
constexpr unsigned long TOUCH_DEBOUNCE_MS = 80;
constexpr uint16_t DEFAULT_FULL_REFRESH_INTERVAL = 10;

class Screen {
public:
    Screen() = default;

    void begin() {
        _gfx = &M5.Display;
        _gfx->setAutoDisplay(false);
    }

    void setRoot(Layout* root) { _root = root; }

    void root(Layout& r) { setRoot(&r); performLayout(); }

    // Full layout pass: measure -> place -> layout -> draw -> push.
    void performLayout() {
        if (!_root || !_gfx) return;
        Constraints sc(SCREEN_W, SCREEN_H, SCREEN_W, SCREEN_H);
        _root->measure(sc);
        _root->place(0, 0, SCREEN_W, SCREEN_H);
        _root->layout();
        // Full initial render
        _gfx->fillScreen(Colors::WHITE);
        _root->draw(*_gfx);
        _gfx->setEpdMode(epd_mode_t::epd_quality);
        _gfx->display();
        clearAllDirty(_root);
    }

    // Call every loop() iteration. Syncs state bindings, processes input, re-renders dirty regions.
    void update() {
        if (StateBase::global_gen() != _last_synced_gen) {
            syncAll(_root);
            _last_synced_gen = StateBase::global_gen();
        }
        processTouch();
        processButtons();
        render();
    }

    // Force a full-quality refresh (clears ghosting)
    void fullRefresh() {
        if (!_gfx || !_root) return;
        _gfx->fillScreen(Colors::WHITE);
        _root->draw(*_gfx);
        _gfx->setEpdMode(epd_mode_t::epd_quality);
        _gfx->display();
        _partial_count = 0;
    }

    // Set how many partial updates before an automatic full refresh.
    // 0 disables automatic full refresh.
    void setFullRefreshInterval(uint16_t n) { _full_refresh_interval = n; }

    // Button callbacks
    void setOnButtonLeft(OnClickCallback cb, void* d = nullptr) {
        _on_btn_left = cb; _btn_data = d;
    }
    void setOnButtonPush(OnClickCallback cb, void* d = nullptr) {
        _on_btn_push = cb; _btn_data = d;
    }
    void setOnButtonRight(OnClickCallback cb, void* d = nullptr) {
        _on_btn_right = cb; _btn_data = d;
    }

    M5GFX& gfx() { return *_gfx; }

private:
    // --- Input ---

    void processTouch() {
        if (!_root) return;

        // Debounce: skip touch entirely during cooldown
        if (_debounce_until) {
            if (millis() < _debounce_until) return;
            _debounce_until = 0;
        }

        auto count = M5.Touch.getCount();
        if (count > 0) {
            auto t = M5.Touch.getDetail(0);
            TouchEvent ev;
            ev.x = t.x;
            ev.y = t.y;
            ev.action = _touch_active ? TouchAction::MOVE : TouchAction::DOWN;
            _touch_active = true;
            _last_x = t.x;
            _last_y = t.y;
            PUI_LOG("touch %s (%d,%d)", ev.action == TouchAction::DOWN ? "DOWN" : "MOVE", ev.x, ev.y);
            _root->onTouch(ev);
        } else if (_touch_active) {
            _touch_active = false;
            _debounce_until = millis() + TOUCH_DEBOUNCE_MS;
            TouchEvent ev;
            ev.x = _last_x;
            ev.y = _last_y;
            ev.action = TouchAction::UP;
            PUI_LOG("touch UP (%d,%d)", ev.x, ev.y);
            _root->onTouch(ev);
        }
    }

    void processButtons() {
        if (M5.BtnA.wasPressed() && _on_btn_left)  _on_btn_left(_btn_data);
        if (M5.BtnB.wasPressed() && _on_btn_push)  _on_btn_push(_btn_data);
        if (M5.BtnC.wasPressed() && _on_btn_right) _on_btn_right(_btn_data);
    }

    // --- Rendering ---

    void render() {
        _dirty_count = 0;
        collectDirtyRects(_root);
        if (_dirty_count == 0) return;

        PUI_LOG("render: %d dirty rects", _dirty_count);

        // Merge if too fragmented
        if (_dirty_count > MAX_DIRTY_RECTS / 2) {
            Rect merged = _dirty_rects[0];
            for (uint8_t i = 1; i < _dirty_count; i++) {
                merged = merged.unite(_dirty_rects[i]);
            }
            _dirty_rects[0] = merged;
            _dirty_count = 1;
            PUI_LOG("  merged to (%d,%d %dx%d)", merged.x, merged.y, merged.w, merged.h);
        }

        // Clear and redraw widgets overlapping each dirty rect
        for (uint8_t r = 0; r < _dirty_count; r++) {
            PUI_LOG("  push rect[%d]: (%d,%d %dx%d)", r,
                    _dirty_rects[r].x, _dirty_rects[r].y,
                    _dirty_rects[r].w, _dirty_rects[r].h);
            _gfx->fillRect(_dirty_rects[r].x, _dirty_rects[r].y,
                           _dirty_rects[r].w, _dirty_rects[r].h, Colors::WHITE);
            redrawRegion(_root, _dirty_rects[r]);
        }

        clearAllDirty(_root);

        // Push each dirty rect to the e-ink display
        for (uint8_t r = 0; r < _dirty_count; r++) {
            pushDirtyRect(_dirty_rects[r]);
        }

        // Periodic full refresh to clear ghosting
        _partial_count++;
        if (_full_refresh_interval > 0 &&
            _partial_count >= _full_refresh_interval) {
            PUI_LOG("full refresh after %u partials", _partial_count);
            fullRefresh();
        }
    }

    void collectDirtyRects(Widget* w) {
        if (!w || !w->isVisible()) return;
        if (w->isDirty() && !w->isLayout()) {
            // Only collect leaf widget rects
            if (_dirty_count < MAX_DIRTY_RECTS) {
                _dirty_rects[_dirty_count++] = w->bounds();
            }
        }
        if (w->isLayout()) {
            Layout* lay = static_cast<Layout*>(w);
            for (uint8_t i = 0; i < lay->childCount(); i++) {
                collectDirtyRects(lay->child(i));
            }
        }
    }

    void redrawRegion(Widget* w, const Rect& region) {
        if (!w || !w->isVisible()) return;
        if (!w->bounds().intersects(region)) return;

        if (w->isLayout()) {
            Layout* lay = static_cast<Layout*>(w);
            if (lay->background() != Colors::WHITE) {
                const Rect& b = lay->bounds();
                _gfx->fillRect(b.x, b.y, b.w, b.h, lay->background());
            }
            for (uint8_t i = 0; i < lay->childCount(); i++) {
                redrawRegion(lay->child(i), region);
            }
        } else {
            w->draw(*_gfx);
        }
    }

    void clearAllDirty(Widget* w) {
        if (!w) return;
        w->clearDirty();
        if (w->isLayout()) {
            Layout* lay = static_cast<Layout*>(w);
            for (uint8_t i = 0; i < lay->childCount(); i++) {
                clearAllDirty(lay->child(i));
            }
        }
    }

    void markAllDirty(Widget* w) {
        if (!w) return;
        w->markDirty();
        if (w->isLayout()) {
            Layout* lay = static_cast<Layout*>(w);
            for (uint8_t i = 0; i < lay->childCount(); i++) {
                markAllDirty(lay->child(i));
            }
        }
    }

    // Push a dirty rect to the e-ink display with appropriate update mode
    void pushDirtyRect(const Rect& dr) {
        auto mode = selectEpdMode(dr);
        _gfx->setEpdMode(mode);
        _gfx->display(dr.x, dr.y, dr.w, dr.h);
    }

    epd_mode_t selectEpdMode(const Rect& region) {
        UpdateHint worst = worstHintInRegion(_root, region);
        switch (worst) {
            case UpdateHint::QUALITY: return epd_mode_t::epd_quality;
            case UpdateHint::TEXT:    return epd_mode_t::epd_text;
            case UpdateHint::FAST:    return epd_mode_t::epd_fast;
            case UpdateHint::MONO:    return epd_mode_t::epd_fastest;
            default:                  return epd_mode_t::epd_fast;
        }
    }

    UpdateHint worstHintInRegion(Widget* w, const Rect& region) {
        if (!w || !w->isVisible() || !w->bounds().intersects(region))
            return UpdateHint::NONE;

        UpdateHint result = UpdateHint::NONE;
        if (w->isDirty()) result = w->updateHint();

        if (w->isLayout()) {
            Layout* lay = static_cast<Layout*>(w);
            for (uint8_t i = 0; i < lay->childCount(); i++) {
                UpdateHint ch = worstHintInRegion(lay->child(i), region);
                if ((uint8_t)ch > (uint8_t)result) result = ch;
            }
        }
        return result;
    }

    void syncAll(Widget* w) {
        if (!w) return;
        w->sync();
        if (w->isLayout()) {
            Layout* lay = static_cast<Layout*>(w);
            for (uint8_t i = 0; i < lay->childCount(); i++)
                syncAll(lay->child(i));
        }
    }

    // --- Members ---
    M5GFX* _gfx = nullptr;
    Layout* _root = nullptr;
    uint32_t _last_synced_gen = 0;

    // Touch state
    bool _touch_active = false;
    int16_t _last_x = 0;
    int16_t _last_y = 0;
    unsigned long _debounce_until = 0;

    // Dirty tracking
    Rect _dirty_rects[MAX_DIRTY_RECTS];
    uint8_t _dirty_count = 0;

    // Full refresh counter
    uint16_t _partial_count = 0;
    uint16_t _full_refresh_interval = DEFAULT_FULL_REFRESH_INTERVAL;

    // Button callbacks
    OnClickCallback _on_btn_left = nullptr;
    OnClickCallback _on_btn_push = nullptr;
    OnClickCallback _on_btn_right = nullptr;
    void* _btn_data = nullptr;
};

} // namespace PaperUI
