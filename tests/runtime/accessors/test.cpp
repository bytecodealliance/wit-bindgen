#include <string>
#include <test_cpp.h>

namespace test_exports = ::exports::test::accessors::i;

static uint32_t counter = 0;
static uint32_t bounded = 0;

uint32_t test_exports::Counter() {
    return counter;
}

void test_exports::SetCounter(uint32_t value) {
    counter = value;
}

wit::string test_exports::ReadOnly() {
    return wit::string::from_view("read only");
}

uint32_t test_exports::Bounded() {
    return bounded;
}

std::expected<void, wit::string> test_exports::SetBounded(uint32_t value) {
    if (value > 100) {
        return std::unexpected(wit::string::from_view(std::to_string(value) + " is out of bounds"));
    }
    bounded = value;
    return std::expected<void, wit::string>();
}
