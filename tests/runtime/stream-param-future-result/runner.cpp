//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    {
        auto [tx, rx] = wit::stream_new<uint8_t>();
        auto result = co_await my::test::i::Consume(std::move(rx));
        auto written = co_await tx.write({1, 2, 3});
        assert(written.state == wit::StreamState::Complete);
        assert(written.count == 3);
        {
            auto drop_tx = std::move(tx);
            (void)drop_tx;
        }
        auto val = co_await result.read();
        assert(val.has_value());
        assert(*val == 6);
    }

    {
        auto [tx, rx] = wit::stream_new<uint8_t>();
        auto result = co_await my::test::i::Consume(std::move(rx));
        auto written = co_await tx.write({0});
        assert(written.state == wit::StreamState::Complete);
        assert(written.count == 1);
        {
            auto drop_tx = std::move(tx);
            (void)drop_tx;
        }
        auto val = co_await result.read();
        assert(!val.has_value());
        assert(val.error().to_string() == "zero");
    }
}
