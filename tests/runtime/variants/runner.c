#include <assert.h>
#include <runner.h>
#include <stddef.h>

int main() {
    {
        float a = 1;
        uint8_t r;
        assert(test_variants_to_test_roundtrip_option(&a, &r) && r == 1);
        assert(!test_variants_to_test_roundtrip_option(NULL, &r));
        a = 2;
        assert(test_variants_to_test_roundtrip_option(&a, &r) && r == 2);
    }

    {
        test_variants_to_test_result_u32_f32_t a;
        double b_ok;
        uint8_t b_err;

        a.is_err = false;
        a.val.ok = 2;
        assert(test_variants_to_test_roundtrip_result(&a, &b_ok, &b_err));
        assert(b_ok == 2.0);

        a.val.ok = 4;
        assert(test_variants_to_test_roundtrip_result(&a, &b_ok, &b_err));
        assert(b_ok == 4);

        a.is_err = true;
        a.val.err = 5.3;
        assert(!test_variants_to_test_roundtrip_result(&a, &b_ok, &b_err));
        assert(b_err == 5);
    }

    assert(test_variants_to_test_roundtrip_enum(TEST_VARIANTS_TO_TEST_E1_A) == TEST_VARIANTS_TO_TEST_E1_A);
    assert(test_variants_to_test_roundtrip_enum(TEST_VARIANTS_TO_TEST_E1_B) == TEST_VARIANTS_TO_TEST_E1_B);

    assert(test_variants_to_test_invert_bool(true) == false);
    assert(test_variants_to_test_invert_bool(false) == true);

    {
        test_variants_to_test_casts_t c, ret;
        c.f0.tag = TEST_VARIANTS_TO_TEST_C1_A;
        c.f0.val.a = 1;
        c.f1.tag = TEST_VARIANTS_TO_TEST_C2_A;
        c.f1.val.a = 2;
        c.f2.tag = TEST_VARIANTS_TO_TEST_C3_A;
        c.f2.val.a = 3;
        c.f3.tag = TEST_VARIANTS_TO_TEST_C4_A;
        c.f3.val.a = 4;
        c.f4.tag = TEST_VARIANTS_TO_TEST_C5_A;
        c.f4.val.a = 5;
        c.f5.tag = TEST_VARIANTS_TO_TEST_C6_A;
        c.f5.val.a = 6;
        test_variants_to_test_variant_casts(&c, &ret);
        assert(ret.f0.tag == TEST_VARIANTS_TO_TEST_C1_A && ret.f0.val.a == 1);
        assert(ret.f1.tag == TEST_VARIANTS_TO_TEST_C2_A && ret.f1.val.a == 2);
        assert(ret.f2.tag == TEST_VARIANTS_TO_TEST_C3_A && ret.f2.val.a == 3);
        assert(ret.f3.tag == TEST_VARIANTS_TO_TEST_C4_A && ret.f3.val.a == 4);
        assert(ret.f4.tag == TEST_VARIANTS_TO_TEST_C5_A && ret.f4.val.a == 5);
        assert(ret.f5.tag == TEST_VARIANTS_TO_TEST_C6_A && ret.f5.val.a == 6);
    }

    {
        test_variants_to_test_casts_t c, ret;
        c.f0.tag = TEST_VARIANTS_TO_TEST_C1_B;
        c.f0.val.b = 1;
        c.f1.tag = TEST_VARIANTS_TO_TEST_C2_B;
        c.f1.val.b = 2;
        c.f2.tag = TEST_VARIANTS_TO_TEST_C3_B;
        c.f2.val.b = 3;
        c.f3.tag = TEST_VARIANTS_TO_TEST_C4_B;
        c.f3.val.b = 4;
        c.f4.tag = TEST_VARIANTS_TO_TEST_C5_B;
        c.f4.val.b = 5;
        c.f5.tag = TEST_VARIANTS_TO_TEST_C6_B;
        c.f5.val.b = 6;
        test_variants_to_test_variant_casts(&c, &ret);
        assert(ret.f0.tag == TEST_VARIANTS_TO_TEST_C1_B && ret.f0.val.b == 1);
        assert(ret.f1.tag == TEST_VARIANTS_TO_TEST_C2_B && ret.f1.val.b == 2);
        assert(ret.f2.tag == TEST_VARIANTS_TO_TEST_C3_B && ret.f2.val.b == 3);
        assert(ret.f3.tag == TEST_VARIANTS_TO_TEST_C4_B && ret.f3.val.b == 4);
        assert(ret.f4.tag == TEST_VARIANTS_TO_TEST_C5_B && ret.f4.val.b == 5);
        assert(ret.f5.tag == TEST_VARIANTS_TO_TEST_C6_B && ret.f5.val.b == 6);
    }

    {
        test_variants_to_test_zeros_t c, ret;
        c.f0.tag = TEST_VARIANTS_TO_TEST_Z1_A;
        c.f0.val.a = 1;
        c.f1.tag = TEST_VARIANTS_TO_TEST_Z2_A;
        c.f1.val.a = 2;
        c.f2.tag = TEST_VARIANTS_TO_TEST_Z3_A;
        c.f2.val.a = 3;
        c.f3.tag = TEST_VARIANTS_TO_TEST_Z4_A;
        c.f3.val.a = 4;
        test_variants_to_test_variant_zeros(&c, &ret);
        assert(ret.f0.tag == TEST_VARIANTS_TO_TEST_Z1_A && ret.f0.val.a == 1);
        assert(ret.f1.tag == TEST_VARIANTS_TO_TEST_Z2_A && ret.f1.val.a == 2);
        assert(ret.f2.tag == TEST_VARIANTS_TO_TEST_Z3_A && ret.f2.val.a == 3);
        assert(ret.f3.tag == TEST_VARIANTS_TO_TEST_Z4_A && ret.f3.val.a == 4);
    }

    {
        test_variants_to_test_zeros_t c, ret;
        c.f0.tag = TEST_VARIANTS_TO_TEST_Z1_B;
        c.f1.tag = TEST_VARIANTS_TO_TEST_Z2_B;
        c.f2.tag = TEST_VARIANTS_TO_TEST_Z3_B;
        c.f3.tag = TEST_VARIANTS_TO_TEST_Z4_B;
        test_variants_to_test_variant_zeros(&c, &ret);
        assert(ret.f0.tag == TEST_VARIANTS_TO_TEST_Z1_B);
        assert(ret.f1.tag == TEST_VARIANTS_TO_TEST_Z2_B);
        assert(ret.f2.tag == TEST_VARIANTS_TO_TEST_Z3_B);
        assert(ret.f3.tag == TEST_VARIANTS_TO_TEST_Z4_B);
    }

    {
        bool b = false;
        test_variants_to_test_result_typedef_t c;
        c.is_err = true;
        test_variants_to_test_variant_typedefs(NULL, b, &c);
    }

    {
        test_variants_to_test_tuple3_bool_result_void_void_my_errno_t ret;
        test_variants_to_test_result_void_void_t b;
        b.is_err = false;
        test_variants_to_test_variant_enums(true, &b, TEST_VARIANTS_TO_TEST_MY_ERRNO_SUCCESS, &ret);
        assert(ret.f0 == true);
        assert(!ret.f1.is_err);
        assert(ret.f2 == TEST_VARIANTS_TO_TEST_MY_ERRNO_SUCCESS);
    }

    return 0;
}
