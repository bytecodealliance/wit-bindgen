//@ wasmtime-flags = '-Wcomponent-model-async'

#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    co_await a::b::i::F();
}
