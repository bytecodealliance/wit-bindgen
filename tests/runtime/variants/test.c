#include <assert.h>
#include <test.h>
#include <stddef.h>

bool exports_test_variants_to_test_roundtrip_option(float *a, uint8_t *ret0) {
    if (a) {
        *ret0 = *a;
    }
    return a != NULL;
}

bool exports_test_variants_to_test_roundtrip_result(exports_test_variants_to_test_result_u32_f32_t *a, double *ok, uint8_t *err) {
    if (a->is_err) {
        *err = a->val.err;
        return false;
    } else {
        *ok = a->val.ok;
        return true;
    }
}

exports_test_variants_to_test_e1_t exports_test_variants_to_test_roundtrip_enum(exports_test_variants_to_test_e1_t a) {
    return a;
}

bool exports_test_variants_to_test_invert_bool(bool a) {
    return !a;
}

void exports_test_variants_to_test_variant_casts(exports_test_variants_to_test_casts_t *a, exports_test_variants_to_test_casts_t *ret) {
    *ret = *a;
}

void exports_test_variants_to_test_variant_zeros(exports_test_variants_to_test_zeros_t *a, exports_test_variants_to_test_zeros_t *b) {
    *b = *a;
}

void exports_test_variants_to_test_variant_typedefs(uint32_t *a, exports_test_variants_to_test_bool_typedef_t b, exports_test_variants_to_test_result_typedef_t *c) {
}

void exports_test_variants_to_test_variant_enums(
    bool a,
    exports_test_variants_to_test_result_void_void_t *b,
    exports_test_variants_to_test_my_errno_t c,
    exports_test_variants_to_test_tuple3_bool_result_void_void_my_errno_t *ret) {
    ret->f0 = a;
    ret->f1 = *b;
    ret->f2 = c;
}
