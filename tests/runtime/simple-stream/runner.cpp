//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

static wit::Task<void> write_items(wit::StreamWriter<void> tx) {
    auto s1 = co_await tx.write(1);
    assert(s1.state == wit::StreamState::Complete);
    assert(s1.count == 1);

    auto s2 = co_await tx.write(2);
    assert(s2.state == wit::StreamState::Complete);
    assert(s2.count == 2);

    auto s3 = co_await tx.write(2);
    assert(s3.state == wit::StreamState::Dropped);
    assert(s3.count == 0);
}

wit::Task<void> exports::runner::Run() {
    auto [tx, rx] = wit::stream_new<void>();
    auto w = write_items(std::move(tx));
    auto r = my::test::i::ReadStream(std::move(rx));
    co_await w;
    co_await r;
}
