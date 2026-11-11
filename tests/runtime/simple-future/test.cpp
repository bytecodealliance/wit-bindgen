#include <test_cpp.h>

wit::Task<void> exports::my::test::i::ReadFuture(wit::FutureReader<void> x) {
    co_await x.read();
}

wit::Task<void> exports::my::test::i::DropFuture(wit::FutureReader<void> x) {
    (void)x;
    co_return;
}
