#include <runner_cpp.h>

void assert_str(wit::string const& str, const char* expected) {
  size_t expected_len = strlen(expected);
  assert(str.size() == expected_len);
  assert(memcmp(str.data(), expected, expected_len) == 0);
}

int main() {
    std::future<wit::string> f = a::b::the_test::F();
    wit::string s = f.get();
    assert_str(s, "Hello");
    return 0;
}
