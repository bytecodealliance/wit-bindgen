#include <assert.h>
#include <exports.h>
#include <imports.h>
#include <limits.h>
#include <math.h>

void exports_many_arguments(
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
    uint64_t a16,
    uint64_t a17,
    uint64_t a18,
    uint64_t a19,
    uint64_t a20
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
  assert(a17 == 17);
  assert(a18 == 18);
  assert(a19 == 19);
  assert(a20 == 20);

  imports_many_arguments(
      a1,
      a2,
      a3,
      a4,
      a5,
      a6,
      a7,
      a8,
      a9,
      a10,
      a11,
      a12,
      a13,
      a14,
      a15,
      a16,
      a17,
      a18,
      a19,
      a20
  );
}
