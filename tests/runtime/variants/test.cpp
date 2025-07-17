#include <assert.h>
#include <test_cpp.h>
#include <stddef.h>

namespace test_exports = ::exports::test::variants::to_test;

std::optional<uint8_t> test_exports::RoundtripOption(std::optional<float> a) {
    if (a.has_value()) {
        return std::optional<uint8_t>(a);
    } else {
        return std::optional<uint8_t>();
    }
}

std::expected<double, uint8_t> test_exports::RoundtripResult(std::expected<uint32_t, float> a) {
    if (a.has_value()) {
        return std::expected<double, uint8_t>(double(a.value()));
    } else {
        return std::expected<double, uint8_t>(std::unexpected(uint8_t(a.error())));
    }
}

test_exports::E1 test_exports::RoundtripEnum(test_exports::E1 a) {
    return a;
}

bool test_exports::InvertBool(bool a) {
    return !a;
}

std::tuple<test_exports::C1, test_exports::C2, test_exports::C3, test_exports::C4, test_exports::C5, test_exports::C6> test_exports::VariantCasts(std::tuple<test_exports::C1, test_exports::C2, test_exports::C3, test_exports::C4, test_exports::C5, test_exports::C6> a) {
    return a;
}

std::tuple<test_exports::Z1, test_exports::Z2, test_exports::Z3, test_exports::Z4> test_exports::VariantZeros(std::tuple<test_exports::Z1, test_exports::Z2, test_exports::Z3, test_exports::Z4> a) {
    return a;
}

void test_exports::VariantTypedefs(std::optional<uint32_t> a, bool b, std::expected<uint32_t, wit::Void> c) {

}

std::tuple<bool, std::expected<void, wit::Void>, test_exports::MyErrno> test_exports::VariantEnums(bool a, std::expected<void, wit::Void> b, test_exports::MyErrno c) {
    return std::tuple<bool, std::expected<void, wit::Void>, test_exports::MyErrno>(a, b, c);
}
