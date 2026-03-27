//@ args = '--string-encoding utf16'
//@ [lang]
//@ cflags = '-Wno-c++-compat'

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

void exports_runner_run() {
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

    // Basic substring extraction
    runner_string_t str6;
    const char16_t *source = u"hello world";
    runner_string_dup_n(&str6, source, 5);
    assert(str6.len == 5);
    assert(memcmp(str6.ptr, u"hello", 5 * 2) == 0);
    runner_string_free(&str6);

    // Zero length (edge case - boundary condition)
    runner_string_t str7;
    runner_string_dup_n(&str7, u"test", 0);
    assert(str7.len == 0);
    runner_string_free(&str7);

    // Full string length
    runner_string_t str8;
    const char16_t *full_str = u"complete";
    size_t full_len = 8;
    runner_string_dup_n(&str8, full_str, full_len);
    assert(str8.len == full_len);
    assert(memcmp(str8.ptr, full_str, full_len * 2) == 0);
    runner_string_free(&str8);

    // Substring from middle (pointer offset)
    runner_string_t str9;
    const char16_t *middle_source = u"prefix_target_suffix";
    runner_string_dup_n(&str9, middle_source + 7, 6);
    assert(str9.len == 6);
    assert(memcmp(str9.ptr, u"target", 6 * 2) == 0);
    runner_string_free(&str9);

    // Unicode content with explicit length
    runner_string_t str10;
    const char16_t *unicode_src = u"🚀🚀🚀 test";
    // Each rocket emoji is 2 UTF-16 code units (surrogate pair), space is 1, "test" is 4
    // Total: 6 + 1 + 4 = 11 code units, extract first 7 (3 rockets + space)
    runner_string_dup_n(&str10, unicode_src, 7);
    assert(str10.len == 7);
    assert(memcmp(str10.ptr, u"🚀🚀🚀 ", 7 * 2) == 0);
    runner_string_free(&str10);

    // Single character
    runner_string_t str11;
    runner_string_dup_n(&str11, u"x", 1);
    assert(str11.len == 1);
    assert(str11.ptr[0] == u'x');
    runner_string_free(&str11);

    // Verify data independence (modification doesn't affect original)
    runner_string_t str12;
    char16_t mutable_src[] = u"original";
    runner_string_dup_n(&str12, mutable_src, 8);
    mutable_src[0] = u'X';
    assert(str12.ptr[0] == u'o');
    runner_string_free(&str12);
}
