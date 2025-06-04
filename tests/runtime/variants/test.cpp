#include <assert.h>
#include <test_cpp.h>
#include <stddef.h>

std::optional<uint8_t> exports::test::variants::to_test::RoundtripOption(std::optional<float> a) {
    if (a.has_value()) {
        return std::optional<uint8_t>(a);
    } else {
        return std::optional<uint8_t>();
    }
}

std::expected<double, uint8_t> exports::test::variants::to_test::RoundtripResult(std::expected<uint32_t, float> a) {
    if (a.has_value()) {
        return std::expected<double, uint8_t>(double(a.value()));
    } else {
        return std::expected<double, uint8_t>(std::unexpected(uint8_t(a.error())));
    }
}

::test::variants::to_test::E1 exports::test::variants::to_test::RoundtripEnum(::test::variants::to_test::E1 a) {
    return a;
}

bool exports::test::variants::to_test::InvertBool(bool a) {
    return !a;
}

std::tuple<::test::variants::to_test::C1, ::test::variants::to_test::C2, ::test::variants::to_test::C3, ::test::variants::to_test::C4, ::test::variants::to_test::C5, ::test::variants::to_test::C6> exports::test::variants::to_test::VariantCasts(std::tuple<::test::variants::to_test::C1, ::test::variants::to_test::C2, ::test::variants::to_test::C3, ::test::variants::to_test::C4, ::test::variants::to_test::C5, ::test::variants::to_test::C6> a) {
    return a;
}

std::tuple<::test::variants::to_test::Z1, ::test::variants::to_test::Z2, ::test::variants::to_test::Z3, ::test::variants::to_test::Z4> exports::test::variants::to_test::VariantZeros(std::tuple<::test::variants::to_test::Z1, ::test::variants::to_test::Z2, ::test::variants::to_test::Z3, ::test::variants::to_test::Z4> a) {
    return a;
}

void exports::test::variants::to_test::VariantTypedefs(std::optional<uint32_t> a, bool b, std::expected<uint32_t, wit::Void> c) {

}

std::tuple<bool, std::expected<void, wit::Void>, ::test::variants::to_test::MyErrno> exports::test::variants::to_test::VariantEnums(bool a, std::expected<void, wit::Void> b, ::test::variants::to_test::MyErrno c) {
    return std::tuple<bool, std::expected<void, wit::Void>, ::test::variants::to_test::MyErrno>(a, b, c);
}
