#include <assert.h>
#include <limits.h>
#include <runner_cpp.h>

template <class T>
static bool equal(T const& a, T const& b) {
    return a==b;
}
static bool equal(wit::Void a, wit::Void b) {
    return true;
}
static bool equal(::test::variants::to_test::C1 a, ::test::variants::to_test::C1 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::C1::A>(a.variants).value, std::get<::test::variants::to_test::C1::A>(b.variants).value);
        case 1: return equal(std::get<::test::variants::to_test::C1::B>(a.variants).value, std::get<::test::variants::to_test::C1::B>(b.variants).value);
    }
    return false;
}
static bool equal(::test::variants::to_test::C2 a, ::test::variants::to_test::C2 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::C2::A>(a.variants).value, std::get<::test::variants::to_test::C2::A>(b.variants).value);
        case 1: return equal(std::get<::test::variants::to_test::C2::B>(a.variants).value, std::get<::test::variants::to_test::C2::B>(b.variants).value);
    }
    return false;
}
static bool equal(::test::variants::to_test::C3 a, ::test::variants::to_test::C3 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::C3::A>(a.variants).value, std::get<::test::variants::to_test::C3::A>(b.variants).value);
        case 1: return equal(std::get<::test::variants::to_test::C3::B>(a.variants).value, std::get<::test::variants::to_test::C3::B>(b.variants).value);
    }
    return false;
}
static bool equal(::test::variants::to_test::C4 a, ::test::variants::to_test::C4 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::C4::A>(a.variants).value, std::get<::test::variants::to_test::C4::A>(b.variants).value);
        case 1: return equal(std::get<::test::variants::to_test::C4::B>(a.variants).value, std::get<::test::variants::to_test::C4::B>(b.variants).value);
    }
    return false;
}
static bool equal(::test::variants::to_test::C5 a, ::test::variants::to_test::C5 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::C5::A>(a.variants).value, std::get<::test::variants::to_test::C5::A>(b.variants).value);
        case 1: return equal(std::get<::test::variants::to_test::C5::B>(a.variants).value, std::get<::test::variants::to_test::C5::B>(b.variants).value);
    }
    return false;
}
static bool equal(::test::variants::to_test::C6 a, ::test::variants::to_test::C6 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::C6::A>(a.variants).value, std::get<::test::variants::to_test::C6::A>(b.variants).value);
        case 1: return equal(std::get<::test::variants::to_test::C6::B>(a.variants).value, std::get<::test::variants::to_test::C6::B>(b.variants).value);
    }
    return false;
}
static bool equal(::test::variants::to_test::Z1 a, ::test::variants::to_test::Z1 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::Z1::A>(a.variants).value, std::get<::test::variants::to_test::Z1::A>(b.variants).value);
        case 1: return true;
    }
    return false;
}
static bool equal(::test::variants::to_test::Z2 a, ::test::variants::to_test::Z2 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::Z2::A>(a.variants).value, std::get<::test::variants::to_test::Z2::A>(b.variants).value);
        case 1: return true;
    }
    return false;
}
static bool equal(::test::variants::to_test::Z3 a, ::test::variants::to_test::Z3 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::Z3::A>(a.variants).value, std::get<::test::variants::to_test::Z3::A>(b.variants).value);
        case 1: return true;
    }
    return false;
}
static bool equal(::test::variants::to_test::Z4 a, ::test::variants::to_test::Z4 b) {
    if (a.variants.index()!=b.variants.index()) { return false; }
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variants::to_test::Z4::A>(a.variants).value, std::get<::test::variants::to_test::Z4::A>(b.variants).value);
        case 1: return true;
    }
    return false;
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
template <class T>
static bool equal(std::optional<T> const& a, std::optional<T> const& b) {
    if (a.has_value() != b.has_value()) return false;
    if (a.has_value()) {
        return equal(a.value(), b.value());
    }
    return true;
}
template <class A, class B, class C, class D, class E, class F>
static bool equal(std::tuple<A,B,C,D,E,F> a, std::tuple<A,B,C,D,E,F> b) {
    return equal(std::get<0>(a), std::get<0>(b)) &&
        equal(std::get<1>(a), std::get<1>(b)) &&
        equal(std::get<2>(a), std::get<2>(b)) &&
        equal(std::get<3>(a), std::get<3>(b)) &&
        equal(std::get<4>(a), std::get<4>(b)) &&
        equal(std::get<5>(a), std::get<5>(b));
}
template <class A, class B, class C, class D>
static bool equal(std::tuple<A,B,C,D> a, std::tuple<A,B,C,D> b) {
    return equal(std::get<0>(a), std::get<0>(b)) &&
        equal(std::get<1>(a), std::get<1>(b)) &&
        equal(std::get<2>(a), std::get<2>(b)) &&
        equal(std::get<3>(a), std::get<3>(b));
}
template <class A, class B, class C>
static bool equal(std::tuple<A,B,C> a, std::tuple<A,B,C> b) {
    return equal(std::get<0>(a), std::get<0>(b)) &&
        equal(std::get<1>(a), std::get<1>(b)) &&
        equal(std::get<2>(a), std::get<2>(b));
}

int main()
{
    using namespace ::test::variants::to_test;

    assert(equal(RoundtripOption(1.0), std::optional<uint8_t>(1)));
    assert(equal(RoundtripOption(std::optional<float>()), std::optional<uint8_t>()));
    assert(equal(RoundtripOption(2.0), std::optional<uint8_t>(2)));
    assert(equal(RoundtripResult(2), std::expected<double, uint8_t>(2.0)));
    assert(equal(RoundtripResult(4), std::expected<double, uint8_t>(4.0)));
    assert(equal(RoundtripResult(std::unexpected(5.3)), std::expected<double, uint8_t>(std::unexpected(5))));

    assert(equal(RoundtripEnum(E1::kA), E1::kA));
    assert(equal(RoundtripEnum(E1::kB), E1::kB));

    assert(equal(InvertBool(true), false));
    assert(equal(InvertBool(false), true));

    assert(equal(VariantCasts(std::tuple<C1, C2, C3, C4, C5, C6>(C1{C1::A{1}}, C2{C2::A{2}}, C3{C3::A{3}}, C4{C4::A{4}}, C5{C5::A{5}}, C6{C6::A{6.0}})), 
        std::tuple<C1, C2, C3, C4, C5, C6>(C1{C1::A{1}}, C2{C2::A{2}}, C3{C3::A{3}}, C4{C4::A{4}}, C5{C5::A{5}}, C6{C6::A{6.0}}) ));
    assert(equal(VariantCasts(std::tuple<C1, C2, C3, C4, C5, C6>(C1{C1::B{1}}, C2{C2::B{2.0}}, C3{C3::B{3.0}}, C4{C4::B{4.0}}, C5{C5::B{5.0}}, C6{C6::B{6.0}})), 
        std::tuple<C1, C2, C3, C4, C5, C6>(C1{C1::B{1}}, C2{C2::B{2.0}}, C3{C3::B{3.0}}, C4{C4::B{4.0}}, C5{C5::B{5.0}}, C6{C6::B{6.0}}) ));

    assert(equal(VariantZeros(std::tuple<Z1, Z2, Z3, Z4>(Z1{Z1::A{1}}, Z2{Z2::A{2}}, Z3{Z3::A{3.0}}, Z4{Z4::A{4.0}})), std::tuple<Z1, Z2, Z3, Z4>(Z1{Z1::A{1}}, Z2{Z2::A{2}}, Z3{Z3::A{3.0}}, Z4{Z4::A{4.0}})));
    assert(equal(VariantZeros(std::tuple<Z1, Z2, Z3, Z4>(Z1{Z1::B{}}, Z2{Z2::B{}}, Z3{Z3::B{}}, Z4{Z4::B{}})), std::tuple<Z1, Z2, Z3, Z4>(Z1{Z1::B{}}, Z2{Z2::B{}}, Z3{Z3::B{}}, Z4{Z4::B{}})));

    VariantTypedefs(std::optional<uint32_t>(), false, std::unexpected(wit::Void{}));

    assert(equal(VariantEnums(true, std::expected<void, wit::Void>(), MyErrno::kSuccess)
    , std::tuple<bool, std::expected<void, wit::Void>, MyErrno>(true, std::expected<void, wit::Void>(), MyErrno::kSuccess)));
}
