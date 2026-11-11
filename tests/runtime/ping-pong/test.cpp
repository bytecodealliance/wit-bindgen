#include <assert.h>
#include <string>
#include <test_cpp.h>

wit::Task<wit::FutureReader<wit::string>> exports::my::test::i::Ping(
    wit::FutureReader<wit::string> x, wit::string y) {
    auto first = co_await x.read();
    std::string combined(first.get_view());
    combined.append(y.get_view());
    auto [tx, rx] = wit::future_new<wit::string>();
    wit::spawn([tx = std::move(tx), msg = wit::string::from_view(combined)]() mutable -> wit::Task<void> {
        bool ok = co_await tx.write(std::move(msg));
        assert(ok);
    });
    co_return std::move(rx);
}

wit::Task<wit::string> exports::my::test::i::Pong(wit::FutureReader<wit::string> x) {
    co_return co_await x.read();
}
