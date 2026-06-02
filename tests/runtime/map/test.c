//@ wasmtime-flags = '-Wcomponent-model-map'

#include <assert.h>
#include <stdlib.h>
#include <string.h>

#include "test.h"

static bool string_eq(const test_string_t *s, const char *lit) {
  size_t len = strlen(lit);
  return s->len == len && memcmp(s->ptr, lit, len) == 0;
}

void exports_test_maps_to_test_named_roundtrip(
    exports_test_maps_to_test_names_by_id_t *a,
    exports_test_maps_to_test_ids_by_name_t *ret) {
  assert(a->len == 2);
  for (size_t i = 0; i < a->len; i++) {
    if (a->ptr[i].key == 1) {
      assert(string_eq(&a->ptr[i].value, "uno"));
    } else if (a->ptr[i].key == 2) {
      assert(string_eq(&a->ptr[i].value, "two"));
    } else {
      assert(0 && "unexpected key");
    }
  }

  ret->len = a->len;
  ret->ptr = (exports_test_maps_to_test_ids_by_name_entry_t *)malloc(
      a->len * sizeof(exports_test_maps_to_test_ids_by_name_entry_t));
  for (size_t i = 0; i < a->len; i++) {
    ret->ptr[i].key = a->ptr[i].value;
    ret->ptr[i].value = a->ptr[i].key;
  }
  free(a->ptr);
}

void exports_test_maps_to_test_bytes_roundtrip(
    exports_test_maps_to_test_bytes_by_name_t *a,
    exports_test_maps_to_test_bytes_by_name_t *ret) {
  assert(a->len == 2);
  for (size_t i = 0; i < a->len; i++) {
    if (string_eq(&a->ptr[i].key, "hello")) {
      assert(a->ptr[i].value.len == 5);
      assert(memcmp(a->ptr[i].value.ptr, "world", 5) == 0);
    } else if (string_eq(&a->ptr[i].key, "bin")) {
      assert(a->ptr[i].value.len == 3);
      uint8_t expected[] = {0, 1, 2};
      assert(memcmp(a->ptr[i].value.ptr, expected, 3) == 0);
    } else {
      assert(0 && "unexpected key");
    }
  }
  *ret = *a;
}

void exports_test_maps_to_test_empty_roundtrip(
    exports_test_maps_to_test_names_by_id_t *a,
    exports_test_maps_to_test_names_by_id_t *ret) {
  assert(a->len == 0);
  *ret = *a;
}

void exports_test_maps_to_test_option_roundtrip(test_map_string_option_u32_t *a,
                                                test_map_string_option_u32_t *ret) {
  assert(a->len == 2);
  bool saw_some = false, saw_none = false;
  for (size_t i = 0; i < a->len; i++) {
    if (string_eq(&a->ptr[i].key, "some")) {
      assert(a->ptr[i].value.is_some);
      assert(a->ptr[i].value.val == 42);
      saw_some = true;
    } else if (string_eq(&a->ptr[i].key, "none")) {
      assert(!a->ptr[i].value.is_some);
      saw_none = true;
    }
  }
  assert(saw_some && saw_none);
  *ret = *a;
}

void exports_test_maps_to_test_record_roundtrip(
    exports_test_maps_to_test_labeled_entry_t *a,
    exports_test_maps_to_test_labeled_entry_t *ret) {
  assert(string_eq(&a->label, "test-label"));
  assert(a->values.len == 2);
  for (size_t i = 0; i < a->values.len; i++) {
    if (a->values.ptr[i].key == 10) {
      assert(string_eq(&a->values.ptr[i].value, "ten"));
    } else if (a->values.ptr[i].key == 20) {
      assert(string_eq(&a->values.ptr[i].value, "twenty"));
    } else {
      assert(0 && "unexpected key");
    }
  }
  *ret = *a;
}

void exports_test_maps_to_test_inline_roundtrip(test_map_u32_string_t *a,
                                                test_map_string_u32_t *ret) {
  ret->len = a->len;
  ret->ptr = (test_map_string_u32_entry_t *)malloc(
      a->len * sizeof(test_map_string_u32_entry_t));
  for (size_t i = 0; i < a->len; i++) {
    ret->ptr[i].key = a->ptr[i].value;
    ret->ptr[i].value = a->ptr[i].key;
  }
  free(a->ptr);
}

void exports_test_maps_to_test_large_roundtrip(
    exports_test_maps_to_test_names_by_id_t *a,
    exports_test_maps_to_test_names_by_id_t *ret) {
  assert(a->len == 100);
  *ret = *a;
}

void exports_test_maps_to_test_multi_param_roundtrip(
    exports_test_maps_to_test_names_by_id_t *a,
    exports_test_maps_to_test_bytes_by_name_t *b,
    exports_test_maps_to_test_tuple2_ids_by_name_bytes_by_name_t *ret) {
  assert(a->len == 2);
  assert(b->len == 1);

  ret->f0.len = a->len;
  ret->f0.ptr = (exports_test_maps_to_test_ids_by_name_entry_t *)malloc(
      a->len * sizeof(exports_test_maps_to_test_ids_by_name_entry_t));
  for (size_t i = 0; i < a->len; i++) {
    ret->f0.ptr[i].key = a->ptr[i].value;
    ret->f0.ptr[i].value = a->ptr[i].key;
  }
  free(a->ptr);

  ret->f1 = *b;
}

void exports_test_maps_to_test_nested_roundtrip(
    test_map_string_map_u32_string_t *a, test_map_string_map_u32_string_t *ret) {
  assert(a->len == 2);
  for (size_t i = 0; i < a->len; i++) {
    if (string_eq(&a->ptr[i].key, "group-a")) {
      assert(a->ptr[i].value.len == 2);
    } else if (string_eq(&a->ptr[i].key, "group-b")) {
      assert(a->ptr[i].value.len == 1);
    } else {
      assert(0 && "unexpected outer key");
    }
  }
  *ret = *a;
}

void exports_test_maps_to_test_variant_roundtrip(
    exports_test_maps_to_test_map_or_string_t *a,
    exports_test_maps_to_test_map_or_string_t *ret) {
  *ret = *a;
}

bool exports_test_maps_to_test_result_roundtrip(
    exports_test_maps_to_test_result_names_by_id_string_t *a,
    exports_test_maps_to_test_names_by_id_t *ret, test_string_t *err) {
  if (a->is_err) {
    *err = a->val.err;
    return false;
  }
  *ret = a->val.ok;
  return true;
}

void exports_test_maps_to_test_tuple_roundtrip(
    exports_test_maps_to_test_tuple2_names_by_id_u64_t *a,
    exports_test_maps_to_test_tuple2_names_by_id_u64_t *ret) {
  assert(a->f0.len == 1);
  assert(a->f0.ptr[0].key == 7);
  assert(string_eq(&a->f0.ptr[0].value, "seven"));
  assert(a->f1 == 42);
  *ret = *a;
}

void exports_test_maps_to_test_single_entry_roundtrip(
    exports_test_maps_to_test_names_by_id_t *a,
    exports_test_maps_to_test_names_by_id_t *ret) {
  assert(a->len == 1);
  *ret = *a;
}
