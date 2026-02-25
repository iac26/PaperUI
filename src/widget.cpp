#include "widget.h"
#include "layout.h"

namespace PaperUI {

void Widget::markDirty() {
    _dirty = true;
    if (_parent) _parent->onChildDirty(this);
}

} // namespace PaperUI
