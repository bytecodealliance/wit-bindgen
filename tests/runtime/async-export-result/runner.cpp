//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    auto r1 = co_await my::test::i::UnitOk();
    assert(r1.has_value());

    auto r2 = co_await my::test::i::UnitErr();
    assert(!r2.has_value());
    assert(r2.error().get_view() == "unit");

    auto r3 = co_await my::test::i::Halve(8);
    assert(r3.has_value());
    assert(*r3 == 4);

    auto r4 = co_await my::test::i::Halve(7);
    assert(!r4.has_value());
    assert(r4.error().get_view() == "odd");
}
