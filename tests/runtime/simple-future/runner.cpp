//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    {
        auto [tx, rx] = wit::future_new<void>();
        auto write_task = tx.write();
        auto read_task = my::test::i::ReadFuture(std::move(rx));
        assert(co_await write_task);
        co_await read_task;
    }
    {
        auto [tx, rx] = wit::future_new<void>();
        auto write_task = tx.write();
        auto drop_task = my::test::i::DropFuture(std::move(rx));
        assert(!(co_await write_task));
        co_await drop_task;
    }
}
