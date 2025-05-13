#include <assert.h>
#include <limits.h>
#include <runner_cpp.h>

template <class T>
static bool equal(T const& a, T const& b) {
    return a==b;
}
static bool equal(wit::string const& a, std::string const& b) {
    return a.get_view() == std::string_view(b);
}
static bool equal(std::optional<wit::string> const& a, std::optional<std::string> const& b) {
    if (a.has_value() != b.has_value()) return false;
    if (a.has_value()) {
        return equal(a.value(), b.value());
    }
    return true;
}

int main()
{
    using namespace ::test::results::to_test;

    assert(equal(string_error(0.0), std::unexpected("zero")));
    assert(equal(string_error(1.0), std::expected<>(1.0));

    assert_eq!(enum_error(0.0), Err(E::A));
    assert_eq!(enum_error(1.0), Ok(1.0));

    assert!(matches!(
        record_error(0.0),
        Err(E2 {
            line: 420,
            column: 0
        })
    ));
    assert!(matches!(
        record_error(1.0),
        Err(E2 {
            line: 77,
            column: 2
        })
    ));
    assert!(record_error(2.0).is_ok());

    assert!(matches!(
        variant_error(0.0),
        Err(E3::E2(E2 {
            line: 420,
            column: 0
        }))
    ));
    assert!(matches!(variant_error(1.0), Err(E3::E1(E::B))));
    assert!(matches!(variant_error(2.0), Err(E3::E1(E::C))));

    assert_eq!(empty_error(0), Err(()));
    assert_eq!(empty_error(1), Ok(42));
    assert_eq!(empty_error(2), Ok(2));

    assert_eq!(double_error(0), Ok(Ok(())));
    assert_eq!(double_error(1), Ok(Err("one".into())));
    assert_eq!(double_error(2), Err("two".into()));

    OptionNoneParam(std::optional<std::string_view>());
    OptionSomeParam(std::optional<std::string_view>("foo"));
    assert(!OptionNoneResult());
    assert(equal(OptionSomeResult(), std::optional<std::string>("foo")));
    assert(equal(OptionRoundtrip(std::optional<std::string_view>("foo")), std::optional<std::string>("foo")));
    assert(equal(DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>>(std::optional<uint32_t>(42))), std::optional<std::optional<uint32_t>>(std::optional<uint32_t>(42))));
    assert(equal(DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>>(std::optional<uint32_t>())), std::optional<std::optional<uint32_t>>(std::optional<uint32_t>())));
    assert(equal(DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>>()), std::optional<std::optional<uint32_t>>()));
}
