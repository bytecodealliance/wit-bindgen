#include <assert.h>
#include <exports.h>
#include <float.h>
#include <imports.h>
#include <limits.h>
#include <math.h>
#include <stdalign.h>
#include <stdlib.h>
#include <string.h>

// "custom allocator" which just keeps track of allocated bytes

static size_t ALLOCATED_BYTES = 0;

__attribute__((export_name("canonical_abi_realloc")))
void *canonical_abi_realloc( void *ptr, size_t orig_size, size_t orig_align, size_t new_size) {
  void *ret = realloc(ptr, new_size);
  if (!ret)
    abort();
  ALLOCATED_BYTES -= orig_size;
  ALLOCATED_BYTES += new_size;
  return ret;
}

__attribute__((export_name("canonical_abi_free")))
void canonical_abi_free(void *ptr, size_t size, size_t align) {
  if (size > 0) {
    ALLOCATED_BYTES -= size;
    free(ptr);
  }
}

uint32_t exports_allocated_bytes(void) {
  return ALLOCATED_BYTES;
}

void exports_test_imports() {
  {
    uint8_t list[] = {};
    imports_list_u8_t a;
    a.ptr = list;
    a.len = 0;
    imports_empty_list_param(&a);
  }

  {
    imports_string_t a;
    imports_string_set(&a, "");
    imports_empty_string_param(&a);
  }

  {
    imports_list_u8_t a;
    imports_empty_list_result(&a);
    assert(a.len == 0);
  }

  {
    imports_string_t a;
    imports_empty_string_result(&a);
    assert(a.len == 0);
  }

  {
    uint8_t list[] = {1, 2, 3, 4};
    imports_list_u8_t a;
    a.ptr = list;
    a.len = 4;
    imports_list_param(&a);
  }

  {
    imports_string_t a;
    imports_string_set(&a, "foo");
    imports_list_param2(&a);
  }

  {
    imports_string_t list[3];
    imports_string_set(&list[0], "foo");
    imports_string_set(&list[1], "bar");
    imports_string_set(&list[2], "baz");
    imports_list_string_t a;
    a.ptr = list;
    a.len = 3;
    imports_list_param3(&a);
  }

  {
    imports_string_t list1[2];
    imports_string_t list2[1];
    imports_string_set(&list1[0], "foo");
    imports_string_set(&list1[1], "bar");
    imports_string_set(&list2[0], "baz");
    imports_list_list_string_t a;
    a.ptr[0].len = 2;
    a.ptr[0].ptr = list1;
    a.ptr[1].len = 1;
    a.ptr[1].ptr = list2;
    a.len = 2;
    imports_list_param4(&a);
  }

  {
    imports_list_u8_t a;
    imports_list_result(&a);
    assert(a.len == 5);
    assert(memcmp(a.ptr, "\x01\x02\x03\x04\x05", 5) == 0);
    imports_list_u8_free(&a);
  }

  {
    imports_string_t a;
    imports_list_result2(&a);
    assert(a.len == 6);
    assert(memcmp(a.ptr, "hello!", 6) == 0);
    imports_string_free(&a);
  }

  {
    imports_list_string_t a;
    imports_list_result3(&a);
    assert(a.len == 2);
    assert(a.ptr[0].len == 6);
    assert(a.ptr[1].len == 6);
    assert(memcmp(a.ptr[0].ptr, "hello,", 6) == 0);
    assert(memcmp(a.ptr[1].ptr, "world!", 6) == 0);
    imports_list_string_free(&a);
  }

  {
    imports_string_t a, b;
    imports_string_set(&a, "x");
    imports_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    imports_string_free(&b);

    imports_string_set(&a, "");
    imports_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    imports_string_free(&b);

    imports_string_set(&a, "hello");
    imports_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    imports_string_free(&b);

    imports_string_set(&a, "hello ⚑ world");
    imports_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    imports_string_free(&b);
  }

  {
    uint8_t u8[2] = {0, UCHAR_MAX};
    int8_t s8[2] = {SCHAR_MIN, SCHAR_MAX};
    imports_list_u8_t list_u8 = { u8, 2 };
    imports_list_s8_t list_s8 = { s8, 2 };
    imports_list_u8_t list_u8_out;
    imports_list_s8_t list_s8_out;
    imports_list_minmax8(&list_u8, &list_s8, &list_u8_out, &list_s8_out);
    assert(list_u8_out.len == 2 && list_u8_out.ptr[0] == 0 && list_u8_out.ptr[1] == UCHAR_MAX);
    assert(list_s8_out.len == 2 && list_s8_out.ptr[0] == SCHAR_MIN && list_s8_out.ptr[1] == SCHAR_MAX);
    imports_list_u8_free(&list_u8_out);
    imports_list_s8_free(&list_s8_out);
  }

  {
    uint16_t u16[2] = {0, USHRT_MAX};
    int16_t s16[2] = {SHRT_MIN, SHRT_MAX};
    imports_list_u16_t list_u16 = { u16, 2 };
    imports_list_s16_t list_s16 = { s16, 2 };
    imports_list_u16_t list_u16_out;
    imports_list_s16_t list_s16_out;
    imports_list_minmax16(&list_u16, &list_s16, &list_u16_out, &list_s16_out);
    assert(list_u16_out.len == 2 && list_u16_out.ptr[0] == 0 && list_u16_out.ptr[1] == USHRT_MAX);
    assert(list_s16_out.len == 2 && list_s16_out.ptr[0] == SHRT_MIN && list_s16_out.ptr[1] == SHRT_MAX);
    imports_list_u16_free(&list_u16_out);
    imports_list_s16_free(&list_s16_out);
  }

  {
    uint32_t u32[2] = {0, UINT_MAX};
    int32_t s32[2] = {INT_MIN, INT_MAX};
    imports_list_u32_t list_u32 = { u32, 2 };
    imports_list_s32_t list_s32 = { s32, 2 };
    imports_list_u32_t list_u32_out;
    imports_list_s32_t list_s32_out;
    imports_list_minmax32(&list_u32, &list_s32, &list_u32_out, &list_s32_out);
    assert(list_u32_out.len == 2 && list_u32_out.ptr[0] == 0 && list_u32_out.ptr[1] == UINT_MAX);
    assert(list_s32_out.len == 2 && list_s32_out.ptr[0] == INT_MIN && list_s32_out.ptr[1] == INT_MAX);
    imports_list_u32_free(&list_u32_out);
    imports_list_s32_free(&list_s32_out);
  }

  {
    uint64_t u64[2] = {0, ULLONG_MAX};
    int64_t s64[2] = {LLONG_MIN, LLONG_MAX};
    imports_list_u64_t list_u64 = { u64, 2 };
    imports_list_s64_t list_s64 = { s64, 2 };
    imports_list_u64_t list_u64_out;
    imports_list_s64_t list_s64_out;
    imports_list_minmax64(&list_u64, &list_s64, &list_u64_out, &list_s64_out);
    assert(list_u64_out.len == 2 && list_u64_out.ptr[0] == 0 && list_u64_out.ptr[1] == ULLONG_MAX);
    assert(list_s64_out.len == 2 && list_s64_out.ptr[0] == LLONG_MIN && list_s64_out.ptr[1] == LLONG_MAX);
    imports_list_u64_free(&list_u64_out);
    imports_list_s64_free(&list_s64_out);
  }

  {
    float f32[4] = {-FLT_MAX, FLT_MAX, -INFINITY, INFINITY};
    double f64[4] = {-DBL_MAX, DBL_MAX, -INFINITY, INFINITY};
    imports_list_float32_t list_float32 = { f32, 4 };
    imports_list_float64_t list_float64 = { f64, 4 };
    imports_list_float32_t list_float32_out;
    imports_list_float64_t list_float64_out;
    imports_list_minmax_float(&list_float32, &list_float64, &list_float32_out, &list_float64_out);
    assert(list_float32_out.len == 4 && list_float32_out.ptr[0] == -FLT_MAX && list_float32_out.ptr[1] == FLT_MAX);
    assert(list_float32_out.ptr[2] == -INFINITY && list_float32_out.ptr[3] == INFINITY);
    assert(list_float64_out.len == 4 && list_float64_out.ptr[0] == -DBL_MAX && list_float64_out.ptr[1] == DBL_MAX);
    assert(list_float64_out.ptr[2] == -INFINITY && list_float64_out.ptr[3] == INFINITY);
    imports_list_float32_free(&list_float32_out);
    imports_list_float64_free(&list_float64_out);
  }
}

void exports_empty_list_param(exports_list_u8_t *a) {
  assert(a->len == 0);
}

void exports_empty_string_param(exports_string_t *a) {
  assert(a->len == 0);
}

void exports_empty_list_result(exports_list_u8_t *ret0) {
  ret0->ptr = 0;
  ret0->len = 0;
}

void exports_empty_string_result(exports_string_t *ret0) {
  ret0->ptr = 0;
  ret0->len = 0;
}

void exports_list_param(exports_list_u8_t *a) {
  assert(a->len == 4);
  assert(a->ptr[0] == 1);
  assert(a->ptr[1] == 2);
  assert(a->ptr[2] == 3);
  assert(a->ptr[3] == 4);
  exports_list_u8_free(a);
}

void exports_list_param2(exports_string_t *a) {
  assert(a->len == 3);
  assert(a->ptr[0] == 'f');
  assert(a->ptr[1] == 'o');
  assert(a->ptr[2] == 'o');
  exports_string_free(a);
}

void exports_list_param3(exports_list_string_t *a) {
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

  exports_list_string_free(a);
}

void exports_list_param4(exports_list_list_string_t *a) {
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

  exports_list_list_string_free(a);
}

void exports_list_result(exports_list_u8_t *ret0) {
  ret0->ptr = canonical_abi_realloc(NULL, 0, 1, 5);
  ret0->len = 5;
  ret0->ptr[0] = 1;
  ret0->ptr[1] = 2;
  ret0->ptr[2] = 3;
  ret0->ptr[3] = 4;
  ret0->ptr[4] = 5;
}

void exports_list_result2(exports_string_t *ret0) {
  exports_string_dup(ret0, "hello!");
}

void exports_list_result3(exports_list_string_t *ret0) {
  ret0->len = 2;
  ret0->ptr = canonical_abi_realloc(NULL, 0, alignof(exports_string_t), 2 * sizeof(exports_string_t));

  exports_string_dup(&ret0->ptr[0], "hello,");
  exports_string_dup(&ret0->ptr[1], "world!");
}

void exports_list_roundtrip(exports_list_u8_t *a, exports_list_u8_t *ret0) {
  *ret0 = *a;
}

void exports_string_roundtrip(exports_string_t *a, exports_string_t *ret0) {
  *ret0 = *a;
}
