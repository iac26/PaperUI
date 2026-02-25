#pragma once

#include <stdint.h>

namespace PaperUI {

// Non-template base for global change tracking across all State instances.
// Screen::update() checks global_gen() to skip tree walks when nothing changed.
struct StateBase {
    static uint32_t& global_gen() {
        static uint32_t g = 0;
        return g;
    }
};

// Lightweight reactive state container.
// Tracks a generation counter so widgets can efficiently detect changes.
template <typename T>
class State : public StateBase {
public:
    State() = default;
    explicit State(T initial) : _value(initial) {}

    const T& get() const { return _value; }

    // Set value. Returns true if it actually changed.
    bool set(const T& new_val) {
        if (_value != new_val) {
            _value = new_val;
            _generation++;
            global_gen()++;
            return true;
        }
        return false;
    }

    uint32_t generation() const { return _generation; }

    operator const T&() const { return _value; }

private:
    T _value = {};
    uint32_t _generation = 0;
};

} // namespace PaperUI
