#include <assert.h>
#include <test_cpp.h>

using namespace exports::test::fixed_size_lists;

void to_test::ListParam(std::array<uint32_t, 4> a) {
    std::array<uint32_t, 4> b = std::array<uint32_t, 4>{1, 2, 3, 4};
    assert(a == b);
}
void to_test::ListParam2(std::array<std::array<uint32_t, 2>, 2> a) {
    std::array<std::array<uint32_t, 2>, 2> b = std::array<std::array<uint32_t, 2>, 2>{std::array<uint32_t, 2>{1, 2}, std::array<uint32_t, 2>{3, 4}};
    assert(a == b);
}
void to_test::ListParam3(std::array<int32_t, 20> a) {
    std::array<int32_t, 20> b = std::array<int32_t, 20>{-1, 2, -3, 4, -5, 6, -7, 8, -9, 10, -11, 12, -13, 14, -15, 16, -17, 18, -19, 20};
    assert(a == b);
}
std::array<uint8_t, 8> to_test::ListResult() {
    return std::array<uint8_t, 8>{'0', '1', 'A', 'B', 'a', 'b', 128, 255};
}
std::tuple<std::array<uint16_t, 4>, std::array<int16_t, 4>>
to_test::ListMinmax16(std::array<uint16_t, 4> a, std::array<int16_t, 4> b) {
    return std::tuple<std::array<uint16_t, 4>, std::array<int16_t, 4>>(a, b);
}
std::tuple<std::array<float, 2>, std::array<double, 2>>
to_test::ListMinmaxFloat(std::array<float, 2> a, std::array<double, 2> b) {
    return std::tuple<std::array<float, 2>, std::array<double, 2>>(a,b);
}
std::array<uint8_t, 12> to_test::ListRoundtrip(std::array<uint8_t, 12> a) {
    return a;
}

std::tuple<std::array<std::array<uint32_t, 2>, 2>,
           std::array<std::array<int32_t, 2>, 2>>
to_test::NestedRoundtrip(std::array<std::array<uint32_t, 2>, 2> a,
                std::array<std::array<int32_t, 2>, 2> b) {
    return std::tuple<std::array<std::array<uint32_t, 2>, 2>,
           std::array<std::array<int32_t, 2>, 2>>(a, b);
}

std::tuple<std::array<std::array<uint32_t, 2>, 2>,
           std::array<std::array<int32_t, 4>, 4>>
to_test::LargeRoundtrip(std::array<std::array<uint32_t, 2>, 2> a,
               std::array<std::array<int32_t, 4>, 4> b) {
    return std::tuple<std::array<std::array<uint32_t, 2>, 2>,
           std::array<std::array<int32_t, 4>, 4>>(a, b);
}
std::array<to_test::Nested, 2>
to_test::NightmareOnCpp(std::array<to_test::Nested, 2> a) {
    return a;
}
