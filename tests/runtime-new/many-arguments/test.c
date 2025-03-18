#include <assert.h>

#include "test.h"

void exports_test_many_arguments_to_test_many_arguments(
    uint64_t a1,
    uint64_t a2,
    uint64_t a3,
    uint64_t a4,
    uint64_t a5,
    uint64_t a6,
    uint64_t a7,
    uint64_t a8,
    uint64_t a9,
    uint64_t a10,
    uint64_t a11,
    uint64_t a12,
    uint64_t a13,
    uint64_t a14,
    uint64_t a15,
    uint64_t a16
    ) {
  assert(a1 == 1);
  assert(a2 == 2);
  assert(a3 == 3);
  assert(a4 == 4);
  assert(a5 == 5);
  assert(a6 == 6);
  assert(a7 == 7);
  assert(a8 == 8);
  assert(a9 == 9);
  assert(a10 == 10);
  assert(a11 == 11);
  assert(a12 == 12);
  assert(a13 == 13);
  assert(a14 == 14);
  assert(a15 == 15);
  assert(a16 == 16);
}
