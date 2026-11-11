#include <test_cpp.h>

wit::Task<void> exports::my::test::i::PendingImport(wit::FutureReader<void> x) {
    co_await x.read();
}
