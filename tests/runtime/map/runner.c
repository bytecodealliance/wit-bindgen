//@ wasmtime-flags = '-Wcomponent-model-map'

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "runner.h"

static bool string_eq(const runner_string_t *s, const char *lit) {
  size_t len = strlen(lit);
  return s->len == len && memcmp(s->ptr, lit, len) == 0;
}

static void test_named_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t entries[2];
  entries[0].key = 1;
  runner_string_dup(&entries[0].value, "uno");
  entries[1].key = 2;
  runner_string_dup(&entries[1].value, "two");
  test_maps_to_test_names_by_id_t input = {.ptr = entries, .len = 2};

  test_maps_to_test_ids_by_name_t result;
  test_maps_to_test_named_roundtrip(&input, &result);

  assert(result.len == 2);
  bool saw_uno = false, saw_two = false;
  for (size_t i = 0; i < result.len; i++) {
    if (string_eq(&result.ptr[i].key, "uno")) {
      assert(result.ptr[i].value == 1);
      saw_uno = true;
    } else if (string_eq(&result.ptr[i].key, "two")) {
      assert(result.ptr[i].value == 2);
      saw_two = true;
    } else {
      assert(0 && "unexpected key");
    }
  }
  assert(saw_uno && saw_two);
  test_maps_to_test_ids_by_name_free(&result);
}

static void test_bytes_roundtrip(void) {
  test_maps_to_test_bytes_by_name_entry_t entries[2];
  runner_string_dup(&entries[0].key, "hello");
  uint8_t world_bytes[] = {'w', 'o', 'r', 'l', 'd'};
  entries[0].value.ptr = (uint8_t *)malloc(sizeof(world_bytes));
  memcpy(entries[0].value.ptr, world_bytes, sizeof(world_bytes));
  entries[0].value.len = sizeof(world_bytes);

  runner_string_dup(&entries[1].key, "bin");
  uint8_t bin_bytes[] = {0, 1, 2};
  entries[1].value.ptr = (uint8_t *)malloc(sizeof(bin_bytes));
  memcpy(entries[1].value.ptr, bin_bytes, sizeof(bin_bytes));
  entries[1].value.len = sizeof(bin_bytes);

  test_maps_to_test_bytes_by_name_t input = {.ptr = entries, .len = 2};
  test_maps_to_test_bytes_by_name_t result;
  test_maps_to_test_bytes_roundtrip(&input, &result);

  assert(result.len == 2);
  for (size_t i = 0; i < result.len; i++) {
    if (string_eq(&result.ptr[i].key, "hello")) {
      assert(result.ptr[i].value.len == 5);
      assert(memcmp(result.ptr[i].value.ptr, "world", 5) == 0);
    } else if (string_eq(&result.ptr[i].key, "bin")) {
      assert(result.ptr[i].value.len == 3);
      uint8_t expected[] = {0, 1, 2};
      assert(memcmp(result.ptr[i].value.ptr, expected, 3) == 0);
    } else {
      assert(0 && "unexpected key");
    }
  }
  test_maps_to_test_bytes_by_name_free(&result);
}

static void test_empty_roundtrip(void) {
  test_maps_to_test_names_by_id_t input = {.ptr = NULL, .len = 0};
  test_maps_to_test_names_by_id_t result;
  test_maps_to_test_empty_roundtrip(&input, &result);
  assert(result.len == 0);
  test_maps_to_test_names_by_id_free(&result);
}

static void test_option_roundtrip(void) {
  runner_map_string_option_u32_entry_t entries[2];
  runner_string_dup(&entries[0].key, "some");
  entries[0].value.is_some = true;
  entries[0].value.val = 42;
  runner_string_dup(&entries[1].key, "none");
  entries[1].value.is_some = false;
  entries[1].value.val = 0;

  runner_map_string_option_u32_t input = {.ptr = entries, .len = 2};
  runner_map_string_option_u32_t result;
  test_maps_to_test_option_roundtrip(&input, &result);

  assert(result.len == 2);
  bool saw_some = false, saw_none = false;
  for (size_t i = 0; i < result.len; i++) {
    if (string_eq(&result.ptr[i].key, "some")) {
      assert(result.ptr[i].value.is_some);
      assert(result.ptr[i].value.val == 42);
      saw_some = true;
    } else if (string_eq(&result.ptr[i].key, "none")) {
      assert(!result.ptr[i].value.is_some);
      saw_none = true;
    }
  }
  assert(saw_some && saw_none);
  runner_map_string_option_u32_free(&result);
}

static void test_record_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t values[2];
  values[0].key = 10;
  runner_string_dup(&values[0].value, "ten");
  values[1].key = 20;
  runner_string_dup(&values[1].value, "twenty");

  test_maps_to_test_labeled_entry_t input;
  runner_string_dup(&input.label, "test-label");
  input.values.ptr = values;
  input.values.len = 2;

  test_maps_to_test_labeled_entry_t result;
  test_maps_to_test_record_roundtrip(&input, &result);

  assert(string_eq(&result.label, "test-label"));
  assert(result.values.len == 2);
  for (size_t i = 0; i < result.values.len; i++) {
    if (result.values.ptr[i].key == 10) {
      assert(string_eq(&result.values.ptr[i].value, "ten"));
    } else if (result.values.ptr[i].key == 20) {
      assert(string_eq(&result.values.ptr[i].value, "twenty"));
    } else {
      assert(0 && "unexpected key");
    }
  }
  test_maps_to_test_labeled_entry_free(&result);
}

static void test_inline_roundtrip(void) {
  runner_map_u32_string_entry_t entries[2];
  entries[0].key = 1;
  runner_string_dup(&entries[0].value, "one");
  entries[1].key = 2;
  runner_string_dup(&entries[1].value, "two");

  runner_map_u32_string_t input = {.ptr = entries, .len = 2};
  runner_map_string_u32_t result;
  test_maps_to_test_inline_roundtrip(&input, &result);

  assert(result.len == 2);
  bool saw_one = false, saw_two = false;
  for (size_t i = 0; i < result.len; i++) {
    if (string_eq(&result.ptr[i].key, "one")) {
      assert(result.ptr[i].value == 1);
      saw_one = true;
    } else if (string_eq(&result.ptr[i].key, "two")) {
      assert(result.ptr[i].value == 2);
      saw_two = true;
    }
  }
  assert(saw_one && saw_two);
  runner_map_string_u32_free(&result);
}

static void test_large_roundtrip(void) {
  size_t n = 100;
  test_maps_to_test_names_by_id_entry_t *entries =
      (test_maps_to_test_names_by_id_entry_t *)malloc(
          n * sizeof(test_maps_to_test_names_by_id_entry_t));
  for (size_t i = 0; i < n; i++) {
    entries[i].key = (uint32_t)i;
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "value-%zu", i);
    runner_string_dup_n(&entries[i].value, buf, (size_t)len);
  }
  test_maps_to_test_names_by_id_t input = {.ptr = entries, .len = n};
  test_maps_to_test_names_by_id_t result;
  test_maps_to_test_large_roundtrip(&input, &result);
  assert(result.len == n);

  // Free locally-allocated input contents (keys are u32, values are strings).
  for (size_t i = 0; i < n; i++) {
    runner_string_free(&entries[i].value);
  }
  free(entries);

  test_maps_to_test_names_by_id_free(&result);
}

static void test_multi_param_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t names_entries[2];
  names_entries[0].key = 1;
  runner_string_dup(&names_entries[0].value, "one");
  names_entries[1].key = 2;
  runner_string_dup(&names_entries[1].value, "two");
  test_maps_to_test_names_by_id_t names = {.ptr = names_entries, .len = 2};

  test_maps_to_test_bytes_by_name_entry_t bytes_entries[1];
  runner_string_dup(&bytes_entries[0].key, "key");
  bytes_entries[0].value.ptr = (uint8_t *)malloc(1);
  bytes_entries[0].value.ptr[0] = 42;
  bytes_entries[0].value.len = 1;
  test_maps_to_test_bytes_by_name_t bytes = {.ptr = bytes_entries, .len = 1};

  test_maps_to_test_tuple2_ids_by_name_bytes_by_name_t result;
  test_maps_to_test_multi_param_roundtrip(&names, &bytes, &result);

  assert(result.f0.len == 2);
  assert(result.f1.len == 1);
  assert(string_eq(&result.f1.ptr[0].key, "key"));
  assert(result.f1.ptr[0].value.len == 1);
  assert(result.f1.ptr[0].value.ptr[0] == 42);
  test_maps_to_test_tuple2_ids_by_name_bytes_by_name_free(&result);
}

static void test_nested_roundtrip(void) {
  runner_map_u32_string_entry_t inner_a[2];
  inner_a[0].key = 1;
  runner_string_dup(&inner_a[0].value, "one");
  inner_a[1].key = 2;
  runner_string_dup(&inner_a[1].value, "two");

  runner_map_u32_string_entry_t inner_b[1];
  inner_b[0].key = 10;
  runner_string_dup(&inner_b[0].value, "ten");

  runner_map_string_map_u32_string_entry_t outer[2];
  runner_string_dup(&outer[0].key, "group-a");
  outer[0].value.ptr = inner_a;
  outer[0].value.len = 2;
  runner_string_dup(&outer[1].key, "group-b");
  outer[1].value.ptr = inner_b;
  outer[1].value.len = 1;

  runner_map_string_map_u32_string_t input = {.ptr = outer, .len = 2};
  runner_map_string_map_u32_string_t result;
  test_maps_to_test_nested_roundtrip(&input, &result);

  assert(result.len == 2);
  for (size_t i = 0; i < result.len; i++) {
    if (string_eq(&result.ptr[i].key, "group-a")) {
      assert(result.ptr[i].value.len == 2);
    } else if (string_eq(&result.ptr[i].key, "group-b")) {
      assert(result.ptr[i].value.len == 1);
    } else {
      assert(0 && "unexpected outer key");
    }
  }
  runner_map_string_map_u32_string_free(&result);
}

static void test_variant_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t entries[1];
  entries[0].key = 1;
  runner_string_dup(&entries[0].value, "one");

  test_maps_to_test_map_or_string_t input;
  input.tag = TEST_MAPS_TO_TEST_MAP_OR_STRING_AS_MAP;
  input.val.as_map.ptr = entries;
  input.val.as_map.len = 1;

  test_maps_to_test_map_or_string_t result;
  test_maps_to_test_variant_roundtrip(&input, &result);

  assert(result.tag == TEST_MAPS_TO_TEST_MAP_OR_STRING_AS_MAP);
  assert(result.val.as_map.len == 1);
  assert(result.val.as_map.ptr[0].key == 1);
  assert(string_eq(&result.val.as_map.ptr[0].value, "one"));
  test_maps_to_test_map_or_string_free(&result);

  test_maps_to_test_map_or_string_t input2;
  input2.tag = TEST_MAPS_TO_TEST_MAP_OR_STRING_AS_STRING;
  runner_string_dup(&input2.val.as_string, "hello");

  test_maps_to_test_map_or_string_t result2;
  test_maps_to_test_variant_roundtrip(&input2, &result2);

  assert(result2.tag == TEST_MAPS_TO_TEST_MAP_OR_STRING_AS_STRING);
  assert(string_eq(&result2.val.as_string, "hello"));
  test_maps_to_test_map_or_string_free(&result2);
}

static void test_result_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t entries[1];
  entries[0].key = 5;
  runner_string_dup(&entries[0].value, "five");

  test_maps_to_test_result_names_by_id_string_t ok_input;
  ok_input.is_err = false;
  ok_input.val.ok.ptr = entries;
  ok_input.val.ok.len = 1;

  test_maps_to_test_names_by_id_t ok_ret;
  runner_string_t err_ret;
  bool is_ok = test_maps_to_test_result_roundtrip(&ok_input, &ok_ret, &err_ret);
  assert(is_ok);
  assert(ok_ret.len == 1);
  assert(ok_ret.ptr[0].key == 5);
  assert(string_eq(&ok_ret.ptr[0].value, "five"));
  test_maps_to_test_names_by_id_free(&ok_ret);

  test_maps_to_test_result_names_by_id_string_t err_input;
  err_input.is_err = true;
  runner_string_dup(&err_input.val.err, "bad input");

  test_maps_to_test_names_by_id_t ok_ret2;
  runner_string_t err_ret2;
  is_ok = test_maps_to_test_result_roundtrip(&err_input, &ok_ret2, &err_ret2);
  assert(!is_ok);
  assert(string_eq(&err_ret2, "bad input"));
  runner_string_free(&err_ret2);
}

static void test_tuple_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t entries[1];
  entries[0].key = 7;
  runner_string_dup(&entries[0].value, "seven");

  test_maps_to_test_tuple2_names_by_id_u64_t input;
  input.f0.ptr = entries;
  input.f0.len = 1;
  input.f1 = 42;

  test_maps_to_test_tuple2_names_by_id_u64_t result;
  test_maps_to_test_tuple_roundtrip(&input, &result);

  assert(result.f0.len == 1);
  assert(result.f0.ptr[0].key == 7);
  assert(string_eq(&result.f0.ptr[0].value, "seven"));
  assert(result.f1 == 42);
  test_maps_to_test_tuple2_names_by_id_u64_free(&result);
}

static void test_single_entry_roundtrip(void) {
  test_maps_to_test_names_by_id_entry_t entries[1];
  entries[0].key = 99;
  runner_string_dup(&entries[0].value, "ninety-nine");

  test_maps_to_test_names_by_id_t input = {.ptr = entries, .len = 1};
  test_maps_to_test_names_by_id_t result;
  test_maps_to_test_single_entry_roundtrip(&input, &result);

  assert(result.len == 1);
  assert(result.ptr[0].key == 99);
  assert(string_eq(&result.ptr[0].value, "ninety-nine"));
  test_maps_to_test_names_by_id_free(&result);
}

void exports_runner_run(void) {
  test_named_roundtrip();
  test_bytes_roundtrip();
  test_empty_roundtrip();
  test_option_roundtrip();
  test_record_roundtrip();
  test_inline_roundtrip();
  test_large_roundtrip();
  test_multi_param_roundtrip();
  test_nested_roundtrip();
  test_variant_roundtrip();
  test_result_roundtrip();
  test_tuple_roundtrip();
  test_single_entry_roundtrip();
}
