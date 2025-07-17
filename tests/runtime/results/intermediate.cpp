#include <assert.h>
#include <intermediate_cpp.h>

template <class T>
bool equal(T const&a, T const&b) {
    return a==b;
}

namespace test_imports = ::test::results::test;
namespace test_exports = ::exports::test::results::test;

std::expected<float, wit::string> test_exports::StringError(float a) {
    return test_imports::StringError(a);
}

test_exports::E to_exports_e(test_imports::E e) {
    switch (e) {
        case test_imports::E::kA: return test_exports::E::kA;
        case test_imports::E::kB: return test_exports::E::kB;
        case test_imports::E::kC: return test_exports::E::kC;
    }
}

std::expected<float, test_exports::E> test_exports::EnumError(float a) {
    auto result = test_imports::EnumError(a);
    if (result.has_value()) { return result.value(); }
    return std::unexpected(to_exports_e(result.error()));
}

std::expected<float, test_exports::E2> test_exports::RecordError(float a) {
    auto result = test_imports::RecordError(a);
    if (result.has_value()) { return result.value(); }
    return std::unexpected(test_exports::E2{ result.error().line, result.error().column });
}

template <class... Fs>
struct overloaded : Fs... {
    using Fs::operator()...;
};
template <class... Fs>
overloaded(Fs...) -> overloaded<Fs...>;

std::expected<float, test_exports::E3> test_exports::VariantError(float a) {
    auto result = test_imports::VariantError(a);
    if (result.has_value()) { return result.value(); }
    return std::visit(overloaded{
        [](test_imports::E3::E1 const& e1) { return std::unexpected(test_exports::E3{test_exports::E3::E1{to_exports_e(e1.value)}}); },
        [](test_imports::E3::E2 const& e2) { return std::unexpected(test_exports::E3{test_exports::E3::E2{e2.value.line, e2.value.column}}); }
    }, result.error().variants);
}

std::expected<uint32_t, wit::Void> test_exports::EmptyError(uint32_t a) {
    return test_imports::EmptyError(a);
}

std::expected<std::expected<void, wit::string>, wit::string> test_exports::DoubleError(uint32_t a) {
    return test_imports::DoubleError(a);
}
