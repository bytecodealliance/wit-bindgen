#include <assert.h>
#include <exports.h>
#include <imports.h>
#include <limits.h>
#include <math.h>

uint8_t exports_roundtrip_u8(uint8_t a) {
  return a;
}

int8_t exports_roundtrip_s8(int8_t a) {
  return a;
}

uint16_t exports_roundtrip_u16(uint16_t a) {
  return a;
}

int16_t exports_roundtrip_s16(int16_t a) {
  return a;
}

uint32_t exports_roundtrip_u32(uint32_t a) {
  return a;
}

int32_t exports_roundtrip_s32(int32_t a) {
  return a;
}

uint64_t exports_roundtrip_u64(uint64_t a) {
  return a;
}

int64_t exports_roundtrip_s64(int64_t a) {
  return a;
}

float exports_roundtrip_float32(float a) {
  return a;
}

double exports_roundtrip_float64(double a) {
  return a;
}

uint32_t exports_roundtrip_char(uint32_t a) {
  return a;
}

static uint32_t SCALAR = 0;

void exports_set_scalar(uint32_t a) {
  SCALAR = a;
}

uint32_t exports_get_scalar(void) {
  return SCALAR;
}


void exports_test_imports() {
  assert(imports_roundtrip_u8(1) == 1);
  assert(imports_roundtrip_u8(0) == 0);
  assert(imports_roundtrip_u8(UCHAR_MAX) == UCHAR_MAX);

  assert(imports_roundtrip_s8(1) == 1);
  assert(imports_roundtrip_s8(SCHAR_MIN) == SCHAR_MIN);
  assert(imports_roundtrip_s8(SCHAR_MAX) == SCHAR_MAX);

  assert(imports_roundtrip_u16(1) == 1);
  assert(imports_roundtrip_u16(0) == 0);
  assert(imports_roundtrip_u16(USHRT_MAX) == USHRT_MAX);

  assert(imports_roundtrip_s16(1) == 1);
  assert(imports_roundtrip_s16(SHRT_MIN) == SHRT_MIN);
  assert(imports_roundtrip_s16(SHRT_MAX) == SHRT_MAX);

  assert(imports_roundtrip_u32(1) == 1);
  assert(imports_roundtrip_u32(0) == 0);
  assert(imports_roundtrip_u32(UINT_MAX) == UINT_MAX);

  assert(imports_roundtrip_s32(1) == 1);
  assert(imports_roundtrip_s32(INT_MIN) == INT_MIN);
  assert(imports_roundtrip_s32(INT_MAX) == INT_MAX);

  assert(imports_roundtrip_u64(1) == 1);
  assert(imports_roundtrip_u64(0) == 0);
  assert(imports_roundtrip_u64(ULONG_MAX) == ULONG_MAX);

  assert(imports_roundtrip_s64(1) == 1);
  assert(imports_roundtrip_s64(LONG_MIN) == LONG_MIN);
  assert(imports_roundtrip_s64(LONG_MAX) == LONG_MAX);

  assert(imports_roundtrip_float32(1.0) == 1.0);
  assert(imports_roundtrip_float32(INFINITY) == INFINITY);
  assert(imports_roundtrip_float32(-INFINITY) == -INFINITY);
  assert(isnan(imports_roundtrip_float32(NAN)));

  assert(imports_roundtrip_float64(1.0) == 1.0);
  assert(imports_roundtrip_float64(INFINITY) == INFINITY);
  assert(imports_roundtrip_float64(-INFINITY) == -INFINITY);
  assert(isnan(imports_roundtrip_float64(NAN)));

  assert(imports_roundtrip_char('a') == 'a');
  assert(imports_roundtrip_char(' ') == ' ');
  assert(imports_roundtrip_char(U'🚩') == U'🚩');

  imports_set_scalar(2);
  assert(imports_get_scalar() == 2);
  imports_set_scalar(4);
  assert(imports_get_scalar() == 4);
}
