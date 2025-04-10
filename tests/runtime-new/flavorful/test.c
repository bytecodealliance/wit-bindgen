//@ args = '--rename test:flavorful/to-test=test'

#include <assert.h>
#include <stdlib.h>
#include <string.h>

#include "test.h"

void exports_test_f_list_in_record1(exports_test_list_in_record1_t *a) {
  assert(memcmp(a->a.ptr, "list_in_record1", a->a.len) == 0);
  exports_test_list_in_record1_free(a);
}

void exports_test_f_list_in_record2(exports_test_list_in_record2_t *ret0) {
  test_string_dup(&ret0->a, "list_in_record2");
}

void exports_test_f_list_in_record3(exports_test_list_in_record3_t *a, exports_test_list_in_record3_t *ret0) {
  assert(memcmp(a->a.ptr, "list_in_record3 input", a->a.len) == 0);
  exports_test_list_in_record3_free(a);
  test_string_dup(&ret0->a, "list_in_record3 output");
}

void exports_test_f_list_in_record4(exports_test_list_in_alias_t *a, exports_test_list_in_alias_t *ret0) {
  assert(memcmp(a->a.ptr, "input4", a->a.len) == 0);
  exports_test_list_in_alias_free(a);
  test_string_dup(&ret0->a, "result4");
}

void exports_test_f_list_in_variant1(test_string_t *maybe_a, exports_test_list_in_variant1_v2_t *b) {
  assert(maybe_a != NULL);
  assert(memcmp(maybe_a->ptr, "foo", maybe_a->len) == 0);
  test_string_free(maybe_a);

  assert(b->is_err);
  assert(memcmp(b->val.err.ptr, "bar", b->val.err.len) == 0);
  exports_test_list_in_variant1_v2_free(b);
}

bool exports_test_f_list_in_variant2(test_string_t *ret0) {
  test_string_dup(ret0, "list_in_variant2");
  return true;
}

bool exports_test_f_list_in_variant3(test_string_t *maybe_a, test_string_t *ret) {
  assert(maybe_a != NULL);
  assert(memcmp(maybe_a->ptr, "input3", maybe_a->len) == 0);
  test_string_free(maybe_a);
  test_string_dup(ret, "output3");
  return true;
}

static bool RESULT_RETURNED = false;

bool exports_test_errno_result(exports_test_my_errno_t *err) {
  if (RESULT_RETURNED) {
    return true;
  } else {
    RESULT_RETURNED = true;
    *err = EXPORTS_TEST_MY_ERRNO_B;
    return false;
  }
}

void exports_test_list_typedefs(
    exports_test_list_typedef_t *a,
    exports_test_list_typedef3_t *c,
    test_tuple2_list_typedef2_list_typedef3_t *ret) {
  assert(memcmp(a->ptr, "typedef1", a->len) == 0);
  exports_test_list_typedef_free(a);

  assert(c->len == 1);
  assert(memcmp(c->ptr[0].ptr, "typedef2", c->ptr[0].len) == 0);
  exports_test_list_typedef3_free(c);

  ret->f0.ptr = (uint8_t *) malloc(8);
  ret->f0.len = 8;
  memcpy(ret->f0.ptr, "typedef3", 8);

  ret->f1.ptr = (test_string_t *) malloc(sizeof(test_string_t));
  ret->f1.len = 1;
  test_string_dup(&ret->f1.ptr[0], "typedef4");
}

void exports_test_list_of_variants(
    test_list_bool_t *a,
    exports_test_list_result_void_void_t *b,
    exports_test_list_my_errno_t *c,
    exports_test_tuple3_list_bool_list_result_void_void_list_my_errno_t *ret) {

  assert(a->len == 2);
  assert(a->ptr[0] == true);
  assert(a->ptr[1] == false);

  assert(b->len == 2);
  assert(!b->ptr[0].is_err);
  assert(b->ptr[1].is_err);

  assert(c->len == 2);
  assert(c->ptr[0] == EXPORTS_TEST_MY_ERRNO_SUCCESS);
  assert(c->ptr[1] == EXPORTS_TEST_MY_ERRNO_A);

  ret->f0.ptr = malloc(2 * sizeof(bool));
  ret->f0.len = 2;
  ret->f0.ptr[0] = false;
  ret->f0.ptr[1] = true;

  ret->f1.ptr = malloc(2 * sizeof(exports_test_result_void_void_t));
  ret->f1.len = 2;
  ret->f1.ptr[0].is_err = true;
  ret->f1.ptr[1].is_err = false;

  ret->f2.ptr = malloc(2 * sizeof(exports_test_my_errno_t));
  ret->f2.len = 2;
  ret->f2.ptr[0] = EXPORTS_TEST_MY_ERRNO_A;
  ret->f2.ptr[1] = EXPORTS_TEST_MY_ERRNO_B;
}
