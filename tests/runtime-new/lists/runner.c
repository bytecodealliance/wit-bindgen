#include <assert.h>
#include <float.h>
#include <limits.h>
#include <math.h>
#include <stdalign.h>
#include <stdlib.h>
#include <string.h>

#include "runner.h"

int main() {
  {
    uint8_t list[] = {};
    runner_list_u8_t a;
    a.ptr = list;
    a.len = 0;
    test_lists_to_test_empty_list_param(&a);
  }

  {
    runner_string_t a;
    runner_string_set(&a, "");
    test_lists_to_test_empty_string_param(&a);
  }

  {
    runner_list_u8_t a;
    test_lists_to_test_empty_list_result(&a);
    assert(a.len == 0);
  }

  {
    runner_string_t a;
    test_lists_to_test_empty_string_result(&a);
    assert(a.len == 0);
  }

  {
    uint8_t list[] = {1, 2, 3, 4};
    runner_list_u8_t a;
    a.ptr = list;
    a.len = 4;
    test_lists_to_test_list_param(&a);
  }

  {
    runner_string_t a;
    runner_string_set(&a, "foo");
    test_lists_to_test_list_param2(&a);
  }

  {
    runner_string_t list[3];
    runner_string_set(&list[0], "foo");
    runner_string_set(&list[1], "bar");
    runner_string_set(&list[2], "baz");
    runner_list_string_t a;
    a.ptr = list;
    a.len = 3;
    test_lists_to_test_list_param3(&a);
  }

  {
    runner_string_t list1[2];
    runner_string_t list2[1];
    runner_string_set(&list1[0], "foo");
    runner_string_set(&list1[1], "bar");
    runner_string_set(&list2[0], "baz");
    runner_list_list_string_t a;
    a.ptr[0].len = 2;
    a.ptr[0].ptr = list1;
    a.ptr[1].len = 1;
    a.ptr[1].ptr = list2;
    a.len = 2;
    test_lists_to_test_list_param4(&a);
  }

  {
    runner_tuple3_u8_u32_u8_t data[2];
    data[0].f0 = 1;
    data[0].f1 = 2;
    data[0].f2 = 3;
    data[1].f0 = 4;
    data[1].f1 = 5;
    data[1].f2 = 6;
    runner_list_tuple3_u8_u32_u8_t a;
    a.len = 2;
    a.ptr = data;
    test_lists_to_test_list_param5(&a);
  }

  {
    runner_list_u8_t a;
    test_lists_to_test_list_result(&a);
    assert(a.len == 5);
    assert(memcmp(a.ptr, "\x01\x02\x03\x04\x05", 5) == 0);
    runner_list_u8_free(&a);
  }

  {
    runner_string_t a;
    test_lists_to_test_list_result2(&a);
    assert(a.len == 6);
    assert(memcmp(a.ptr, "hello!", 6) == 0);
    runner_string_free(&a);
  }

  {
    runner_list_string_t a;
    test_lists_to_test_list_result3(&a);
    assert(a.len == 2);
    assert(a.ptr[0].len == 6);
    assert(a.ptr[1].len == 6);
    assert(memcmp(a.ptr[0].ptr, "hello,", 6) == 0);
    assert(memcmp(a.ptr[1].ptr, "world!", 6) == 0);
    runner_list_string_free(&a);
  }

  {
    runner_list_u8_t a, b;
    a.len = 0;
    a.ptr = (unsigned char*) "";
    test_lists_to_test_list_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    runner_list_u8_free(&b);

    a.len = 1;
    a.ptr = (unsigned char*) "x";
    test_lists_to_test_list_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    runner_list_u8_free(&b);

    a.len = 5;
    a.ptr = (unsigned char*) "hello";
    test_lists_to_test_list_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    runner_list_u8_free(&b);
  }

  {
    runner_string_t a, b;
    runner_string_set(&a, "x");
    test_lists_to_test_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    runner_string_free(&b);

    runner_string_set(&a, "");
    test_lists_to_test_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    runner_string_free(&b);

    runner_string_set(&a, "hello");
    test_lists_to_test_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    runner_string_free(&b);

    runner_string_set(&a, "hello ⚑ world");
    test_lists_to_test_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    runner_string_free(&b);
  }

  {
    uint8_t u8[2] = {0, UCHAR_MAX};
    int8_t s8[2] = {SCHAR_MIN, SCHAR_MAX};
    runner_list_u8_t list_u8 = { u8, 2 };
    runner_list_s8_t list_s8 = { s8, 2 };
    runner_tuple2_list_u8_list_s8_t ret;
    test_lists_to_test_list_minmax8(&list_u8, &list_s8, &ret);
    assert(ret.f0.len == 2 && ret.f0.ptr[0] == 0 && ret.f0.ptr[1] == UCHAR_MAX);
    assert(ret.f1.len == 2 && ret.f1.ptr[0] == SCHAR_MIN && ret.f1.ptr[1] == SCHAR_MAX);
    runner_list_u8_free(&ret.f0);
    runner_list_s8_free(&ret.f1);
  }

  {
    uint16_t u16[2] = {0, USHRT_MAX};
    int16_t s16[2] = {SHRT_MIN, SHRT_MAX};
    runner_list_u16_t list_u16 = { u16, 2 };
    runner_list_s16_t list_s16 = { s16, 2 };
    runner_tuple2_list_u16_list_s16_t ret;
    test_lists_to_test_list_minmax16(&list_u16, &list_s16, &ret);
    assert(ret.f0.len == 2 && ret.f0.ptr[0] == 0 && ret.f0.ptr[1] == USHRT_MAX);
    assert(ret.f1.len == 2 && ret.f1.ptr[0] == SHRT_MIN && ret.f1.ptr[1] == SHRT_MAX);
    runner_list_u16_free(&ret.f0);
    runner_list_s16_free(&ret.f1);
  }

  {
    uint32_t u32[2] = {0, UINT_MAX};
    int32_t s32[2] = {INT_MIN, INT_MAX};
    runner_list_u32_t list_u32 = { u32, 2 };
    runner_list_s32_t list_s32 = { s32, 2 };
    runner_tuple2_list_u32_list_s32_t ret;
    test_lists_to_test_list_minmax32(&list_u32, &list_s32, &ret);
    assert(ret.f0.len == 2 && ret.f0.ptr[0] == 0 && ret.f0.ptr[1] == UINT_MAX);
    assert(ret.f1.len == 2 && ret.f1.ptr[0] == INT_MIN && ret.f1.ptr[1] == INT_MAX);
    runner_list_u32_free(&ret.f0);
    runner_list_s32_free(&ret.f1);
  }

  {
    uint64_t u64[2] = {0, ULLONG_MAX};
    int64_t s64[2] = {LLONG_MIN, LLONG_MAX};
    runner_list_u64_t list_u64 = { u64, 2 };
    runner_list_s64_t list_s64 = { s64, 2 };
    runner_tuple2_list_u64_list_s64_t ret;
    test_lists_to_test_list_minmax64(&list_u64, &list_s64, &ret);
    assert(ret.f0.len == 2 && ret.f0.ptr[0] == 0 && ret.f0.ptr[1] == ULLONG_MAX);
    assert(ret.f1.len == 2 && ret.f1.ptr[0] == LLONG_MIN && ret.f1.ptr[1] == LLONG_MAX);
    runner_list_u64_free(&ret.f0);
    runner_list_s64_free(&ret.f1);
  }

  {
    float f32[4] = {-FLT_MAX, FLT_MAX, -INFINITY, INFINITY};
    double f64[4] = {-DBL_MAX, DBL_MAX, -INFINITY, INFINITY};
    runner_list_f32_t list_f32 = { f32, 4 };
    runner_list_f64_t list_f64 = { f64, 4 };
    runner_tuple2_list_f32_list_f64_t ret;
    test_lists_to_test_list_minmax_float(&list_f32, &list_f64, &ret);
    assert(ret.f0.len == 4 && ret.f0.ptr[0] == -FLT_MAX && ret.f0.ptr[1] == FLT_MAX);
    assert(ret.f0.ptr[2] == -INFINITY && ret.f0.ptr[3] == INFINITY);
    assert(ret.f1.len == 4 && ret.f1.ptr[0] == -DBL_MAX && ret.f1.ptr[1] == DBL_MAX);
    assert(ret.f1.ptr[2] == -INFINITY && ret.f1.ptr[3] == INFINITY);
    runner_list_f32_free(&ret.f0);
    runner_list_f64_free(&ret.f1);
  }
}
