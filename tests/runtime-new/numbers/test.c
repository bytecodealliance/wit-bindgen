#include <assert.h>
#include <limits.h>
#include <math.h>
#include <test.h>

uint8_t exports_test_numbers_numbers_roundtrip_u8(uint8_t a) {
  return a;
}

int8_t exports_test_numbers_numbers_roundtrip_s8(int8_t a) {
  return a;
}

uint16_t exports_test_numbers_numbers_roundtrip_u16(uint16_t a) {
  return a;
}

int16_t exports_test_numbers_numbers_roundtrip_s16(int16_t a) {
  return a;
}

uint32_t exports_test_numbers_numbers_roundtrip_u32(uint32_t a) {
  return a;
}

int32_t exports_test_numbers_numbers_roundtrip_s32(int32_t a) {
  return a;
}

uint64_t exports_test_numbers_numbers_roundtrip_u64(uint64_t a) {
  return a;
}

int64_t exports_test_numbers_numbers_roundtrip_s64(int64_t a) {
  return a;
}

float exports_test_numbers_numbers_roundtrip_f32(float a) {
  return a;
}

double exports_test_numbers_numbers_roundtrip_f64(double a) {
  return a;
}

uint32_t exports_test_numbers_numbers_roundtrip_char(uint32_t a) {
  return a;
}

static uint32_t SCALAR = 0;

void exports_test_numbers_numbers_set_scalar(uint32_t a) {
  SCALAR = a;
}

uint32_t exports_test_numbers_numbers_get_scalar(void) {
  return SCALAR;
}
