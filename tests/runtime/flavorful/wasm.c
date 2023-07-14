#include <assert.h>
#include <flavorful.h>
#include <stdlib.h>
#include <string.h>

void flavorful_test_imports() {
  {
    test_flavorful_test_list_in_record1_t a;
    flavorful_string_set(&a.a, "list_in_record1");
    test_flavorful_test_f_list_in_record1(&a);

    test_flavorful_test_list_in_record2_t b;
    test_flavorful_test_f_list_in_record2(&b);
    assert(memcmp(b.a.ptr, "list_in_record2", b.a.len) == 0);
    test_flavorful_test_list_in_record2_free(&b);
  }

  {
    test_flavorful_test_list_in_record3_t a, b;
    flavorful_string_set(&a.a, "list_in_record3 input");
    test_flavorful_test_f_list_in_record3(&a, &b);
    assert(memcmp(b.a.ptr, "list_in_record3 output", b.a.len) == 0);
    test_flavorful_test_list_in_record3_free(&b);
  }

  {
    test_flavorful_test_list_in_record4_t a, b;
    flavorful_string_set(&a.a, "input4");
    test_flavorful_test_f_list_in_record4(&a, &b);
    assert(memcmp(b.a.ptr, "result4", b.a.len) == 0);
    test_flavorful_test_list_in_record4_free(&b);
  }

  {
    test_flavorful_test_list_in_variant1_v1_t a;
    test_flavorful_test_list_in_variant1_v2_t b;
    test_flavorful_test_list_in_variant1_v3_t c;
    a.is_some = true;
    flavorful_string_set(&a.val, "foo");
    b.is_err = true;
    flavorful_string_set(&b.val.err, "bar");
    c.tag = 0;
    flavorful_string_set(&c.val.f0, "baz");
    test_flavorful_test_f_list_in_variant1(&a.val, &b, &c);
  }

  {
    flavorful_string_t a;
    assert(test_flavorful_test_f_list_in_variant2(&a));
    assert(memcmp(a.ptr, "list_in_variant2", a.len) == 0);
    flavorful_string_free(&a);
  }

  {
    test_flavorful_test_list_in_variant3_t a;
    a.is_some = true;
    flavorful_string_set(&a.val, "input3");
    flavorful_string_t b;
    assert(test_flavorful_test_f_list_in_variant3(&a.val, &b));
    assert(memcmp(b.ptr, "output3", b.len) == 0);
    flavorful_string_free(&b);
  }

  {
    test_flavorful_test_my_errno_t errno;
    assert(!test_flavorful_test_errno_result(&errno));
    assert(errno == TEST_FLAVORFUL_TEST_MY_ERRNO_B);
  }

  {
    test_flavorful_test_my_errno_t errno;
    assert(test_flavorful_test_errno_result(&errno));
  }

  {
    flavorful_string_t a;
    flavorful_string_set(&a, "typedef1");
    flavorful_string_t b_str;
    flavorful_string_set(&b_str, "typedef2");
    test_flavorful_test_list_typedef3_t b;
    b.ptr = &b_str;
    b.len = 1;
    test_flavorful_test_list_typedef2_t c;
    test_flavorful_test_list_typedef3_t d;
    test_flavorful_test_list_typedefs(&a, &b, &c, &d);

    assert(memcmp(c.ptr, "typedef3", c.len) == 0);
    assert(d.len == 1);
    assert(memcmp(d.ptr[0].ptr, "typedef4", d.ptr[0].len) == 0);

    test_flavorful_test_list_typedef2_free(&c);
    test_flavorful_test_list_typedef3_free(&d);
  }

  {
    flavorful_list_bool_t a;
    bool a_val[] = {true, false};
    a.ptr = a_val;
    a.len = 2;

    flavorful_list_result_void_void_t b;
    flavorful_result_void_void_t b_val[2];
    b_val[0].is_err = false;
    b_val[1].is_err = true;
    b.ptr = b_val;
    b.len = 2;

    flavorful_list_test_flavorful_test_my_errno_t c;
    test_flavorful_test_my_errno_t c_val[2];
    c_val[0] = TEST_FLAVORFUL_TEST_MY_ERRNO_SUCCESS;
    c_val[1] = TEST_FLAVORFUL_TEST_MY_ERRNO_A;
    c.ptr = c_val;
    c.len = 2;

    flavorful_list_bool_t d;
    flavorful_list_result_void_void_t e;
    flavorful_list_test_flavorful_test_my_errno_t f;
    test_flavorful_test_list_of_variants(&a, &b, &c, &d, &e, &f);

    assert(d.len == 2);
    assert(d.ptr[0] == false);
    assert(d.ptr[1] == true);

    assert(e.len == 2);
    assert(e.ptr[0].is_err == true);
    assert(e.ptr[1].is_err == false);

    assert(f.len == 2);
    assert(f.ptr[0] == TEST_FLAVORFUL_TEST_MY_ERRNO_A);
    assert(f.ptr[1] == TEST_FLAVORFUL_TEST_MY_ERRNO_B);

    flavorful_list_bool_free(&d);
    flavorful_list_result_void_void_free(&e);
    flavorful_list_test_flavorful_test_my_errno_free(&f);
  }
}

void exports_test_flavorful_test_f_list_in_record1(test_flavorful_test_list_in_record1_t *a) {
  assert(memcmp(a->a.ptr, "list_in_record1", a->a.len) == 0);
  test_flavorful_test_list_in_record1_free(a);
}

void exports_test_flavorful_test_f_list_in_record2(test_flavorful_test_list_in_record2_t *ret0) {
  flavorful_string_dup(&ret0->a, "list_in_record2");
}

void exports_test_flavorful_test_f_list_in_record3(test_flavorful_test_list_in_record3_t *a, test_flavorful_test_list_in_record3_t *ret0) {
  assert(memcmp(a->a.ptr, "list_in_record3 input", a->a.len) == 0);
  test_flavorful_test_list_in_record3_free(a);
  flavorful_string_dup(&ret0->a, "list_in_record3 output");
}

void exports_test_flavorful_test_f_list_in_record4(test_flavorful_test_list_in_alias_t *a, test_flavorful_test_list_in_alias_t *ret0) {
  assert(memcmp(a->a.ptr, "input4", a->a.len) == 0);
  test_flavorful_test_list_in_alias_free(a);
  flavorful_string_dup(&ret0->a, "result4");
}

void exports_test_flavorful_test_f_list_in_variant1(flavorful_string_t *maybe_a, test_flavorful_test_list_in_variant1_v2_t *b, test_flavorful_test_list_in_variant1_v3_t *c) {
  assert(maybe_a != NULL);
  assert(memcmp(maybe_a->ptr, "foo", maybe_a->len) == 0);
  flavorful_string_free(maybe_a);

  assert(b->is_err);
  assert(memcmp(b->val.err.ptr, "bar", b->val.err.len) == 0);
  test_flavorful_test_list_in_variant1_v2_free(b);

  assert(c->tag == 0);
  assert(memcmp(c->val.f0.ptr, "baz", c->val.f0.len) == 0);
  test_flavorful_test_list_in_variant1_v3_free(c);
}

bool exports_test_flavorful_test_f_list_in_variant2(flavorful_string_t *ret0) {
  flavorful_string_dup(ret0, "list_in_variant2");
  return true;
}

bool exports_test_flavorful_test_f_list_in_variant3(flavorful_string_t *maybe_a, flavorful_string_t *ret) {
  assert(maybe_a != NULL);
  assert(memcmp(maybe_a->ptr, "input3", maybe_a->len) == 0);
  flavorful_string_free(maybe_a);
  flavorful_string_dup(ret, "output3");
  return true;
}

bool exports_test_flavorful_test_errno_result(test_flavorful_test_my_errno_t *err) {
  *err = TEST_FLAVORFUL_TEST_MY_ERRNO_B;
  return false;
}

void exports_test_flavorful_test_list_typedefs(test_flavorful_test_list_typedef_t *a, test_flavorful_test_list_typedef3_t *c, test_flavorful_test_list_typedef2_t *ret0, test_flavorful_test_list_typedef3_t *ret1) {
  assert(memcmp(a->ptr, "typedef1", a->len) == 0);
  test_flavorful_test_list_typedef_free(a);

  assert(c->len == 1);
  assert(memcmp(c->ptr[0].ptr, "typedef2", c->ptr[0].len) == 0);
  test_flavorful_test_list_typedef3_free(c);

  ret0->ptr = malloc(8);
  ret0->len = 8;
  memcpy(ret0->ptr, "typedef3", 8);

  ret1->ptr = malloc(sizeof(flavorful_string_t));
  ret1->len = 1;
  flavorful_string_dup(&ret1->ptr[0], "typedef4");
}

void exports_test_flavorful_test_list_of_variants(
    flavorful_list_bool_t *a,
    flavorful_list_result_void_void_t *b,
    flavorful_list_test_flavorful_test_my_errno_t *c,
    flavorful_list_bool_t *ret0,
    flavorful_list_result_void_void_t *ret1,
    flavorful_list_test_flavorful_test_my_errno_t *ret2) {
  assert(0); // unimplemented
}
