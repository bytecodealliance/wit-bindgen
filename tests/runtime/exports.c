#include <assert.h>
#include <stdalign.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>


wasm_wasm_state_t wasm_wasm_state_create(void) {
  return wasm_wasm_state_new((void*) 100);
}

uint32_t wasm_wasm_state_get_val(wasm_wasm_state_t a) {
  uint32_t ret = (uint32_t) wasm_wasm_state_get(&a);
  wasm_wasm_state_free(&a);
  return ret;
}

wasm_wasm_state2_t wasm_wasm_state2_create(void) {
  return wasm_wasm_state2_new((void*) 33);
}

static bool WASM_STATE2_CLOSED = false;

bool wasm_wasm_state2_saw_close(void) {
  return WASM_STATE2_CLOSED;
}

void wasm_wasm_state2_dtor(void *data) {
  WASM_STATE2_CLOSED = true;
}

void wasm_two_wasm_states(wasm_wasm_state_t a, wasm_wasm_state2_t b, wasm_wasm_state_t *ret0, wasm_wasm_state2_t *ret1) {
  wasm_wasm_state_free(&a);
  wasm_wasm_state2_free(&b);

  *ret0 = wasm_wasm_state_new((void*) 101);
  *ret1 = wasm_wasm_state2_new((void*) 102);
}

void wasm_wasm_state2_param_record(wasm_wasm_state_param_record_t *a) {
  wasm_wasm_state_param_record_free(a);
}

void wasm_wasm_state2_param_tuple(wasm_wasm_state_param_tuple_t *a) {
  wasm_wasm_state_param_tuple_free(a);
}

void wasm_wasm_state2_param_option(wasm_wasm_state_param_option_t *a) {
  wasm_wasm_state_param_option_free(a);
}

void wasm_wasm_state2_param_result(wasm_wasm_state_param_result_t *a) {
  wasm_wasm_state_param_result_free(a);
}

void wasm_wasm_state2_param_variant(wasm_wasm_state_param_variant_t *a) {
  wasm_wasm_state_param_variant_free(a);
}

void wasm_wasm_state2_param_list(wasm_list_wasm_state2_t *a) {
  wasm_list_wasm_state2_free(a);
}

void wasm_wasm_state2_result_record(wasm_wasm_state_result_record_t *ret0) {
  ret0->a = wasm_wasm_state2_new((void*) 222);
}

void wasm_wasm_state2_result_tuple(wasm_wasm_state2_t *ret0) {
  *ret0 = wasm_wasm_state2_new((void*) 333);
}

bool wasm_wasm_state2_result_option(wasm_wasm_state2_t *ret0) {
  *ret0 = wasm_wasm_state2_new((void*) 444);
  return true;
}

void wasm_wasm_state2_result_result(wasm_wasm_state_result_result_t *ret0) {
  ret0->tag = WASM_WASM_STATE_RESULT_RESULT_OK;
  ret0->val.ok = wasm_wasm_state2_new((void*) 555);
}

void wasm_wasm_state2_result_variant(wasm_wasm_state_result_variant_t *ret0) {
  ret0->tag = WASM_WASM_STATE_RESULT_VARIANT_0;
  ret0->val.f0 = wasm_wasm_state2_new((void*) 666);
}

void wasm_wasm_state2_result_list(wasm_list_wasm_state2_t *ret0) {
  ret0->len = 2;
  ret0->ptr = canonical_abi_realloc(NULL, 0, alignof(wasm_wasm_state2_t), 2 * sizeof(wasm_wasm_state2_t));
  ret0->ptr[0] = wasm_wasm_state2_new((void*) 777);
  ret0->ptr[1] = wasm_wasm_state2_new((void*) 888);
}

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

bool wasm_markdown_create(wasm_markdown_t *md) {
  return false;
}

void wasm_markdown_append(wasm_markdown_t md, wasm_string_t *s) {
  abort();
}

void wasm_markdown_render(wasm_markdown_t md, wasm_string_t *ret) {
  abort();
}
