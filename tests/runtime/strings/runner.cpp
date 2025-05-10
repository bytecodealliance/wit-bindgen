//@ args = '--new-api'

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <runner_cpp.h>

void assert_str(wit::string const& str, const char* expected) {
  size_t expected_len = strlen(expected);
  assert(str.size() == expected_len);
  assert(memcmp(str.data(), expected, expected_len) == 0);
}

int main() {
    test::strings::to_test::TakeBasic("latin utf16");

    auto str2 = test::strings::to_test::ReturnUnicode();
    assert_str(str2, "🚀🚀🚀 𠈄𓀀");

    auto str3 = test::strings::to_test::ReturnEmpty();
    assert_str(str3, "");

    auto str5 = test::strings::to_test::Roundtrip("🚀🚀🚀 𠈄𓀀");
    assert_str(str5, "🚀🚀🚀 𠈄𓀀");
    
    return 0;
}
