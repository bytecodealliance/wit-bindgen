#include <assert.h>
#include <test_cpp.h>

namespace test_exports = ::exports::test::records::to_test;

std::tuple<uint8_t, uint16_t> test_exports::MultipleResults() {
    return std::tuple<uint8_t, uint16_t>(4, 5);
}

std::tuple<uint32_t, uint8_t> test_exports::SwapTuple(std::tuple<uint8_t, uint32_t> a) {
    return std::tuple<uint32_t, uint8_t>(std::get<1>(a), std::get<0>(a));
}

test_exports::F1 test_exports::RoundtripFlags1(test_exports::F1 a) {
    return a;
}

test_exports::F2 test_exports::RoundtripFlags2(test_exports::F2 a) {
    return a;
}

std::tuple<test_exports::Flag8, test_exports::Flag16, test_exports::Flag32> test_exports::RoundtripFlags3(test_exports::Flag8 a, test_exports::Flag16 b, test_exports::Flag32 c) {
    return std::tuple<test_exports::Flag8, test_exports::Flag16, test_exports::Flag32>(a, b, c);
}

test_exports::R1 test_exports::RoundtripRecord1(R1 a) {
    return a;
}

std::tuple<uint8_t> test_exports::Tuple1(std::tuple<uint8_t> a) {
    return std::tuple<uint8_t>(std::get<0>(a));
}
