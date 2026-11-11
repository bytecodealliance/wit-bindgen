#include <test_cpp.h>

wit::Task<void> exports::a::b::i::F() {
    for (int i = 0; i < 10; ++i) {
        co_await wit::yield_async();
    }
}
