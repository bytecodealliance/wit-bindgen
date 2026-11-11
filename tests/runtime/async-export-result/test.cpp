#include <test_cpp.h>

wit::Task<std::expected<void, wit::Void>> exports::my::test::i::UnitOk() {
    co_return std::expected<void, wit::Void>();
}

wit::Task<std::expected<void, wit::string>> exports::my::test::i::UnitErr() {
    co_return std::unexpected(wit::string::from_view("unit"));
}

wit::Task<std::expected<uint32_t, wit::string>> exports::my::test::i::Halve(uint32_t x) {
    if (x % 2 != 0) {
        co_return std::unexpected(wit::string::from_view("odd"));
    }
    co_return x / 2;
}
