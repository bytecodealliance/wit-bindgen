#include <assert.h>
#include <string>
#include <test_cpp.h>

wit::Task<wit::string> exports::my::test::i::Echo(wit::string s) {
    co_await wit::yield_async();
    std::string out(s.get_view());
    out += "!";
    co_return wit::string::from_view(out);
}

wit::Task<exports::my::test::i::Point> exports::my::test::i::MakePoint(uint32_t x, uint32_t y) {
    co_await wit::yield_async();
    std::string label = "(" + std::to_string(x) + ", " + std::to_string(y) + ")";
    co_return exports::my::test::i::Point{
        x,
        y,
        wit::string::from_view(label),
    };
}

wit::Task<wit::StreamReader<wit::string>> exports::my::test::i::Words(wit::vector<wit::string> all) {
    auto [tx, rx] = wit::stream_new<wit::string>();
    wit::spawn([tx = std::move(tx), all = std::move(all)]() mutable -> wit::Task<void> {
        size_t total = all.size();
        size_t written = co_await tx.write_all(std::move(all));
        assert(written == total);
    });
    co_return std::move(rx);
}
