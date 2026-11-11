//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    {
        auto [tx, rx] = wit::future_new<void>();
        auto import_task = my::test::i::PendingImport(std::move(rx));
        assert(!import_task.done());
        assert(co_await tx.write());
        co_await import_task;
    }
    {
        auto [tx, rx] = wit::future_new<void>();
        auto import_task = my::test::i::PendingImport(std::move(rx));
        assert(!import_task.done());
        assert(co_await tx.write());
        for (int i = 0; i < 5; ++i) {
            co_await wit::yield_async();
        }
    }
}
