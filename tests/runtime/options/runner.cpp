#include <assert.h>
#include <limits.h>
#include <runner_cpp.h>

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
