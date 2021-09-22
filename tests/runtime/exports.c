#include <assert.h>
#include <stdalign.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>

// "custom allocator" which just keeps track of allocated bytes

static size_t ALLOCATED_BYTES = 0;

__attribute__((export_name("canonical_abi_realloc")))
void *canonical_abi_realloc( void *ptr, size_t orig_size, size_t orig_align, size_t new_size) {
  void *ret = realloc(ptr, new_size);
  if (!ret)
    abort();
  ALLOCATED_BYTES -= orig_size;
  ALLOCATED_BYTES += new_size;
  return ret;
}

__attribute__((export_name("canonical_abi_free")))
void canonical_abi_free(void *ptr, size_t size, size_t align) {
  if (size > 0) {
    ALLOCATED_BYTES -= size;
    free(ptr);
  }
}

uint32_t wasm_allocated_bytes(void) {
  return ALLOCATED_BYTES;
}


void wasm_multiple_results(uint8_t *ret0, uint16_t *ret1) {
  *ret0 = 100;
  *ret1 = 200;
}

void wasm_swap_tuple(wasm_tuple2_u8_u32_t *a, uint32_t *ret0, uint8_t *ret1) {
  *ret0 = a->f1;
  *ret1 = a->f0;
}

wasm_f1_t wasm_roundtrip_flags1(wasm_f1_t a) {
  return a;
}

wasm_f2_t wasm_roundtrip_flags2(wasm_f2_t a) {
  return a;
}

void wasm_roundtrip_flags3(wasm_f8_t a, wasm_f16_t b, wasm_f32_t c, wasm_f64_t d, wasm_f8_t *ret0, wasm_f16_t *ret1, wasm_f32_t *ret2, wasm_f64_t *ret3) {
  *ret0 = a;
  *ret1 = b;
  *ret2 = c;
  *ret3 = d;
}

void wasm_roundtrip_record1(wasm_r1_t *a, wasm_r1_t *ret0) {
  *ret0 = *a;
}

void wasm_tuple0(wasm_tuple0_t *a) {
}

void wasm_tuple1(wasm_tuple1_u8_t *a, uint8_t *ret0) {
  *ret0 = a->f0;
}

bool wasm_roundtrip_option(wasm_option_f32_t *a, uint8_t *ret0) {
  if (a->tag) {
    *ret0 = a->val;
  }
  return a->tag;
}

void wasm_roundtrip_result(wasm_expected_u32_f32_t *a, wasm_expected_f64_u8_t *ret0) {
  ret0->tag = a->tag;
  if (a->tag == 0) {
    ret0->val.ok = a->val.ok;
  } else {
    ret0->val.err = a->val.err;
  }
}

wasm_e1_t wasm_roundtrip_enum(wasm_e1_t a) {
  return a;
}

bool wasm_invert_bool(bool a) {
  return !a;
}

void wasm_variant_casts(wasm_casts_t *a, wasm_c1_t *ret0, wasm_c2_t *ret1, wasm_c3_t *ret2, wasm_c4_t *ret3, wasm_c5_t *ret4, wasm_c6_t *ret5) {
  *ret0 = a->f0;
  *ret1 = a->f1;
  *ret2 = a->f2;
  *ret3 = a->f3;
  *ret4 = a->f4;
  *ret5 = a->f5;
}

void wasm_variant_zeros(wasm_zeros_t *a, wasm_z1_t *ret0, wasm_z2_t *ret1, wasm_z3_t *ret2, wasm_z4_t *ret3) {
  *ret0 = a->f0;
  *ret1 = a->f1;
  *ret2 = a->f2;
  *ret3 = a->f3;
}

void wasm_variant_typedefs(wasm_option_typedef_t *a, wasm_bool_typedef_t b, wasm_result_typedef_t *c) {
}

void wasm_list_param(wasm_list_u8_t *a) {
  assert(a->len == 4);
  assert(a->ptr[0] == 1);
  assert(a->ptr[1] == 2);
  assert(a->ptr[2] == 3);
  assert(a->ptr[3] == 4);
  wasm_list_u8_free(a);
}

void wasm_list_param2(wasm_string_t *a) {
  assert(a->len == 3);
  assert(a->ptr[0] == 'f');
  assert(a->ptr[1] == 'o');
  assert(a->ptr[2] == 'o');
  wasm_string_free(a);
}

void wasm_list_param3(wasm_list_string_t *a) {
  assert(a->len == 3);
  assert(a->ptr[0].len == 3);
  assert(a->ptr[0].ptr[0] == 'f');
  assert(a->ptr[0].ptr[1] == 'o');
  assert(a->ptr[0].ptr[2] == 'o');

  assert(a->ptr[1].len == 3);
  assert(a->ptr[1].ptr[0] == 'b');
  assert(a->ptr[1].ptr[1] == 'a');
  assert(a->ptr[1].ptr[2] == 'r');

  assert(a->ptr[2].len == 3);
  assert(a->ptr[2].ptr[0] == 'b');
  assert(a->ptr[2].ptr[1] == 'a');
  assert(a->ptr[2].ptr[2] == 'z');

  wasm_list_string_free(a);
}

void wasm_list_param4(wasm_list_list_string_t *a) {
  assert(a->len == 2);
  assert(a->ptr[0].len == 2);
  assert(a->ptr[1].len == 1);

  assert(a->ptr[0].ptr[0].len == 3);
  assert(a->ptr[0].ptr[0].ptr[0] == 'f');
  assert(a->ptr[0].ptr[0].ptr[1] == 'o');
  assert(a->ptr[0].ptr[0].ptr[2] == 'o');

  assert(a->ptr[0].ptr[1].len == 3);
  assert(a->ptr[0].ptr[1].ptr[0] == 'b');
  assert(a->ptr[0].ptr[1].ptr[1] == 'a');
  assert(a->ptr[0].ptr[1].ptr[2] == 'r');

  assert(a->ptr[1].ptr[0].len == 3);
  assert(a->ptr[1].ptr[0].ptr[0] == 'b');
  assert(a->ptr[1].ptr[0].ptr[1] == 'a');
  assert(a->ptr[1].ptr[0].ptr[2] == 'z');

  wasm_list_list_string_free(a);
}

void wasm_list_result(wasm_list_u8_t *ret0) {
  ret0->ptr = canonical_abi_realloc(NULL, 0, 1, 5);
  ret0->len = 5;
  ret0->ptr[0] = 1;
  ret0->ptr[1] = 2;
  ret0->ptr[2] = 3;
  ret0->ptr[3] = 4;
  ret0->ptr[4] = 5;
}

void wasm_list_result2(wasm_string_t *ret0) {
  wasm_string_dup(ret0, "hello!");
}

void wasm_list_result3(wasm_list_string_t *ret0) {
  ret0->len = 2;
  ret0->ptr = canonical_abi_realloc(NULL, 0, alignof(wasm_string_t), 2 * sizeof(wasm_string_t));

  wasm_string_dup(&ret0->ptr[0], "hello,");
  wasm_string_dup(&ret0->ptr[1], "world!");
}

void wasm_string_roundtrip(wasm_string_t *a, wasm_string_t *ret0) {
  *ret0 = *a;
}

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
