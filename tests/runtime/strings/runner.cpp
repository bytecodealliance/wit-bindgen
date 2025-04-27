//@ args = '--new-api'

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <runner_cpp.h>

void assert_str(std::string_view str, const char* expected) {
  size_t expected_len = strlen(expected);
  assert(str.size() == expected_len);
  assert(memcmp(str.data(), expected, expected_len) == 0);
}

int main() {
    test::strings::to_test::take_basic("latin utf16")

    return 0;
}
