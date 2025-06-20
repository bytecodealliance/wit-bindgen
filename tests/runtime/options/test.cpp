#include <assert.h>
#include <limits.h>
#include <math.h>
#include <test_cpp.h>

template <class T>
static bool equal(T const& a, T const& b) {
    return a==b;
}

void exports::test::options::to_test::OptionNoneParam(std::optional<std::string_view> a)
{
    assert(!a.has_value());
}

std::optional<wit::string> exports::test::options::to_test::OptionNoneResult() {
    return std::optional<wit::string>();
}

void exports::test::options::to_test::OptionSomeParam(std::optional<std::string_view> a) {
    assert(equal(a, std::optional<std::string_view>("foo")));
}

std::optional<wit::string> exports::test::options::to_test::OptionSomeResult() {
    return std::optional<wit::string>(wit::string::from_view("foo"));
}

std::optional<wit::string> exports::test::options::to_test::OptionRoundtrip(std::optional<std::string_view> a) {
    if (!a.has_value()) return std::optional<wit::string>();
    return std::optional<wit::string>(wit::string::from_view(*a));
}

std::optional<std::optional<uint32_t>> exports::test::options::to_test::DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>> a) {
    return a;
}
