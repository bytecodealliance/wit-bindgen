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

uint64_t test_exports::Blob::max_size = 1024;

test_exports::Blob::Blob(wit::vector<uint8_t> init) : position(0) {
    auto view = init.get_const_view();
    contents.assign(view.begin(), view.end());
}

wit::vector<uint8_t> test_exports::Blob::Contents() {
    return wit::vector<uint8_t>::from_view(std::span<const uint8_t>(contents));
}

uint64_t test_exports::Blob::Position() {
    return position;
}

void test_exports::Blob::SetPosition(uint64_t value) {
    position = value;
}

wit::string test_exports::Blob::Label() {
    return wit::string::from_view(label);
}

std::expected<void, wit::string> test_exports::Blob::SetLabel(wit::string value) {
    if (value.size() == 0) {
        return std::unexpected(wit::string::from_view("label must not be empty"));
    }
    label = value.to_string();
    return std::expected<void, wit::string>();
}

uint64_t test_exports::Blob::MaxSize() {
    return max_size;
}

std::expected<void, wit::string> test_exports::Blob::SetMaxSize(uint64_t value) {
    if (value == 0) {
        return std::unexpected(wit::string::from_view("max size must be nonzero"));
    }
    max_size = value;
    return std::expected<void, wit::string>();
}
