//@ args = '--string-encoding utf16'

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include "runner.h"


char16_t STR_BUFFER[500];

void assert_str(runner_string_t* str, const char16_t* expected) {
    size_t expected_len = 0;
    while (expected[expected_len])
      expected_len++;
    assert(str->len == expected_len);
    assert(memcmp(str->ptr, expected, expected_len * 2) == 0);
  }

int main() {
    runner_string_t str1;
    runner_string_set(&str1, u"latin utf16");
    test_strings_to_test_take_basic(&str1);

    runner_string_t str2;
    test_strings_to_test_return_unicode(&str2);
    assert_str(&str2, u"🚀🚀🚀 𠈄𓀀");
    runner_string_free(&str2);

    runner_string_t str3;
    test_strings_to_test_return_empty(&str3);
    assert_str(&str3, u"");
    runner_string_free(&str3);

    runner_string_t str4;
    runner_string_t str5;
    runner_string_set(&str4, u"🚀🚀🚀 𠈄𓀀");
    test_strings_to_test_roundtrip(&str4, &str5);
    assert_str(&str5, u"🚀🚀🚀 𠈄𓀀");
    runner_string_free(&str5);
}
