#include <assert.h>
#include <test_cpp.h>

wit::Task<void> exports::my::test::i::ReadStream(wit::StreamReader<void> x) {
    auto s1 = co_await x.read(1);
    assert(s1.state == wit::StreamState::Complete);
    assert(s1.count == 1);

    auto s2 = co_await x.read(2);
    assert(s2.state == wit::StreamState::Complete);
    assert(s2.count == 2);
}
