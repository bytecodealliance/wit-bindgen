#include <assert.h>
#include <records.h>

void records_test_imports() {
  {
    uint8_t a;
    uint16_t b;
    imports_multiple_results(&a, &b);
    assert(a == 4);
    assert(b == 5);
  }

  records_tuple2_u8_u32_t input;
  records_tuple2_u32_u8_t output;
  input.f0 = 1;
  input.f1 = 2;
  imports_swap_tuple(&input, &output);
  assert(output.f0 == 2);
  assert(output.f1 == 1);

  assert(imports_roundtrip_flags1(TEST_F1_A) == TEST_F1_A);
  assert(imports_roundtrip_flags1(0) == 0);
  assert(imports_roundtrip_flags1(TEST_F1_B) == TEST_F1_B);
  assert(imports_roundtrip_flags1(TEST_F1_A | TEST_F1_B) == (TEST_F1_A | TEST_F1_B));

  assert(imports_roundtrip_flags2(TEST_F2_C) == TEST_F2_C);
  assert(imports_roundtrip_flags2(0) == 0);
  assert(imports_roundtrip_flags2(TEST_F2_D) == TEST_F2_D);
  assert(imports_roundtrip_flags2(TEST_F2_C | TEST_F2_E) == (TEST_F2_C | TEST_F2_E));

  test_flag8_t flag8;
  test_flag16_t flag16;
  test_flag32_t flag32;
  test_flag64_t flag64;
  imports_roundtrip_flags3(TEST_FLAG8_B0, TEST_FLAG16_B1, TEST_FLAG32_B2, TEST_FLAG64_B3,
      &flag8, &flag16, &flag32, &flag64);
  assert(flag8 == TEST_FLAG8_B0);
  assert(flag16 == TEST_FLAG16_B1);
  assert(flag32 == TEST_FLAG32_B2);
  assert(flag64 == TEST_FLAG64_B3);

  {
    test_r1_t a, b;
    a.a = 8;
    a.b = 0;
    imports_roundtrip_record1(&a, &b);
    assert(b.a == 8);
    assert(b.b == 0);
  }

  {
    test_r1_t a, b;
    a.a = 0;
    a.b = TEST_F1_A | TEST_F1_B;
    imports_roundtrip_record1(&a, &b);
    assert(b.a == 0);
    assert(b.b == (TEST_F1_A | TEST_F1_B));
  }

  records_tuple0_t t0;
  imports_tuple0(&t0, &t0);

  records_tuple1_u8_t t1, t2;
  t1.f0 = 1;
  imports_tuple1(&t1, &t2);
  assert(t2.f0 == 1);
}

void exports_multiple_results(uint8_t *ret0, uint16_t *ret1) {
  *ret0 = 100;
  *ret1 = 200;
}

void exports_swap_tuple(records_tuple2_u8_u32_t *a, records_tuple2_u32_u8_t *b) {
  b->f0 = a->f1;
  b->f1 = a->f0;
}

test_f1_t exports_roundtrip_flags1(test_f1_t a) {
  return a;
}

test_f2_t exports_roundtrip_flags2(test_f2_t a) {
  return a;
}

void exports_roundtrip_flags3(test_flag8_t a, test_flag16_t b, test_flag32_t c, test_flag64_t d, test_flag8_t *ret0, test_flag16_t *ret1, test_flag32_t *ret2, test_flag64_t *ret3) {
  *ret0 = a;
  *ret1 = b;
  *ret2 = c;
  *ret3 = d;
}

void exports_roundtrip_record1(test_r1_t *a, test_r1_t *ret0) {
  *ret0 = *a;
}

void exports_tuple0(records_tuple0_t *a, records_tuple0_t *b) {
}

void exports_tuple1(records_tuple1_u8_t *a, records_tuple1_u8_t *b) {
  b->f0 = a->f0;
}
