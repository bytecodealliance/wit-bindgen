#include <assert.h>
#include <exports.h>
#include <imports.h>
#include <stdlib.h>
#include <string.h>

void exports_test_imports() {
  imports_host_state_t s = imports_host_state_create();
  assert(imports_host_state_get(s) == 100);
  imports_host_state_free(&s);

  assert(imports_host_state2_saw_close() == false);
  imports_host_state2_t s2 = imports_host_state2_create();
  assert(imports_host_state2_saw_close() == false);
  imports_host_state2_free(&s2);
  assert(imports_host_state2_saw_close() == true);

  {
    imports_host_state_t a, b;
    imports_host_state2_t c, d;

    a = imports_host_state_create();
    c = imports_host_state2_create();
    imports_two_host_states(a, c, &b, &d);
    imports_host_state_free(&a);
    imports_host_state_free(&b);
    imports_host_state2_free(&c);

    {
      imports_host_state_param_record_t a;
      a.a = d;
      imports_host_state2_param_record(&a);
    }
    {
      imports_host_state_param_tuple_t a;
      a.f0 = d;
      imports_host_state2_param_tuple(&a);
    }
    {
      imports_host_state_param_option_t a;
      a.is_some = true;
      a.val = d;
      imports_host_state2_param_option(&a);
    }
    {
      imports_host_state_param_result_t a;
      a.is_err = false;
      a.val.ok = d;
      imports_host_state2_param_result(&a);
      a.is_err = true;
      a.val.err = 2;
      imports_host_state2_param_result(&a);
    }
    {
      imports_host_state_param_variant_t a;
      a.tag = 0;
      a.val.f0 = d;
      imports_host_state2_param_variant(&a);
      a.tag = 1;
      a.val.f1 = 2;
      imports_host_state2_param_variant(&a);
    }
    {
      imports_host_state2_t arr[2];
      arr[0] = d;
      arr[1] = d;
      imports_list_host_state2_t list;
      list.len = 0;
      list.ptr = arr;
      imports_host_state2_param_list(&list);
      list.len = 1;
      imports_host_state2_param_list(&list);
      list.len = 2;
      imports_host_state2_param_list(&list);
    }

    imports_host_state2_free(&d);
  }

  {
    imports_host_state_result_record_t a;
    imports_host_state2_result_record(&a);
    imports_host_state2_free(&a.a);
  }
  {
    imports_host_state2_t a;
    imports_host_state2_result_tuple(&a);
    imports_host_state2_free(&a);
  }
  {
    imports_host_state2_t a;
    assert(imports_host_state2_result_option(&a));
    imports_host_state2_free(&a);
  }
  {
    imports_host_state_result_result_t a;
    imports_host_state2_result_result(&a);
    assert(!a.is_err);
    imports_host_state2_free(&a.val.ok);
  }
  {
    imports_host_state_result_variant_t a;
    imports_host_state2_result_variant(&a);
    assert(a.tag == 0);
    imports_host_state2_free(&a.val.f0);
  }
  {
    imports_list_host_state2_t a;
    imports_host_state2_result_list(&a);
    imports_list_host_state2_free(&a);
  }
  {
    imports_markdown2_t a = imports_markdown2_create();
    imports_string_t s;
    imports_string_set(&s, "red is the best color");
    imports_markdown2_append(a, &s);
    imports_markdown2_render(a, &s);

    const char *expected = "green is the best color";
    assert(s.len == strlen(expected));
    assert(memcmp(s.ptr, expected, s.len) == 0);
    imports_string_free(&s);
    imports_markdown2_free(&a);
  }
}

exports_wasm_state_t exports_wasm_state_create(void) {
  return exports_wasm_state_new((void*) 100);
}

uint32_t exports_wasm_state_get_val(exports_wasm_state_t a) {
  uint32_t ret = (uint32_t) exports_wasm_state_get(&a);
  exports_wasm_state_free(&a);
  return ret;
}

exports_wasm_state2_t exports_wasm_state2_create(void) {
  return exports_wasm_state2_new((void*) 33);
}

static bool WASM_STATE2_CLOSED = false;

bool exports_wasm_state2_saw_close(void) {
  return WASM_STATE2_CLOSED;
}

void exports_wasm_state2_dtor(void *data) {
  WASM_STATE2_CLOSED = true;
}

void exports_two_wasm_states(exports_wasm_state_t a, exports_wasm_state2_t b, exports_wasm_state_t *ret0, exports_wasm_state2_t *ret1) {
  exports_wasm_state_free(&a);
  exports_wasm_state2_free(&b);

  *ret0 = exports_wasm_state_new((void*) 101);
  *ret1 = exports_wasm_state2_new((void*) 102);
}

void exports_wasm_state2_param_record(exports_wasm_state_param_record_t *a) {
  exports_wasm_state_param_record_free(a);
}

void exports_wasm_state2_param_tuple(exports_wasm_state_param_tuple_t *a) {
  exports_wasm_state_param_tuple_free(a);
}

void exports_wasm_state2_param_option(exports_wasm_state_param_option_t *a) {
  exports_wasm_state_param_option_free(a);
}

void exports_wasm_state2_param_result(exports_wasm_state_param_result_t *a) {
  exports_wasm_state_param_result_free(a);
}

void exports_wasm_state2_param_variant(exports_wasm_state_param_variant_t *a) {
  exports_wasm_state_param_variant_free(a);
}

void exports_wasm_state2_param_list(exports_list_wasm_state2_t *a) {
  exports_list_wasm_state2_free(a);
}

void exports_wasm_state2_result_record(exports_wasm_state_result_record_t *ret0) {
  ret0->a = exports_wasm_state2_new((void*) 222);
}

void exports_wasm_state2_result_tuple(exports_wasm_state2_t *ret0) {
  *ret0 = exports_wasm_state2_new((void*) 333);
}

bool exports_wasm_state2_result_option(exports_wasm_state2_t *ret0) {
  *ret0 = exports_wasm_state2_new((void*) 444);
  return true;
}

void exports_wasm_state2_result_result(exports_wasm_state_result_result_t *ret0) {
  ret0->is_err = false;
  ret0->val.ok = exports_wasm_state2_new((void*) 555);
}

void exports_wasm_state2_result_variant(exports_wasm_state_result_variant_t *ret0) {
  ret0->tag = 0;
  ret0->val.f0 = exports_wasm_state2_new((void*) 666);
}

void exports_wasm_state2_result_list(exports_list_wasm_state2_t *ret0) {
  ret0->len = 2;
  ret0->ptr = malloc(2 * sizeof(exports_wasm_state2_t));
  ret0->ptr[0] = exports_wasm_state2_new((void*) 777);
  ret0->ptr[1] = exports_wasm_state2_new((void*) 888);
}

bool exports_markdown_create(exports_markdown_t *md) {
  return false;
}

void exports_markdown_append(exports_markdown_t md, exports_string_t *s) {
  abort();
}

void exports_markdown_render(exports_markdown_t md, exports_string_t *ret) {
  abort();
}
