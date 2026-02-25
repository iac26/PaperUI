#pragma once

#include "pool.h"
#include "widgets/text_widget.h"
#include "widgets/value_widget.h"
#include "widgets/button_widget.h"
#include "widgets/switch_widget.h"
#include "widgets/slider_widget.h"
#include "widgets/checkbox_widget.h"
#include "widgets/progress_bar_widget.h"
#include "layouts/column.h"
#include "layouts/row.h"
#include "layouts/stack.h"
#include "layouts/spacer.h"

// Pool sizes — override before #include <PaperUI.h>
#ifndef PAPERUI_POOL_TEXT
#define PAPERUI_POOL_TEXT 12
#endif
#ifndef PAPERUI_POOL_VALUE
#define PAPERUI_POOL_VALUE 4
#endif
#ifndef PAPERUI_POOL_BUTTON
#define PAPERUI_POOL_BUTTON 4
#endif
#ifndef PAPERUI_POOL_SWITCH
#define PAPERUI_POOL_SWITCH 2
#endif
#ifndef PAPERUI_POOL_SLIDER
#define PAPERUI_POOL_SLIDER 2
#endif
#ifndef PAPERUI_POOL_CHECKBOX
#define PAPERUI_POOL_CHECKBOX 2
#endif
#ifndef PAPERUI_POOL_PROGRESS
#define PAPERUI_POOL_PROGRESS 2
#endif
#ifndef PAPERUI_POOL_COLUMN
#define PAPERUI_POOL_COLUMN 8
#endif
#ifndef PAPERUI_POOL_ROW
#define PAPERUI_POOL_ROW 8
#endif
#ifndef PAPERUI_POOL_STACK
#define PAPERUI_POOL_STACK 2
#endif
#ifndef PAPERUI_POOL_SPACER
#define PAPERUI_POOL_SPACER 4
#endif

namespace PaperUI {
namespace ui {

struct Pools {
    StaticPool<TextWidget,       PAPERUI_POOL_TEXT>     texts;
    StaticPool<ValueWidget,      PAPERUI_POOL_VALUE>    values;
    StaticPool<ButtonWidget,     PAPERUI_POOL_BUTTON>   buttons;
    StaticPool<SwitchWidget,     PAPERUI_POOL_SWITCH>   switches;
    StaticPool<SliderWidget,     PAPERUI_POOL_SLIDER>   sliders;
    StaticPool<CheckboxWidget,   PAPERUI_POOL_CHECKBOX> checkboxes;
    StaticPool<ProgressBarWidget,PAPERUI_POOL_PROGRESS> progressBars;
    StaticPool<Column,           PAPERUI_POOL_COLUMN>   columns;
    StaticPool<Row,              PAPERUI_POOL_ROW>      rows;
    StaticPool<Stack,            PAPERUI_POOL_STACK>     stacks;
    StaticPool<Spacer,           PAPERUI_POOL_SPACER>   spacers;
};

inline Pools& pools() {
    static Pools p;
    return p;
}

// --- Factory functions ---

inline TextWidget& text(const char* t, uint8_t sz = 2) {
    return pools().texts.alloc().text(t).fontSize(sz);
}

inline ValueWidget& value(const char* fmt = "%.1f", uint8_t sz = 2) {
    return pools().values.alloc().format(fmt).fontSize(sz);
}

inline ButtonWidget& button(const char* l) {
    return pools().buttons.alloc().label(l);
}

inline SwitchWidget& toggle() {
    return pools().switches.alloc();
}

inline SliderWidget& slider(int16_t lo = 0, int16_t hi = 100) {
    return pools().sliders.alloc().range(lo, hi);
}

inline CheckboxWidget& checkbox(const char* l) {
    return pools().checkboxes.alloc().label(l);
}

inline ProgressBarWidget& progress(int16_t val = 0, int16_t mx = 100) {
    return pools().progressBars.alloc().max(mx).value(val);
}

// --- Non-variadic layout factories (zero children) ---

inline Column& col(int16_t sp = 4) {
    Column& c = pools().columns.alloc();
    c.setSpacing(sp);
    return c;
}

inline Row& row(int16_t sp = 4) {
    Row& r = pools().rows.alloc();
    r.setSpacing(sp);
    return r;
}

inline Row& row(Arrangement arr, Align cross = Align::START, int16_t sp = 4) {
    return row(sp).arrange(arr).crossAlign(cross);
}

inline Stack& stack() {
    return pools().stacks.alloc();
}

// --- Variadic layout factories ---

template <typename... Children>
Column& col(int16_t sp, Children&... children) {
    Column& c = pools().columns.alloc();
    c.setSpacing(sp);
    using expander = int[];
    (void)expander{0, (c.add(&children), 0)...};
    return c;
}

template <typename... Children>
Row& row(int16_t sp, Children&... children) {
    Row& r = pools().rows.alloc();
    r.setSpacing(sp);
    using expander = int[];
    (void)expander{0, (r.add(&children), 0)...};
    return r;
}

template <typename... Children>
Row& row(Arrangement arr, Align cross, int16_t sp, Children&... children) {
    Row& r = row(sp, children...);
    r.arrange(arr).crossAlign(cross);
    return r;
}

template <typename... Children>
Stack& stack(Children&... children) {
    Stack& s = pools().stacks.alloc();
    using expander = int[];
    (void)expander{0, (s.add(&children), 0)...};
    return s;
}

// --- Other factories ---

inline Spacer& spacer(int16_t w = 0, int16_t h = 0) {
    return pools().spacers.alloc().size(w, h);
}

inline void reset() {
    pools().texts.reset();
    pools().values.reset();
    pools().buttons.reset();
    pools().switches.reset();
    pools().sliders.reset();
    pools().checkboxes.reset();
    pools().progressBars.reset();
    pools().columns.reset();
    pools().rows.reset();
    pools().stacks.reset();
    pools().spacers.reset();
}

} // namespace ui
} // namespace PaperUI
