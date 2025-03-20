#include <assert.h>
#include "runner.h"

int main()
{
    {
        runner_tuple2_u8_u16_t ret;
        test_records_to_test_multiple_results(&ret);
        assert(ret.f0 == 4);
        assert(ret.f1 == 5);
    }

    runner_tuple2_u8_u32_t input;
    runner_tuple2_u32_u8_t output;
    input.f0 = 1;
    input.f1 = 2;
    test_records_to_test_swap_tuple(&input, &output);
    assert(output.f0 == 2);
    assert(output.f1 == 1);

    assert(test_records_to_test_roundtrip_flags1(TEST_RECORDS_TO_TEST_F1_A) == TEST_RECORDS_TO_TEST_F1_A);
    assert(test_records_to_test_roundtrip_flags1(0) == 0);
    assert(test_records_to_test_roundtrip_flags1(TEST_RECORDS_TO_TEST_F1_B) == TEST_RECORDS_TO_TEST_F1_B);
    assert(test_records_to_test_roundtrip_flags1(TEST_RECORDS_TO_TEST_F1_A | TEST_RECORDS_TO_TEST_F1_B) == (TEST_RECORDS_TO_TEST_F1_A | TEST_RECORDS_TO_TEST_F1_B));

    assert(test_records_to_test_roundtrip_flags2(TEST_RECORDS_TO_TEST_F2_C) == TEST_RECORDS_TO_TEST_F2_C);
    assert(test_records_to_test_roundtrip_flags2(0) == 0);
    assert(test_records_to_test_roundtrip_flags2(TEST_RECORDS_TO_TEST_F2_D) == TEST_RECORDS_TO_TEST_F2_D);
    assert(test_records_to_test_roundtrip_flags2(TEST_RECORDS_TO_TEST_F2_C | TEST_RECORDS_TO_TEST_F2_E) == (TEST_RECORDS_TO_TEST_F2_C | TEST_RECORDS_TO_TEST_F2_E));

    test_records_to_test_tuple3_flag8_flag16_flag32_t ret;
    test_records_to_test_roundtrip_flags3(TEST_RECORDS_TO_TEST_FLAG8_B0, TEST_RECORDS_TO_TEST_FLAG16_B1, TEST_RECORDS_TO_TEST_FLAG32_B2,
                                       &ret);
    assert(ret.f0 == TEST_RECORDS_TO_TEST_FLAG8_B0);
    assert(ret.f1 == TEST_RECORDS_TO_TEST_FLAG16_B1);
    assert(ret.f2 == TEST_RECORDS_TO_TEST_FLAG32_B2);

    {
        test_records_to_test_r1_t a, b;
        a.a = 8;
        a.b = 0;
        test_records_to_test_roundtrip_record1(&a, &b);
        assert(b.a == 8);
        assert(b.b == 0);
    }

    {
        test_records_to_test_r1_t a, b;
        a.a = 0;
        a.b = TEST_RECORDS_TO_TEST_F1_A | TEST_RECORDS_TO_TEST_F1_B;
        test_records_to_test_roundtrip_record1(&a, &b);
        assert(b.a == 0);
        assert(b.b == (TEST_RECORDS_TO_TEST_F1_A | TEST_RECORDS_TO_TEST_F1_B));
    }

    runner_tuple1_u8_t t1, t2;
    t1.f0 = 1;
    test_records_to_test_tuple1(&t1, &t2);
    assert(t2.f0 == 1);
}