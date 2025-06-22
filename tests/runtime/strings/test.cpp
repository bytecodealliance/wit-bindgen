#include <assert.h>
#include <test_cpp.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

void assert_str(std::string_view const& str, const char* expected) {
  size_t expected_len = strlen(expected);
  assert(str.size() == expected_len);
  assert(memcmp(str.data(), expected, expected_len) == 0);
}

void exports::test::strings::to_test::TakeBasic(wit::string str) {
  assert_str(str.get_view(), "latin utf16");
}

wit::string exports::test::strings::to_test::ReturnUnicode() {
  // return a non-zero address (follows cabi_realloc logic)
  return wit::string::from_view("🚀🚀🚀 𠈄𓀀");
}

wit::string exports::test::strings::to_test::ReturnEmpty() {
  // return a non-zero address (follows cabi_realloc logic)
  return wit::string((char const*)1, 0);
}

wit::string exports::test::strings::to_test::Roundtrip(wit::string str) {
  return str;
}
