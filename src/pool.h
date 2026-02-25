#pragma once

#include <stdint.h>

namespace PaperUI {

template <typename T, uint16_t N>
class StaticPool {
public:
    T& alloc() {
        if (_count < N) {
            return _items[_count++];
        }
        // Exhausted: reuse last slot (better than crashing)
        return _items[N - 1];
    }

    void reset() { _count = 0; }
    uint16_t count() const { return _count; }
    static constexpr uint16_t capacity() { return N; }

private:
    T _items[N];
    uint16_t _count = 0;
};

} // namespace PaperUI
