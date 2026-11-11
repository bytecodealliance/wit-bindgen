//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

static wit::Task<void> write_items(wit::StreamWriter<uint8_t> tx) {
    auto s1 = co_await tx.write({0});
    assert(s1.state == wit::StreamState::Complete);
    assert(s1.count == 1);

    auto s2 = co_await tx.write({1, 2});
    assert(s2.state == wit::StreamState::Complete);
    assert(s2.count == 2);

    auto s3 = co_await tx.write({3, 4});
    assert(s3.state == wit::StreamState::Complete);
    assert(s3.count == 2);

    auto s4 = co_await tx.write({0});
    assert(s4.state == wit::StreamState::Dropped);
    assert(s4.count == 0);
}

wit::Task<void> exports::runner::Run() {
    {
        auto [tx, rx] = wit::stream_new<uint8_t>();
        {
            auto drop_tx = std::move(tx);
            (void)drop_tx;
        }
        for (int i = 0; i < 2; ++i) {
            auto [st, buf] = co_await rx.read_chunk(1);
            assert(st.state == wit::StreamState::Dropped);
            assert(buf.empty());
        }
    }
    {
        auto [tx, rx] = wit::stream_new<uint8_t>();
        {
            auto drop_rx = std::move(rx);
            (void)drop_rx;
        }
        for (int i = 0; i < 2; ++i) {
            auto st = co_await tx.write({42});
            assert(st.state == wit::StreamState::Dropped);
            assert(st.count == 0);
        }
    }
    {
        auto [tx, rx] = wit::stream_new<uint8_t>();
        auto w = write_items(std::move(tx));
        auto r = my::test::i::ReadStream(std::move(rx));
        co_await w;
        co_await r;
    }
}
