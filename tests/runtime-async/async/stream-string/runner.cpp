#include <runner_cpp.h>
#include "module_cpp.h"
#include <assert.h>

static constexpr uint32_t SIZE = 5;
static const char *(expected)[SIZE] = { 
    "Hello", "World!", "From", "a", "stream."
};
static uint32_t next = 0;

static bool equal(wit::string const& a, char const* b) {
    if (a.size()!=strlen(b)) return false;
    return !memcmp(a.data(), b, a.size());
}

int main() {
    wit::stream<wit::string> stream = a::b::the_test::F();
    stream.buffering(1);

    std::move(stream).set_reader([](wit::span<wit::string> data) {
        if (data.size() > 0) {
            assert(data.size()==1);
            assert(next<SIZE);
            assert(equal(data[0], expected[next]));
            ++next;
        }
    });
    symmetric::runtime::symmetric_executor::Run();
    return 0;
}
