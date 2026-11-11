//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    auto got = co_await my::test::i::ReturnString();
    assert(got.get_view() == "hello");
}
