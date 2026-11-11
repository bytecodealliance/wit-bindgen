//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    // 1. Test cancelling an import in progress
    {
        auto [tx, rx] = wit::future_new<void>();
        {
            auto import_task = my::test::i::PendingImport(std::move(rx));
            assert(!import_task.done());
        }
        assert(!(co_await tx.write()));
    }

    // 2. Test cancelling an import in the started/starting state with backpressure
    {
        auto [tx1, rx1] = wit::future_new<void>();
        auto [tx2, rx2] = wit::future_new<void>();

        auto started_import = my::test::i::PendingImport(std::move(rx1));
        assert(!started_import.done());

        my::test::i::BackpressureSet(true);
        {
            auto starting_import = my::test::i::PendingImport(std::move(rx2));
            assert(!starting_import.done());
            // Destroy starting_import while in STARTING state
        }
        {
            auto drop_started = std::move(started_import);
            (void)drop_started;
        }
        my::test::i::BackpressureSet(false);

        assert(!(co_await tx1.write()));
        assert(!(co_await tx2.write()));
    }

    // 3. Race an import's cancellation with completion
    {
        auto [tx, rx] = wit::future_new<void>();
        auto import_task = my::test::i::PendingImport(std::move(rx));
        assert(!import_task.done());

        assert(co_await tx.write());
        for (int i = 0; i < 5; ++i) {
            co_await wit::yield_async();
        }
    }

    // 4. Race an import's cancellation with a pending status code (STARTING => STARTED)
    {
        auto [tx1, rx1] = wit::future_new<void>();
        auto started_import = my::test::i::PendingImport(std::move(rx1));
        assert(!started_import.done());

        my::test::i::BackpressureSet(true);
        auto [tx2, rx2] = wit::future_new<void>();
        {
            auto starting_import = my::test::i::PendingImport(std::move(rx2));
            assert(!starting_import.done());

            my::test::i::BackpressureSet(false);
            for (int i = 0; i < 5; ++i) {
                co_await wit::yield_async();
            }
            // Destroy starting_import after STARTING => STARTED notification
        }

        assert(!(co_await tx2.write()));

        assert(co_await tx1.write());
        co_await started_import;
    }
}
