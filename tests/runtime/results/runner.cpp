#include <assert.h>
#include <limits.h>
#include <runner_cpp.h>

template <class T>
static bool equal(T const& a, T const& b) {
    return a==b;
}
static bool equal(::test::results::test::E2 a, ::test::results::test::E2 b) {
    return a.line==b.line && a.column==b.column;
}
static bool equal(::test::results::test::E3 a, ::test::results::test::E3 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::results::test::E3::E1>(a.variants).value, std::get<::test::results::test::E3::E1>(b.variants).value);
        case 1: return equal(std::get<::test::results::test::E3::E2>(a.variants).value, std::get<::test::results::test::E3::E2>(b.variants).value);
    }
    return false;
}
static bool equal(wit::Void a, wit::Void b) {
    return true;
}
template <class T, class U>
static bool equal(std::expected<T, U> const& a, std::expected<T, U> const& b) {
    if (a.has_value()) {
        if (!b.has_value()) return false;
        return equal(*a, *b);
    } else {
        if (b.has_value()) return false;
        return equal(a.error(), b.error());
    }
}
template <class U>
static bool equal(std::expected<void, U> const& a, std::expected<void, U> const& b) {
    if (a.has_value()) {
        if (!b.has_value()) return false;
        return true;
    } else {
        if (b.has_value()) return false;
        return equal(a.error(), b.error());
    }
}
static bool equal(std::expected<void, wit::string> const& a, std::expected<void, std::string_view> const& b) {
    if (a.has_value()) {
        if (!b.has_value()) return false;
        return true;
    } else {
        if (b.has_value()) return false;
        return equal(a.error().get_view(), b.error());
    }
}
template <class T>
static bool equal(std::expected<T, wit::string> const& a, std::expected<T, std::string_view> const& b) {
    if (a.has_value()) {
        if (!b.has_value()) return false;
        return equal(*a, *b);
    } else {
        if (b.has_value()) return false;
        return equal(a.error().get_view(), b.error());
    }
}
static bool equal(std::expected<std::expected<void, wit::string>, wit::string> const& a, std::expected<std::expected<void, std::string_view>, std::string_view> const& b) {
    if (a.has_value()) {
        if (!b.has_value()) return false;
        return equal(*a, *b);
    } else {
        if (b.has_value()) return false;
        return equal(a.error().get_view(), b.error());
    }
}

int main()
{
    using namespace ::test::results::test;

    assert(equal(StringError(0.0), std::expected<float, std::string_view>(std::unexpected("zero"))));
    assert(equal(StringError(1.0), std::expected<float, std::string_view>(1.0)));

    assert(equal(EnumError(0.0), std::expected<float, E>(std::unexpected(E::kA))));
    assert(equal(EnumError(1.0), std::expected<float, E>(1.0)));

    assert(equal(RecordError(0.0), std::expected<float, E2>(std::unexpected(E2{420,0}))));
    assert(equal(RecordError(1.0), std::expected<float, E2>(std::unexpected(E2{77,2}))));
    assert(equal(RecordError(2.0), std::expected<float, E2>(2.0)));

    assert(equal(VariantError(0.0), std::expected<float, E3>(std::unexpected(E3{E3::E2{E2{420,0}}}))));
    assert(equal(VariantError(1.0), std::expected<float, E3>(std::unexpected(E3{E3::E1{E::kB}}))));
    assert(equal(VariantError(2.0), std::expected<float, E3>(std::unexpected(E3{E3::E1{E::kC}}))));

    assert(equal(EmptyError(0), std::expected<uint32_t, wit::Void>(std::unexpected(wit::Void{}))));
    assert(equal(EmptyError(1), std::expected<uint32_t, wit::Void>(42)));
    assert(equal(EmptyError(2), std::expected<uint32_t, wit::Void>(2)));

    assert(equal(DoubleError(0), std::expected<std::expected<void, std::string_view>, std::string_view>(std::expected<void, std::string_view>())));
    assert(equal(DoubleError(1), std::expected<std::expected<void, std::string_view>, std::string_view>(std::expected<void, std::string_view>(std::unexpected("one")))));
    assert(equal(DoubleError(2), std::expected<std::expected<void, std::string_view>, std::string_view>(std::unexpected("two"))));
}
