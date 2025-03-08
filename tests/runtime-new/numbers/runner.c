#include <assert.h>
#include <limits.h>
#include <math.h>
#include <runner.h>

int main() {
  assert(test_numbers_numbers_roundtrip_u8(1) == 1);
  assert(test_numbers_numbers_roundtrip_u8(0) == 0);
  assert(test_numbers_numbers_roundtrip_u8(UCHAR_MAX) == UCHAR_MAX);

  assert(test_numbers_numbers_roundtrip_s8(1) == 1);
  assert(test_numbers_numbers_roundtrip_s8(SCHAR_MIN) == SCHAR_MIN);
  assert(test_numbers_numbers_roundtrip_s8(SCHAR_MAX) == SCHAR_MAX);

  assert(test_numbers_numbers_roundtrip_u16(1) == 1);
  assert(test_numbers_numbers_roundtrip_u16(0) == 0);
  assert(test_numbers_numbers_roundtrip_u16(USHRT_MAX) == USHRT_MAX);

  assert(test_numbers_numbers_roundtrip_s16(1) == 1);
  assert(test_numbers_numbers_roundtrip_s16(SHRT_MIN) == SHRT_MIN);
  assert(test_numbers_numbers_roundtrip_s16(SHRT_MAX) == SHRT_MAX);

  assert(test_numbers_numbers_roundtrip_u32(1) == 1);
  assert(test_numbers_numbers_roundtrip_u32(0) == 0);
  assert(test_numbers_numbers_roundtrip_u32(UINT_MAX) == UINT_MAX);

  assert(test_numbers_numbers_roundtrip_s32(1) == 1);
  assert(test_numbers_numbers_roundtrip_s32(INT_MIN) == INT_MIN);
  assert(test_numbers_numbers_roundtrip_s32(INT_MAX) == INT_MAX);

  assert(test_numbers_numbers_roundtrip_u64(1) == 1);
  assert(test_numbers_numbers_roundtrip_u64(0) == 0);
  assert(test_numbers_numbers_roundtrip_u64(ULONG_MAX) == ULONG_MAX);

  assert(test_numbers_numbers_roundtrip_s64(1) == 1);
  assert(test_numbers_numbers_roundtrip_s64(LONG_MIN) == LONG_MIN);
  assert(test_numbers_numbers_roundtrip_s64(LONG_MAX) == LONG_MAX);

  assert(test_numbers_numbers_roundtrip_f32(1.0) == 1.0);
  assert(test_numbers_numbers_roundtrip_f32(INFINITY) == INFINITY);
  assert(test_numbers_numbers_roundtrip_f32(-INFINITY) == -INFINITY);
  assert(isnan(test_numbers_numbers_roundtrip_f32(NAN)));

  assert(test_numbers_numbers_roundtrip_f64(1.0) == 1.0);
  assert(test_numbers_numbers_roundtrip_f64(INFINITY) == INFINITY);
  assert(test_numbers_numbers_roundtrip_f64(-INFINITY) == -INFINITY);
  assert(isnan(test_numbers_numbers_roundtrip_f64(NAN)));

  assert(test_numbers_numbers_roundtrip_char('a') == 'a');
  assert(test_numbers_numbers_roundtrip_char(' ') == ' ');
  assert(test_numbers_numbers_roundtrip_char(U'🚩') == U'🚩');

  test_numbers_numbers_set_scalar(2);
  assert(test_numbers_numbers_get_scalar() == 2);
  test_numbers_numbers_set_scalar(4);
  assert(test_numbers_numbers_get_scalar() == 4);

  return 0;
}
