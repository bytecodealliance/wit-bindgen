#include <assert.h>
#include <strings.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

char16_t STR_BUFFER[500];

void assert_str(strings_string_t* str, char16_t* expected) {
  size_t expected_len = 0;
  while (expected[expected_len])
    expected_len++;
  assert(str->len == expected_len);
  assert(memcmp(str->ptr, expected, expected_len * 2) == 0);
}

void strings_test_imports() {
  strings_string_t str1;
  strings_string_set(&str1, u"latin utf16");
  imports_take_basic(&str1);

  strings_string_t str2;
  imports_return_unicode(&str2);
  assert_str(&str2, u"🚀🚀🚀 𠈄𓀀");
  strings_string_free(&str2);
}

void strings_return_empty(strings_string_t *ret) {
  strings_string_dup(ret, u""); // Exercise cabi_realloc new_size = 0
}

void strings_roundtrip(strings_string_t *str, strings_string_t *ret) {
  assert(str->len > 0);
  ret->len = str->len;
  ret->ptr = malloc(ret->len * 2);
  memcpy(ret->ptr, str->ptr, 2 * ret->len);
  strings_string_free(str);
}
