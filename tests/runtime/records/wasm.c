#include <assert.h>
#include <records.h>

void records_test_imports() {
  {
    uint8_t a;
    uint16_t b;
    test_records_test_multiple_results(&a, &b);
    assert(a == 4);
    assert(b == 5);
  }

  records_tuple2_u8_u32_t input;
  records_tuple2_u32_u8_t output;
  input.f0 = 1;
  input.f1 = 2;
  test_records_test_swap_tuple(&input, &output);
  assert(output.f0 == 2);
  assert(output.f1 == 1);

  assert(test_records_test_roundtrip_flags1(TEST_RECORDS_TEST_F1_A) == TEST_RECORDS_TEST_F1_A);
  assert(test_records_test_roundtrip_flags1(0) == 0);
  assert(test_records_test_roundtrip_flags1(TEST_RECORDS_TEST_F1_B) == TEST_RECORDS_TEST_F1_B);
  assert(test_records_test_roundtrip_flags1(TEST_RECORDS_TEST_F1_A | TEST_RECORDS_TEST_F1_B) == (TEST_RECORDS_TEST_F1_A | TEST_RECORDS_TEST_F1_B));

  assert(test_records_test_roundtrip_flags2(TEST_RECORDS_TEST_F2_C) == TEST_RECORDS_TEST_F2_C);
  assert(test_records_test_roundtrip_flags2(0) == 0);
  assert(test_records_test_roundtrip_flags2(TEST_RECORDS_TEST_F2_D) == TEST_RECORDS_TEST_F2_D);
  assert(test_records_test_roundtrip_flags2(TEST_RECORDS_TEST_F2_C | TEST_RECORDS_TEST_F2_E) == (TEST_RECORDS_TEST_F2_C | TEST_RECORDS_TEST_F2_E));

  test_records_test_flag8_t flag8;
  test_records_test_flag16_t flag16;
  test_records_test_flag32_t flag32;
  test_records_test_flag64_t flag64;
  test_records_test_roundtrip_flags3(TEST_RECORDS_TEST_FLAG8_B0, TEST_RECORDS_TEST_FLAG16_B1, TEST_RECORDS_TEST_FLAG32_B2, TEST_RECORDS_TEST_FLAG64_B3,
      &flag8, &flag16, &flag32, &flag64);
  assert(flag8 == TEST_RECORDS_TEST_FLAG8_B0);
  assert(flag16 == TEST_RECORDS_TEST_FLAG16_B1);
  assert(flag32 == TEST_RECORDS_TEST_FLAG32_B2);
  assert(flag64 == TEST_RECORDS_TEST_FLAG64_B3);

  {
    test_records_test_r1_t a, b;
    a.a = 8;
    a.b = 0;
    test_records_test_roundtrip_record1(&a, &b);
    assert(b.a == 8);
    assert(b.b == 0);
  }

  {
    test_records_test_r1_t a, b;
    a.a = 0;
    a.b = TEST_RECORDS_TEST_F1_A | TEST_RECORDS_TEST_F1_B;
    test_records_test_roundtrip_record1(&a, &b);
    assert(b.a == 0);
    assert(b.b == (TEST_RECORDS_TEST_F1_A | TEST_RECORDS_TEST_F1_B));
  }

  records_tuple1_u8_t t1, t2;
  t1.f0 = 1;
  test_records_test_tuple1(&t1, &t2);
  assert(t2.f0 == 1);
}

void exports_test_records_test_multiple_results(uint8_t *ret0, uint16_t *ret1) {
  *ret0 = 100;
  *ret1 = 200;
}

void exports_test_records_test_swap_tuple(records_tuple2_u8_u32_t *a, records_tuple2_u32_u8_t *b) {
  b->f0 = a->f1;
  b->f1 = a->f0;
}

test_records_test_f1_t exports_test_records_test_roundtrip_flags1(test_records_test_f1_t a) {
  return a;
}

test_records_test_f2_t exports_test_records_test_roundtrip_flags2(test_records_test_f2_t a) {
  return a;
}

void exports_test_records_test_roundtrip_flags3(
      test_records_test_flag8_t a,
      test_records_test_flag16_t b,
      test_records_test_flag32_t c,
      test_records_test_flag64_t d,
      test_records_test_flag8_t *ret0,
      test_records_test_flag16_t *ret1,
      test_records_test_flag32_t *ret2,
      test_records_test_flag64_t *ret3) {
  *ret0 = a;
  *ret1 = b;
  *ret2 = c;
  *ret3 = d;
}

void exports_test_records_test_roundtrip_record1(test_records_test_r1_t *a, test_records_test_r1_t *ret0) {
  *ret0 = *a;
}

void exports_test_records_test_tuple1(records_tuple1_u8_t *a, records_tuple1_u8_t *b) {
  b->f0 = a->f0;
}
