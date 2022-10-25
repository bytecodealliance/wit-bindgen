#include <assert.h>
#include <imports.h>
#include <exports.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

char16_t BASIC_STRING[] = { 'l', 'a', 't', 'i', 'n', ' ', 'u', 't', 'f', '1', '6', '\0' };
char16_t STR_BUFFER[500];

void exports_test_imports() {
  imports_string_t str1;
  imports_string_set(&str1, BASIC_STRING);
  imports_f1(&str1);
  imports_string_t str2;
  imports_f2(&str2);
  memcpy(&STR_BUFFER[0], str2.ptr, str2.len * 2);
  STR_BUFFER[str2.len] = '\0';
}

void exports_f1(exports_string_t *str) {
  memcpy(&STR_BUFFER[0], str->ptr, str->len * 2);
  STR_BUFFER[str->len] = '\0';
}

void exports_f2(exports_string_t *ret) {
  exports_string_set(ret, STR_BUFFER);
}
