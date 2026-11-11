#include <assert.h>
#include <test_cpp.h>

wit::Task<void> exports::my::test::i::ReadStream(wit::StreamReader<uint8_t> x) {
    auto [s1, b1] = co_await x.read_chunk(1);
    assert(s1.state == wit::StreamState::Complete);
    assert(b1.size() == 1);
    assert(b1[0] == 0);

    auto [s2, b2] = co_await x.read_chunk(2);
    assert(s2.state == wit::StreamState::Complete);
    assert(b2.size() == 2);
    assert(b2[0] == 1);
    assert(b2[1] == 2);

    auto [s3, b3] = co_await x.read_chunk(1);
    assert(s3.state == wit::StreamState::Complete);
    assert(b3.size() == 1);
    assert(b3[0] == 3);

    auto [s4, b4] = co_await x.read_chunk(1);
    assert(s4.state == wit::StreamState::Complete);
    assert(b4.size() == 1);
    assert(b4[0] == 4);
}
