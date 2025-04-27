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
    test::strings::to_test::take_basic("latin utf16");

    let str2 = test::strings::to_test::return_unicode();
    assert_str(str2, "🚀🚀🚀 𠈄𓀀");

    let str3 = test::strings::to_test::return_empty();
    assert_str(str3, "");

    let str5 = test::strings::to_test::roundtrip("🚀🚀🚀 𠈄𓀀");
    assert_str(str5, "🚀🚀🚀 𠈄𓀀");
    
    return 0;
}
