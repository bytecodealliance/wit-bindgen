#include <assert.h>
#include <variants.h>

void variants_test_imports() {
  {
    float a = 1;
    uint8_t r;
    assert(imports_roundtrip_option(&a, &r) && r == 1);
    assert(r == 1);
    assert(!imports_roundtrip_option(NULL, &r));
    a = 2;
    assert(imports_roundtrip_option(&a, &r) && r == 2);
  }


  {
    imports_result_u32_float32_t a;
    double b_ok;
    uint8_t b_err;

    a.is_err = false;
    a.val.ok = 2;
    assert(imports_roundtrip_result(&a, &b_ok, &b_err));
    assert(b_ok == 2.0);

    a.val.ok = 4;
    assert(imports_roundtrip_result(&a, &b_ok, &b_err));
    assert(b_ok == 4);

    a.is_err = true;
    a.val.err = 5.3;
    assert(!imports_roundtrip_result(&a, &b_ok, &b_err));
    assert(b_err == 5);
  }

  assert(imports_roundtrip_enum(IMPORTS_E1_A) == IMPORTS_E1_A);
  assert(imports_roundtrip_enum(IMPORTS_E1_B) == IMPORTS_E1_B);

  assert(imports_invert_bool(true) == false);
  assert(imports_invert_bool(false) == true);

  {
    imports_casts_t c, ret;
    c.f0.tag = IMPORTS_C1_A;
    c.f0.val.a = 1;
    c.f1.tag = IMPORTS_C2_A;
    c.f1.val.a = 2;
    c.f2.tag = IMPORTS_C3_A;
    c.f2.val.a = 3;
    c.f3.tag = IMPORTS_C4_A;
    c.f3.val.a = 4;
    c.f4.tag = IMPORTS_C5_A;
    c.f4.val.a = 5;
    c.f5.tag = IMPORTS_C6_A;
    c.f5.val.a = 6;
    imports_variant_casts(&c, &ret);
    assert(ret.f0.tag == IMPORTS_C1_A && ret.f0.val.a == 1);
    assert(ret.f1.tag == IMPORTS_C2_A && ret.f1.val.a == 2);
    assert(ret.f2.tag == IMPORTS_C3_A && ret.f2.val.a == 3);
    assert(ret.f3.tag == IMPORTS_C4_A && ret.f3.val.a == 4);
    assert(ret.f4.tag == IMPORTS_C5_A && ret.f4.val.a == 5);
    assert(ret.f5.tag == IMPORTS_C6_A && ret.f5.val.a == 6);
  }

  {
    imports_casts_t c, ret;
    c.f0.tag = IMPORTS_C1_B;
    c.f0.val.b = 1;
    c.f1.tag = IMPORTS_C2_B;
    c.f1.val.b = 2;
    c.f2.tag = IMPORTS_C3_B;
    c.f2.val.b = 3;
    c.f3.tag = IMPORTS_C4_B;
    c.f3.val.b = 4;
    c.f4.tag = IMPORTS_C5_B;
    c.f4.val.b = 5;
    c.f5.tag = IMPORTS_C6_B;
    c.f5.val.b = 6;
    imports_variant_casts(&c, &ret);
    assert(ret.f0.tag == IMPORTS_C1_B && ret.f0.val.b == 1);
    assert(ret.f1.tag == IMPORTS_C2_B && ret.f1.val.b == 2);
    assert(ret.f2.tag == IMPORTS_C3_B && ret.f2.val.b == 3);
    assert(ret.f3.tag == IMPORTS_C4_B && ret.f3.val.b == 4);
    assert(ret.f4.tag == IMPORTS_C5_B && ret.f4.val.b == 5);
    assert(ret.f5.tag == IMPORTS_C6_B && ret.f5.val.b == 6);
  }

  {
    imports_zeros_t c, ret;
    c.f0.tag = IMPORTS_Z1_A;
    c.f0.val.a = 1;
    c.f1.tag = IMPORTS_Z2_A;
    c.f1.val.a = 2;
    c.f2.tag = IMPORTS_Z3_A;
    c.f2.val.a = 3;
    c.f3.tag = IMPORTS_Z4_A;
    c.f3.val.a = 4;
    imports_variant_zeros(&c, &ret);
    assert(ret.f0.tag == IMPORTS_Z1_A && ret.f0.val.a == 1);
    assert(ret.f1.tag == IMPORTS_Z2_A && ret.f1.val.a == 2);
    assert(ret.f2.tag == IMPORTS_Z3_A && ret.f2.val.a == 3);
    assert(ret.f3.tag == IMPORTS_Z4_A && ret.f3.val.a == 4);
  }

  {
    imports_zeros_t c, ret;
    c.f0.tag = IMPORTS_Z1_B;
    c.f1.tag = IMPORTS_Z2_B;
    c.f2.tag = IMPORTS_Z3_B;
    c.f3.tag = IMPORTS_Z4_B;
    imports_variant_zeros(&c, &ret);
    assert(ret.f0.tag == IMPORTS_Z1_B);
    assert(ret.f1.tag == IMPORTS_Z2_B);
    assert(ret.f2.tag == IMPORTS_Z3_B);
    assert(ret.f3.tag == IMPORTS_Z4_B);
  }

  {
    bool b = false;
    imports_result_typedef_t c;
    c.is_err = true;
    imports_variant_typedefs(NULL, b, &c);
  }

  {
    imports_tuple3_bool_result_void_void_my_errno_t ret;
    imports_result_void_void_t b;
    b.is_err = false;
    imports_variant_enums(true, &b, IMPORTS_MY_ERRNO_SUCCESS, &ret);
    assert(ret.f0 == false);
    assert(ret.f1.is_err);
    assert(ret.f2 == IMPORTS_MY_ERRNO_A);
  }
}

bool variants_roundtrip_option(float *a, uint8_t *ret0) {
  if (a) {
    *ret0 = *a;
  }
  return a != NULL;
}

bool variants_roundtrip_result(variants_result_u32_float32_t *a, double *ok, uint8_t *err) {
  if (a->is_err) {
    *err = a->val.err;
    return false;
  } else {
    *ok = a->val.ok;
    return true;
  }
}

variants_e1_t variants_roundtrip_enum(variants_e1_t a) {
  return a;
}

bool variants_invert_bool(bool a) {
  return !a;
}

void variants_variant_casts(variants_casts_t *a, variants_casts_t *ret) {
  *ret = *a;
}

void variants_variant_zeros(variants_zeros_t *a, variants_zeros_t *b) {
  *b = *a;
}

void variants_variant_typedefs(uint32_t *a, variants_bool_typedef_t b, variants_result_typedef_t *c) {
}

