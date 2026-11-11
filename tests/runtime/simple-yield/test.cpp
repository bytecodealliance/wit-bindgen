#include <assert.h>
#include <test_cpp.h>

wit::Task<void> exports::a::b::i::F() {
    co_await wit::yield_async();
    assert(wit::yield_blocking());
}
