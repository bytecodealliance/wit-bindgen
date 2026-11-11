#include <test_cpp.h>

wit::Task<void> exports::my::test::i::PendingImport(wit::FutureReader<void> x) {
    co_await x.read();
}

void exports::my::test::i::BackpressureSet(bool x) {
    if (x) {
        wit::backpressure_inc();
    } else {
        wit::backpressure_dec();
    }
}
