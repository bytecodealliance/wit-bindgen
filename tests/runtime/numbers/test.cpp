#include <assert.h>
#include <limits.h>
#include <math.h>
#include <test_cpp.h>

uint8_t exports::test::numbers::numbers::RoundtripU8(uint8_t a) {
  return a;
}

int8_t exports::test::numbers::numbers::RoundtripS8(int8_t a) {
  return a;
}

uint16_t exports::test::numbers::numbers::RoundtripU16(uint16_t a) {
  return a;
}

int16_t exports::test::numbers::numbers::RoundtripS16(int16_t a) {
  return a;
}

uint32_t exports::test::numbers::numbers::RoundtripU32(uint32_t a) {
  return a;
}

int32_t exports::test::numbers::numbers::RoundtripS32(int32_t a) {
  return a;
}

uint64_t exports::test::numbers::numbers::RoundtripU64(uint64_t a) {
  return a;
}

int64_t exports::test::numbers::numbers::RoundtripS64(int64_t a) {
  return a;
}

float exports::test::numbers::numbers::RoundtripF32(float a) {
  return a;
}

double exports::test::numbers::numbers::RoundtripF64(double a) {
  return a;
}

uint32_t exports::test::numbers::numbers::RoundtripChar(uint32_t a) {
  return a;
}

static uint32_t SCALAR = 0;

void exports::test::numbers::numbers::SetScalar(uint32_t a) {
  SCALAR = a;
}

uint32_t exports::test::numbers::numbers::GetScalar(void) {
  return SCALAR;
}
