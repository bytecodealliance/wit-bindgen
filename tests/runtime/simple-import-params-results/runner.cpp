//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    co_await a::b::i::OneArgument(1);
    assert((co_await a::b::i::OneResult()) == 2);
    assert((co_await a::b::i::OneArgumentAndResult(3)) == 4);
    co_await a::b::i::TwoArguments(5, 6);
    assert((co_await a::b::i::TwoArgumentsAndResult(7, 8)) == 9);
}
