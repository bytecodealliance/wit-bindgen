#include <assert.h>
#include <float.h>
#include <limits.h>
#include <math.h>
#include <stdalign.h>
#include <stdlib.h>
#include <string.h>

#include "test.h"

uint32_t exports_test_lists_to_test_allocated_bytes(void) {
  // TODO: should ideally fill this out
  return 0;
}

void exports_test_lists_to_test_empty_list_param(test_list_u8_t *a) {
  assert(a->len == 0);
}

void exports_test_lists_to_test_empty_string_param(test_string_t *a) {
  assert(a->len == 0);
}

void exports_test_lists_to_test_empty_list_result(test_list_u8_t *ret0) {
  ret0->ptr = 0;
  ret0->len = 0;
}

void exports_test_lists_to_test_empty_string_result(test_string_t *ret0) {
  ret0->ptr = 0;
  ret0->len = 0;
}

void exports_test_lists_to_test_list_param(test_list_u8_t *a) {
  assert(a->len == 4);
  assert(a->ptr[0] == 1);
  assert(a->ptr[1] == 2);
  assert(a->ptr[2] == 3);
  assert(a->ptr[3] == 4);
  test_list_u8_free(a);
}

void exports_test_lists_to_test_list_param2(test_string_t *a) {
  assert(a->len == 3);
  assert(a->ptr[0] == 'f');
  assert(a->ptr[1] == 'o');
  assert(a->ptr[2] == 'o');
  test_string_free(a);
}

void exports_test_lists_to_test_list_param3(test_list_string_t *a) {
  assert(a->len == 3);
  assert(a->ptr[0].len == 3);
  assert(a->ptr[0].ptr[0] == 'f');
  assert(a->ptr[0].ptr[1] == 'o');
  assert(a->ptr[0].ptr[2] == 'o');

  assert(a->ptr[1].len == 3);
  assert(a->ptr[1].ptr[0] == 'b');
  assert(a->ptr[1].ptr[1] == 'a');
  assert(a->ptr[1].ptr[2] == 'r');

  assert(a->ptr[2].len == 3);
  assert(a->ptr[2].ptr[0] == 'b');
  assert(a->ptr[2].ptr[1] == 'a');
  assert(a->ptr[2].ptr[2] == 'z');

  test_list_string_free(a);
}

void exports_test_lists_to_test_list_param4(test_list_list_string_t *a) {
  assert(a->len == 2);
  assert(a->ptr[0].len == 2);
  assert(a->ptr[1].len == 1);

  assert(a->ptr[0].ptr[0].len == 3);
  assert(a->ptr[0].ptr[0].ptr[0] == 'f');
  assert(a->ptr[0].ptr[0].ptr[1] == 'o');
  assert(a->ptr[0].ptr[0].ptr[2] == 'o');

  assert(a->ptr[0].ptr[1].len == 3);
  assert(a->ptr[0].ptr[1].ptr[0] == 'b');
  assert(a->ptr[0].ptr[1].ptr[1] == 'a');
  assert(a->ptr[0].ptr[1].ptr[2] == 'r');

  assert(a->ptr[1].ptr[0].len == 3);
  assert(a->ptr[1].ptr[0].ptr[0] == 'b');
  assert(a->ptr[1].ptr[0].ptr[1] == 'a');
  assert(a->ptr[1].ptr[0].ptr[2] == 'z');

  test_list_list_string_free(a);
}

void exports_test_lists_to_test_list_param_large(test_list_string_t *a) {
  assert(a->len == 1000);
  test_list_string_free(a);
}

void exports_test_lists_to_test_list_param5(test_list_tuple3_u8_u32_u8_t *a) {
  assert(a->len == 2);
  assert(a->ptr[0].f0 == 1);
  assert(a->ptr[0].f1 == 2);
  assert(a->ptr[0].f2 == 3);
  assert(a->ptr[1].f0 == 4);
  assert(a->ptr[1].f1 == 5);
  assert(a->ptr[1].f2 == 6);
  test_list_tuple3_u8_u32_u8_free(a);
}

void exports_test_lists_to_test_list_result(test_list_u8_t *ret0) {
  ret0->ptr = (uint8_t *) malloc(5);
  ret0->len = 5;
  ret0->ptr[0] = 1;
  ret0->ptr[1] = 2;
  ret0->ptr[2] = 3;
  ret0->ptr[3] = 4;
  ret0->ptr[4] = 5;
}

void exports_test_lists_to_test_list_result2(test_string_t *ret0) {
  test_string_dup(ret0, "hello!");
}

void exports_test_lists_to_test_list_result3(test_list_string_t *ret0) {
  ret0->len = 2;
  ret0->ptr = (test_string_t *) malloc(2 * sizeof(test_string_t));

  test_string_dup(&ret0->ptr[0], "hello,");
  test_string_dup(&ret0->ptr[1], "world!");
}

void exports_test_lists_to_test_list_roundtrip(test_list_u8_t *a, test_list_u8_t *ret0) {
  *ret0 = *a;
}

void exports_test_lists_to_test_string_roundtrip(test_string_t *a, test_string_t *ret0) {
  *ret0 = *a;
}

void exports_test_lists_to_test_list_minmax8(test_list_u8_t *a, test_list_s8_t *b, test_tuple2_list_u8_list_s8_t *ret) {
  ret->f0 = *a;
  ret->f1 = *b;
}

void exports_test_lists_to_test_list_minmax16(test_list_u16_t *a, test_list_s16_t *b, test_tuple2_list_u16_list_s16_t *ret) {
  ret->f0 = *a;
  ret->f1 = *b;
}

void exports_test_lists_to_test_list_minmax32(test_list_u32_t *a, test_list_s32_t *b, test_tuple2_list_u32_list_s32_t *ret) {
  ret->f0 = *a;
  ret->f1 = *b;
}

void exports_test_lists_to_test_list_minmax64(test_list_u64_t *a, test_list_s64_t *b, test_tuple2_list_u64_list_s64_t *ret) {
  ret->f0 = *a;
  ret->f1 = *b;
}

void exports_test_lists_to_test_list_minmax_float(test_list_f32_t *a, test_list_f64_t *b, test_tuple2_list_f32_list_f64_t *ret) {
  ret->f0 = *a;
  ret->f1 = *b;
}
