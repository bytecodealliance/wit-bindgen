#include <assert.h>
#include <flavorful.h>
#include <stdlib.h>
#include <string.h>

void exports_flavorful_test_imports() {
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
    a.is_some = true;
    flavorful_string_set(&a.val, "foo");
    b.is_err = true;
    flavorful_string_set(&b.val.err, "bar");
    test_flavorful_test_f_list_in_variant1(&a.val, &b);
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
    flavorful_tuple2_list_typedef2_list_typedef3_t ret;
    test_flavorful_test_list_typedefs(&a, &b, &ret);

    assert(memcmp(ret.f0.ptr, "typedef3", ret.f0.len) == 0);
    assert(ret.f1.len == 1);
    assert(memcmp(ret.f1.ptr[0].ptr, "typedef4", ret.f1.ptr[0].len) == 0);

    test_flavorful_test_list_typedef2_free(&ret.f0);
    test_flavorful_test_list_typedef3_free(&ret.f1);
  }

  {
    flavorful_list_bool_t a;
    bool a_val[] = {true, false};
    a.ptr = a_val;
    a.len = 2;

    test_flavorful_test_list_result_void_void_t b;
    test_flavorful_test_result_void_void_t b_val[2];
    b_val[0].is_err = false;
    b_val[1].is_err = true;
    b.ptr = b_val;
    b.len = 2;

    test_flavorful_test_list_my_errno_t c;
    test_flavorful_test_my_errno_t c_val[2];
    c_val[0] = TEST_FLAVORFUL_TEST_MY_ERRNO_SUCCESS;
    c_val[1] = TEST_FLAVORFUL_TEST_MY_ERRNO_A;
    c.ptr = c_val;
    c.len = 2;

    test_flavorful_test_tuple3_list_bool_list_result_void_void_list_my_errno_t ret;
    test_flavorful_test_list_of_variants(&a, &b, &c, &ret);

    assert(ret.f0.len == 2);
    assert(ret.f0.ptr[0] == false);
    assert(ret.f0.ptr[1] == true);

    assert(ret.f1.len == 2);
    assert(ret.f1.ptr[0].is_err == true);
    assert(ret.f1.ptr[1].is_err == false);

    assert(ret.f2.len == 2);
    assert(ret.f2.ptr[0] == TEST_FLAVORFUL_TEST_MY_ERRNO_A);
    assert(ret.f2.ptr[1] == TEST_FLAVORFUL_TEST_MY_ERRNO_B);

    flavorful_list_bool_free(&ret.f0);
    test_flavorful_test_list_result_void_void_free(&ret.f1);
    test_flavorful_test_list_my_errno_free(&ret.f2);
  }
}

void exports_test_flavorful_test_f_list_in_record1(exports_test_flavorful_test_list_in_record1_t *a) {
  assert(memcmp(a->a.ptr, "list_in_record1", a->a.len) == 0);
  exports_test_flavorful_test_list_in_record1_free(a);
}

void exports_test_flavorful_test_f_list_in_record2(exports_test_flavorful_test_list_in_record2_t *ret0) {
  flavorful_string_dup(&ret0->a, "list_in_record2");
}

void exports_test_flavorful_test_f_list_in_record3(exports_test_flavorful_test_list_in_record3_t *a, exports_test_flavorful_test_list_in_record3_t *ret0) {
  assert(memcmp(a->a.ptr, "list_in_record3 input", a->a.len) == 0);
  exports_test_flavorful_test_list_in_record3_free(a);
  flavorful_string_dup(&ret0->a, "list_in_record3 output");
}

void exports_test_flavorful_test_f_list_in_record4(exports_test_flavorful_test_list_in_alias_t *a, exports_test_flavorful_test_list_in_alias_t *ret0) {
  assert(memcmp(a->a.ptr, "input4", a->a.len) == 0);
  exports_test_flavorful_test_list_in_alias_free(a);
  flavorful_string_dup(&ret0->a, "result4");
}

void exports_test_flavorful_test_f_list_in_variant1(flavorful_string_t *maybe_a, exports_test_flavorful_test_list_in_variant1_v2_t *b) {
  assert(maybe_a != NULL);
  assert(memcmp(maybe_a->ptr, "foo", maybe_a->len) == 0);
  flavorful_string_free(maybe_a);

  assert(b->is_err);
  assert(memcmp(b->val.err.ptr, "bar", b->val.err.len) == 0);
  exports_test_flavorful_test_list_in_variant1_v2_free(b);
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

void exports_test_flavorful_test_list_typedefs(
    exports_test_flavorful_test_list_typedef_t *a,
    exports_test_flavorful_test_list_typedef3_t *c,
    flavorful_tuple2_list_typedef2_list_typedef3_t *ret) {
  assert(memcmp(a->ptr, "typedef1", a->len) == 0);
  test_flavorful_test_list_typedef_free(a);

  assert(c->len == 1);
  assert(memcmp(c->ptr[0].ptr, "typedef2", c->ptr[0].len) == 0);
  exports_test_flavorful_test_list_typedef3_free(c);

  ret->f0.ptr = (uint8_t *) malloc(8);
  ret->f0.len = 8;
  memcpy(ret->f0.ptr, "typedef3", 8);

  ret->f1.ptr = (flavorful_string_t *) malloc(sizeof(flavorful_string_t));
  ret->f1.len = 1;
  flavorful_string_dup(&ret->f1.ptr[0], "typedef4");
}

void exports_test_flavorful_test_list_of_variants(
    flavorful_list_bool_t *a,
    exports_test_flavorful_test_list_result_void_void_t *b,
    exports_test_flavorful_test_list_my_errno_t *c,
    exports_test_flavorful_test_tuple3_list_bool_list_result_void_void_list_my_errno_t *ret) {
  assert(0); // unimplemented
}
