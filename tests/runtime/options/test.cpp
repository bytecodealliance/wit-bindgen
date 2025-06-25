#include <assert.h>
#include <limits.h>
#include <math.h>
#include <test_cpp.h>

static bool equal(std::optional<wit::string> const& a, std::optional<std::string_view> const& b) {
    return a.has_value() == b.has_value() && a->get_view()==b.value();
}

void exports::test::options::to_test::OptionNoneParam(std::optional<wit::string> a)
{
    assert(!a.has_value());
}

std::optional<wit::string> exports::test::options::to_test::OptionNoneResult() {
    return std::optional<wit::string>();
}

void exports::test::options::to_test::OptionSomeParam(std::optional<wit::string> a) {
    assert(equal(a, std::optional<std::string_view>("foo")));
}

std::optional<wit::string> exports::test::options::to_test::OptionSomeResult() {
    return std::optional<wit::string>(wit::string::from_view("foo"));
}

std::optional<wit::string> exports::test::options::to_test::OptionRoundtrip(std::optional<wit::string> a) {
    return a;
}

std::optional<std::optional<uint32_t>> exports::test::options::to_test::DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>> a) {
    return a;
}
