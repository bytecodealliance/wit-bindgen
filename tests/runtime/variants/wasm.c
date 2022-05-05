#include <assert.h>
#include <imports.h>
#include <exports.h>

void exports_test_imports() {
  {
    imports_option_float32_t a;
    uint8_t r;
    a.is_some = true;
    a.val = 1;
    assert(imports_roundtrip_option(&a, &r) && r == 1);
    assert(r == 1);
    a.is_some = false;
    assert(!imports_roundtrip_option(&a, &r));
    a.is_some = true;
    a.val = 2;
    assert(imports_roundtrip_option(&a, &r) && r == 2);
  }


  {
    imports_expected_u32_float32_t a;
    imports_expected_float64_u8_t b;

    a.is_err = false;
    a.val.ok = 2;
    imports_roundtrip_result(&a, &b);
    assert(!b.is_err);
    assert(b.val.ok == 2.0);

    a.val.ok = 4;
    imports_roundtrip_result(&a, &b);
    assert(!b.is_err);
    assert(b.val.ok == 4);

    a.is_err = true;
    a.val.err = 5.3;
    imports_roundtrip_result(&a, &b);
    assert(b.is_err);
    assert(b.val.err == 5);
  }

  assert(imports_roundtrip_enum(IMPORTS_E1_A) == IMPORTS_E1_A);
  assert(imports_roundtrip_enum(IMPORTS_E1_B) == IMPORTS_E1_B);

  assert(imports_invert_bool(true) == false);
  assert(imports_invert_bool(false) == true);

  {
    imports_casts_t c;
    imports_c1_t r1;
    imports_c2_t r2;
    imports_c3_t r3;
    imports_c4_t r4;
    imports_c5_t r5;
    imports_c6_t r6;
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
    imports_variant_casts(&c, &r1, &r2, &r3, &r4, &r5, &r6);
    assert(r1.tag == IMPORTS_C1_A && r1.val.a == 1);
    assert(r2.tag == IMPORTS_C2_A && r2.val.a == 2);
    assert(r3.tag == IMPORTS_C3_A && r3.val.a == 3);
    assert(r4.tag == IMPORTS_C4_A && r4.val.a == 4);
    assert(r5.tag == IMPORTS_C5_A && r5.val.a == 5);
    assert(r6.tag == IMPORTS_C6_A && r6.val.a == 6);
  }

  {
    imports_casts_t c;
    imports_c1_t r1;
    imports_c2_t r2;
    imports_c3_t r3;
    imports_c4_t r4;
    imports_c5_t r5;
    imports_c6_t r6;
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
    imports_variant_casts(&c, &r1, &r2, &r3, &r4, &r5, &r6);
    assert(r1.tag == IMPORTS_C1_B && r1.val.b == 1);
    assert(r2.tag == IMPORTS_C2_B && r2.val.b == 2);
    assert(r3.tag == IMPORTS_C3_B && r3.val.b == 3);
    assert(r4.tag == IMPORTS_C4_B && r4.val.b == 4);
    assert(r5.tag == IMPORTS_C5_B && r5.val.b == 5);
    assert(r6.tag == IMPORTS_C6_B && r6.val.b == 6);
  }

  {
    imports_zeros_t c;
    imports_z1_t r1;
    imports_z2_t r2;
    imports_z3_t r3;
    imports_z4_t r4;
    c.f0.tag = IMPORTS_Z1_A;
    c.f0.val.a = 1;
    c.f1.tag = IMPORTS_Z2_A;
    c.f1.val.a = 2;
    c.f2.tag = IMPORTS_Z3_A;
    c.f2.val.a = 3;
    c.f3.tag = IMPORTS_Z4_A;
    c.f3.val.a = 4;
    imports_variant_zeros(&c, &r1, &r2, &r3, &r4);
    assert(r1.tag == IMPORTS_Z1_A && r1.val.a == 1);
    assert(r2.tag == IMPORTS_Z2_A && r2.val.a == 2);
    assert(r3.tag == IMPORTS_Z3_A && r3.val.a == 3);
    assert(r4.tag == IMPORTS_Z4_A && r4.val.a == 4);
  }

  {
    imports_zeros_t c;
    imports_z1_t r1;
    imports_z2_t r2;
    imports_z3_t r3;
    imports_z4_t r4;
    c.f0.tag = IMPORTS_Z1_B;
    c.f1.tag = IMPORTS_Z2_B;
    c.f2.tag = IMPORTS_Z3_B;
    c.f3.tag = IMPORTS_Z4_B;
    imports_variant_zeros(&c, &r1, &r2, &r3, &r4);
    assert(r1.tag == IMPORTS_Z1_B);
    assert(r2.tag == IMPORTS_Z2_B);
    assert(r3.tag == IMPORTS_Z3_B);
    assert(r4.tag == IMPORTS_Z4_B);
  }

  {
    imports_option_typedef_t a;
    a.is_some = false;
    bool b = false;
    imports_result_typedef_t c;
    c.is_err = true;
    imports_variant_typedefs(&a, b, &c);
  }

  {
    bool a;
    imports_expected_unit_unit_t b;
    imports_my_errno_t c;
    b.is_err = false;
    imports_variant_enums(true, &b, IMPORTS_MY_ERRNO_SUCCESS, &a, &b, &c);
    assert(a == false);
    assert(b.is_err);
    assert(c == IMPORTS_MY_ERRNO_A);
  }
}

bool exports_roundtrip_option(exports_option_float32_t *a, uint8_t *ret0) {
  if (a->is_some) {
    *ret0 = a->val;
  }
  return a->is_some;
}

void exports_roundtrip_result(exports_expected_u32_float32_t *a, exports_expected_float64_u8_t *ret0) {
  ret0->is_err = a->is_err;
  if (a->is_err) {
    ret0->val.err = a->val.err;
  } else {
    ret0->val.ok = a->val.ok;
  }
}

exports_e1_t exports_roundtrip_enum(exports_e1_t a) {
  return a;
}

bool exports_invert_bool(bool a) {
  return !a;
}

void exports_variant_casts(exports_casts_t *a, exports_c1_t *ret0, exports_c2_t *ret1, exports_c3_t *ret2, exports_c4_t *ret3, exports_c5_t *ret4, exports_c6_t *ret5) {
  *ret0 = a->f0;
  *ret1 = a->f1;
  *ret2 = a->f2;
  *ret3 = a->f3;
  *ret4 = a->f4;
  *ret5 = a->f5;
}

void exports_variant_zeros(exports_zeros_t *a, exports_z1_t *ret0, exports_z2_t *ret1, exports_z3_t *ret2, exports_z4_t *ret3) {
  *ret0 = a->f0;
  *ret1 = a->f1;
  *ret2 = a->f2;
  *ret3 = a->f3;
}

void exports_variant_typedefs(exports_option_typedef_t *a, exports_bool_typedef_t b, exports_result_typedef_t *c) {
}

