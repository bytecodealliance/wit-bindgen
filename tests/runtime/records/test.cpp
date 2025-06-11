#include <assert.h>
#include <test_cpp.h>

std::tuple<uint8_t, uint16_t> exports::test::records::to_test::MultipleResults() {
    return std::tuple<uint8_t, uint16_t>(4, 5);
}

std::tuple<uint32_t, uint8_t> exports::test::records::to_test::SwapTuple(std::tuple<uint8_t, uint32_t> a) {
    return std::tuple<uint32_t, uint8_t>(std::get<1>(a), std::get<0>(a));
}

test::records::to_test::F1 exports::test::records::to_test::RoundtripFlags1(::test::records::to_test::F1 a) {
    return a;
}

test::records::to_test::F2 exports::test::records::to_test::RoundtripFlags2(::test::records::to_test::F2 a) {
    return a;
}

std::tuple<test::records::to_test::Flag8, test::records::to_test::Flag16, test::records::to_test::Flag32> exports::test::records::to_test::RoundtripFlags3(::test::records::to_test::Flag8 a, ::test::records::to_test::Flag16 b, ::test::records::to_test::Flag32 c) {
    return std::tuple<::test::records::to_test::Flag8, ::test::records::to_test::Flag16, ::test::records::to_test::Flag32>(a, b, c);
}

test::records::to_test::R1 exports::test::records::to_test::RoundtripRecord1(::test::records::to_test::R1 a) {
    return a;
}

std::tuple<uint8_t> exports::test::records::to_test::Tuple1(std::tuple<uint8_t> a) {
    return std::tuple<uint8_t>(std::get<0>(a));
}
