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

    // assert!(matches!(
    //     variant_error(0.0),
    //     Err(E3::E2(E2 {
    //         line: 420,
    //         column: 0
    //     }))
    // ));
    // assert!(matches!(variant_error(1.0), Err(E3::E1(E::B))));
    // assert!(matches!(variant_error(2.0), Err(E3::E1(E::C))));

    // assert_eq!(empty_error(0), Err(()));
    // assert_eq!(empty_error(1), Ok(42));
    // assert_eq!(empty_error(2), Ok(2));

    // assert_eq!(double_error(0), Ok(Ok(())));
    // assert_eq!(double_error(1), Ok(Err("one".into())));
    // assert_eq!(double_error(2), Err("two".into()));
}
