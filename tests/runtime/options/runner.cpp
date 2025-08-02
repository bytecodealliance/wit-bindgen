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
    using namespace ::test::options::to_test;

    OptionNoneParam(std::optional<std::string_view>());
    OptionSomeParam(std::optional<std::string_view>("foo"));
    assert(!OptionNoneResult());
    assert(equal(OptionSomeResult(), std::optional<std::string>("foo")));
    assert(equal(OptionRoundtrip(std::optional<std::string_view>("foo")), std::optional<std::string>("foo")));
    assert(equal(DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>>(std::optional<uint32_t>(42))), std::optional<std::optional<uint32_t>>(std::optional<uint32_t>(42))));
    assert(equal(DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>>(std::optional<uint32_t>())), std::optional<std::optional<uint32_t>>(std::optional<uint32_t>())));
    assert(equal(DoubleOptionRoundtrip(std::optional<std::optional<uint32_t>>()), std::optional<std::optional<uint32_t>>()));
}
