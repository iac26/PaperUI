#pragma once

// PaperUI — Declarative UI framework for M5Stack Paper e-ink displays
//
// Usage:
//   #include <PaperUI.h>
//   using namespace PaperUI;

// Foundation
#include "src/types.h"
#include "src/state.h"
#include "src/widget.h"
#include "src/layout.h"
#include "src/screen.h"

// Widgets
#include "src/widgets/text_widget.h"
#include "src/widgets/value_widget.h"
#include "src/widgets/button_widget.h"
#include "src/widgets/switch_widget.h"
#include "src/widgets/slider_widget.h"
#include "src/widgets/checkbox_widget.h"
#include "src/widgets/progress_bar_widget.h"

// Layouts
#include "src/layouts/column.h"
#include "src/layouts/row.h"
#include "src/layouts/stack.h"
#include "src/layouts/spacer.h"

// Builder API
#include "src/pool.h"
#include "src/ui.h"
