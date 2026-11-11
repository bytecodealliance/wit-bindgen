//@ wasmtime-flags = '-Wcomponent-model-accessors'

#include <assert.h>
#include <string.h>
#include <runner.h>

static void assert_str(runner_string_t *str, const char *expected) {
    size_t expected_len = strlen(expected);
    assert(str->len == expected_len);
    assert(memcmp(str->ptr, expected, expected_len) == 0);
}

void exports_runner_run(void) {
    runner_string_t str;
    runner_string_t err;

    assert(test_accessors_i_get_counter() == 0);
    test_accessors_i_set_counter(5);
    assert(test_accessors_i_get_counter() == 5);
    test_accessors_i_set_counter(UINT32_MAX);
    assert(test_accessors_i_get_counter() == UINT32_MAX);

    test_accessors_i_get_read_only(&str);
    assert_str(&str, "read only");
    runner_string_free(&str);

    assert(test_accessors_i_get_bounded() == 0);
    assert(test_accessors_i_set_bounded(10, &err));
    assert(test_accessors_i_get_bounded() == 10);
    assert(!test_accessors_i_set_bounded(101, &err));
    assert_str(&err, "101 is out of bounds");
    runner_string_free(&err);
    assert(test_accessors_i_get_bounded() == 10);

    uint8_t a_init[] = {1, 2, 3};
    uint8_t b_init[] = {4, 5};
    runner_list_u8_t a_list = {a_init, 3};
    runner_list_u8_t b_list = {b_init, 2};
    test_accessors_i_own_blob_t a_own = test_accessors_i_constructor_blob(&a_list);
    test_accessors_i_own_blob_t b_own = test_accessors_i_constructor_blob(&b_list);
    test_accessors_i_borrow_blob_t a = test_accessors_i_borrow_blob(a_own);
    test_accessors_i_borrow_blob_t b = test_accessors_i_borrow_blob(b_own);

    runner_list_u8_t contents;
    test_accessors_i_method_blob_contents(a, &contents);
    assert(contents.len == 3);
    assert(memcmp(contents.ptr, a_init, 3) == 0);
    runner_list_u8_free(&contents);

    assert(test_accessors_i_method_get_blob_position(a) == 0);
    assert(test_accessors_i_method_get_blob_position(b) == 0);
    test_accessors_i_method_set_blob_position(a, 2);
    assert(test_accessors_i_method_get_blob_position(a) == 2);
    assert(test_accessors_i_method_get_blob_position(b) == 0);
    test_accessors_i_method_set_blob_position(b, 1);
    assert(test_accessors_i_method_get_blob_position(a) == 2);
    assert(test_accessors_i_method_get_blob_position(b) == 1);

    test_accessors_i_method_get_blob_label(a, &str);
    assert_str(&str, "");
    runner_string_free(&str);

    runner_string_t label;
    runner_string_set(&label, "hello");
    assert(test_accessors_i_method_set_blob_label(a, &label, &err));
    test_accessors_i_method_get_blob_label(a, &str);
    assert_str(&str, "hello");
    runner_string_free(&str);
    test_accessors_i_method_get_blob_label(b, &str);
    assert_str(&str, "");
    runner_string_free(&str);

    runner_string_set(&label, "");
    assert(!test_accessors_i_method_set_blob_label(a, &label, &err));
    assert_str(&err, "label must not be empty");
    runner_string_free(&err);
    test_accessors_i_method_get_blob_label(a, &str);
    assert_str(&str, "hello");
    runner_string_free(&str);

    assert(test_accessors_i_static_get_blob_max_size() == 1024);
    assert(test_accessors_i_static_set_blob_max_size(2048, &err));
    assert(test_accessors_i_static_get_blob_max_size() == 2048);
    assert(!test_accessors_i_static_set_blob_max_size(0, &err));
    assert_str(&err, "max size must be nonzero");
    runner_string_free(&err);
    assert(test_accessors_i_static_get_blob_max_size() == 2048);

    test_accessors_i_blob_drop_own(a_own);
    test_accessors_i_blob_drop_own(b_own);
}
