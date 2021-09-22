#include <assert.h>
#include <stdalign.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>



void wasm_list_in_record1(wasm_list_in_record1_t *a) {
  assert(memcmp(a->a.ptr, "list_in_record1", a->a.len) == 0);
  wasm_list_in_record1_free(a);
}

void wasm_list_in_record2(wasm_list_in_record2_t *ret0) {
  wasm_string_dup(&ret0->a, "list_in_record2");
}

void wasm_list_in_record3(wasm_list_in_record3_t *a, wasm_list_in_record3_t *ret0) {
  assert(memcmp(a->a.ptr, "list_in_record3 input", a->a.len) == 0);
  wasm_list_in_record3_free(a);
  wasm_string_dup(&ret0->a, "list_in_record3 output");
}

void wasm_list_in_record4(wasm_list_in_alias_t *a, wasm_list_in_alias_t *ret0) {
  assert(memcmp(a->a.ptr, "input4", a->a.len) == 0);
  wasm_list_in_alias_free(a);
  wasm_string_dup(&ret0->a, "result4");
}

void wasm_list_in_variant1(wasm_list_in_variant1_1_t *a, wasm_list_in_variant1_2_t *b, wasm_list_in_variant1_3_t *c) {
  assert(a->tag == WASM_LIST_IN_VARIANT1_1_SOME);
  assert(memcmp(a->val.ptr, "foo", a->val.len) == 0);
  wasm_list_in_variant1_1_free(a);

  assert(b->tag == WASM_LIST_IN_VARIANT1_2_ERR);
  assert(memcmp(b->val.err.ptr, "bar", b->val.err.len) == 0);
  wasm_list_in_variant1_2_free(b);

  assert(c->tag == WASM_LIST_IN_VARIANT1_3_0);
  assert(memcmp(c->val.f0.ptr, "baz", c->val.f0.len) == 0);
  wasm_list_in_variant1_3_free(c);
}

bool wasm_list_in_variant2(wasm_string_t *ret0) {
  wasm_string_dup(ret0, "list_in_variant2");
  return true;
}

bool wasm_list_in_variant3(wasm_list_in_variant3_t *a, wasm_string_t *ret0) {
  assert(a->tag == WASM_LIST_IN_VARIANT3_SOME);
  assert(memcmp(a->val.ptr, "input3", a->val.len) == 0);
  wasm_list_in_variant3_free(a);
  wasm_string_dup(ret0, "output3");
  return true;
}

wasm_my_errno_t wasm_errno_result(void) {
  return WASM_MY_ERRNO_B;
}

void wasm_list_typedefs(wasm_list_typedef_t *a, wasm_list_typedef3_t *c, wasm_list_typedef2_t *ret0, wasm_list_typedef3_t *ret1) {
  assert(memcmp(a->ptr, "typedef1", a->len) == 0);
  wasm_list_typedef_free(a);

  assert(c->len == 1);
  assert(memcmp(c->ptr[0].ptr, "typedef2", c->ptr[0].len) == 0);
  wasm_list_typedef3_free(c);

  ret0->ptr = canonical_abi_realloc(NULL, 0, 1, 8);
  ret0->len = 8;
  memcpy(ret0->ptr, "typedef3", 8);

  ret1->ptr = canonical_abi_realloc(NULL, 0, alignof(wasm_string_t), sizeof(wasm_string_t));
  ret1->len = 1;
  wasm_string_dup(&ret1->ptr[0], "typedef4");
}
