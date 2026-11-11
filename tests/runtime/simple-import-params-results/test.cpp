#include <assert.h>
#include <test_cpp.h>

wit::Task<void> exports::a::b::i::OneArgument(uint32_t x) {
    assert(x == 1);
    co_return;
}

wit::Task<uint32_t> exports::a::b::i::OneResult() {
    co_return 2;
}

wit::Task<uint32_t> exports::a::b::i::OneArgumentAndResult(uint32_t x) {
    assert(x == 3);
    co_return 4;
}

wit::Task<void> exports::a::b::i::TwoArguments(uint32_t x, uint32_t y) {
    assert(x == 5);
    assert(y == 6);
    co_return;
}

wit::Task<uint32_t> exports::a::b::i::TwoArgumentsAndResult(uint32_t x, uint32_t y) {
    assert(x == 7);
    assert(y == 8);
    co_return 9;
}
