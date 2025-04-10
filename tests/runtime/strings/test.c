//@ args = '--string-encoding utf16'

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include "test.h"

char16_t STR_BUFFER[500];

void assert_str(test_string_t* str, const char16_t* expected) {
  size_t expected_len = 0;
  while (expected[expected_len])
    expected_len++;
  assert(str->len == expected_len);
  assert(memcmp(str->ptr, expected, expected_len * 2) == 0);
}

void exports_test_strings_to_test_take_basic(test_string_t *str1) {
  assert_str(str1, u"latin utf16");
  test_string_free(str1);
}

void exports_test_strings_to_test_return_unicode(test_string_t *ret) {
  test_string_dup(ret, u"🚀🚀🚀 𠈄𓀀");
}

void exports_test_strings_to_test_return_empty(test_string_t *ret) {
  test_string_dup(ret, u""); // Exercise cabi_realloc new_size = 0
}

void exports_test_strings_to_test_roundtrip(test_string_t *str, test_string_t *ret) {
  assert(str->len > 0);
  ret->len = str->len;
  ret->ptr = (uint16_t *) malloc(ret->len * 2);
  memcpy(ret->ptr, str->ptr, 2 * ret->len);
  test_string_free(str);
}
