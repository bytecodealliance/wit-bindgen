#include <assert.h>
#include <test_cpp.h>

static wit::Task<void>
consume_stream(wit::StreamReader<uint8_t> data,
               wit::FutureWriter<std::expected<uint32_t, wit::string>> tx) {
    uint32_t sum = 0;
    bool saw_zero = false;
    for (;;) {
        auto [status, chunk] = co_await data.read_chunk(16);
        for (size_t i = 0; i < chunk.size(); ++i) {
            if (chunk[i] == 0) {
                saw_zero = true;
            }
            sum += chunk[i];
        }
        if (status.state != wit::StreamState::Complete) {
            break;
        }
    }
    std::expected<uint32_t, wit::string> res = sum;
    if (saw_zero) {
        res = std::unexpected(wit::string::from_view("zero"));
    }
    bool ok = co_await tx.write(std::move(res));
    assert(ok);
}

wit::Task<wit::FutureReader<std::expected<uint32_t, wit::string>>>
exports::my::test::i::Consume(wit::StreamReader<uint8_t> data) {
    auto [tx, rx] =
        wit::future_new<std::expected<uint32_t, wit::string>>();
    wit::spawn(consume_stream(std::move(data), std::move(tx)));
    co_return std::move(rx);
}
