#include <assert.h>
#include <host.h>
#include <limits.h>
#include <math.h>
#include <float.h>
#include <stdio.h>
#include <string.h>
#include <wasm.h>


static void test_flavorful() {
  {
    host_list_in_record1_t a;
    host_string_set(&a.a, "list_in_record1");
    host_list_in_record1(&a);

    host_list_in_record2_t b;
    host_list_in_record2(&b);
    assert(memcmp(b.a.ptr, "list_in_record2", b.a.len) == 0);
    host_list_in_record2_free(&b);
  }

  {
    host_list_in_record3_t a, b;
    host_string_set(&a.a, "list_in_record3 input");
    host_list_in_record3(&a, &b);
    assert(memcmp(b.a.ptr, "list_in_record3 output", b.a.len) == 0);
    host_list_in_record3_free(&b);
  }

  {
    host_list_in_record4_t a, b;
    host_string_set(&a.a, "input4");
    host_list_in_record4(&a, &b);
    assert(memcmp(b.a.ptr, "result4", b.a.len) == 0);
    host_list_in_record4_free(&b);
  }

  {
    host_list_in_variant1_1_t a;
    host_list_in_variant1_2_t b;
    host_list_in_variant1_3_t c;
    a.tag = HOST_LIST_IN_VARIANT1_1_SOME;
    host_string_set(&a.val, "foo");
    b.tag = HOST_LIST_IN_VARIANT1_2_ERR;
    host_string_set(&b.val.err, "bar");
    c.tag = HOST_LIST_IN_VARIANT1_3_0;
    host_string_set(&c.val.f0, "baz");
    host_list_in_variant1(&a, &b, &c);
  }

  {
    host_string_t a;
    assert(host_list_in_variant2(&a));
    assert(memcmp(a.ptr, "list_in_variant2", a.len) == 0);
    host_string_free(&a);
  }

  {
    host_list_in_variant3_t a;
    a.tag = HOST_LIST_IN_VARIANT3_SOME;
    host_string_set(&a.val, "input3");
    host_string_t b;
    assert(host_list_in_variant3(&a, &b));
    assert(memcmp(b.ptr, "output3", b.len) == 0);
    host_string_free(&b);
  }

  assert(host_errno_result() == HOST_MY_ERRNO_B);

  {
    host_string_t a;
    host_string_set(&a, "typedef1");
    host_string_t b_str;
    host_string_set(&b_str, "typedef2");
    host_list_typedef3_t b;
    b.ptr = &b_str;
    b.len = 1;
    host_list_typedef2_t c;
    host_list_typedef3_t d;
    host_list_typedefs(&a, &b, &c, &d);

    assert(memcmp(c.ptr, "typedef3", c.len) == 0);
    assert(d.len == 1);
    assert(memcmp(d.ptr[0].ptr, "typedef4", d.ptr[0].len) == 0);

    host_list_typedef2_free(&c);
    host_list_typedef3_free(&d);
  }

  {
    host_list_bool_t a;
    bool a_val[] = {true, false};
    a.ptr = a_val;
    a.len = 2;

    host_list_expected_void_void_t b;
    host_expected_void_void_t b_val[2];
    b_val[0] = 0;
    b_val[1] = 1;
    b.ptr = b_val;
    b.len = 2;

    host_list_my_errno_t c;
    host_my_errno_t c_val[2];
    c_val[0] = HOST_MY_ERRNO_SUCCESS;
    c_val[1] = HOST_MY_ERRNO_A;
    c.ptr = c_val;
    c.len = 2;

    host_list_bool_t d;
    host_list_expected_void_void_t e;
    host_list_my_errno_t f;
    host_list_of_variants(&a, &b, &c, &d, &e, &f);

    assert(d.len == 2);
    assert(d.ptr[0] == false);
    assert(d.ptr[1] == true);

    assert(e.len == 2);
    assert(e.ptr[0] == 1);
    assert(e.ptr[1] == 0);

    assert(f.len == 2);
    assert(f.ptr[0] == HOST_MY_ERRNO_A);
    assert(f.ptr[1] == HOST_MY_ERRNO_B);

    host_list_bool_free(&d);
    host_list_expected_void_void_free(&e);
    host_list_my_errno_free(&f);
  }
}

static void test_handles() {
  host_host_state_t s = host_host_state_create();
  assert(host_host_state_get(s) == 100);
  host_host_state_free(&s);

  assert(host_host_state2_saw_close() == false);
  host_host_state2_t s2 = host_host_state2_create();
  assert(host_host_state2_saw_close() == false);
  host_host_state2_free(&s2);
  assert(host_host_state2_saw_close() == true);

  {
    host_host_state_t a, b;
    host_host_state2_t c, d;

    a = host_host_state_create();
    c = host_host_state2_create();
    host_two_host_states(a, c, &b, &d);
    host_host_state_free(&a);
    host_host_state_free(&b);
    host_host_state2_free(&c);

    {
      host_host_state_param_record_t a;
      a.a = d;
      host_host_state2_param_record(&a);
    }
    {
      host_host_state_param_tuple_t a;
      a.f0 = d;
      host_host_state2_param_tuple(&a);
    }
    {
      host_host_state_param_option_t a;
      a.tag = 1;
      a.val = d;
      host_host_state2_param_option(&a);
    }
    {
      host_host_state_param_result_t a;
      a.tag = 0;
      a.val.ok = d;
      host_host_state2_param_result(&a);
      a.tag = 1;
      a.val.err = 2;
      host_host_state2_param_result(&a);
    }
    {
      host_host_state_param_variant_t a;
      a.tag = HOST_HOST_STATE_PARAM_VARIANT_0;
      a.val.f0 = d;
      host_host_state2_param_variant(&a);
      a.tag = HOST_HOST_STATE_PARAM_VARIANT_1;
      a.val.f1 = 2;
      host_host_state2_param_variant(&a);
    }
    {
      host_host_state2_t arr[2];
      arr[0] = d;
      arr[1] = d;
      host_list_host_state2_t list;
      list.len = 0;
      list.ptr = arr;
      host_host_state2_param_list(&list);
      list.len = 1;
      host_host_state2_param_list(&list);
      list.len = 2;
      host_host_state2_param_list(&list);
    }

    host_host_state2_free(&d);
  }

  {
    host_host_state_result_record_t a;
    host_host_state2_result_record(&a);
    host_host_state2_free(&a.a);
  }
  {
    host_host_state2_t a;
    host_host_state2_result_tuple(&a);
    host_host_state2_free(&a);
  }
  {
    host_host_state2_t a;
    assert(host_host_state2_result_option(&a));
    host_host_state2_free(&a);
  }
  {
    host_host_state_result_result_t a;
    host_host_state2_result_result(&a);
    assert(a.tag == 0);
    host_host_state2_free(&a.val.ok);
  }
  {
    host_host_state_result_variant_t a;
    host_host_state2_result_variant(&a);
    assert(a.tag == 0);
    host_host_state2_free(&a.val.f0);
  }
  {
    host_list_host_state2_t a;
    host_host_state2_result_list(&a);
    host_list_host_state2_free(&a);
  }
  {
    host_markdown2_t a = host_markdown2_create();
    host_string_t s;
    host_string_set(&s, "red is the best color");
    host_markdown2_append(a, &s);
    host_markdown2_render(a, &s);

    const char *expected = "green is the best color";
    assert(s.len == strlen(expected));
    assert(memcmp(s.ptr, expected, s.len) == 0);
    host_string_free(&s);
    host_markdown2_free(&a);
  }
}

static void test_buffers() {
  {
    host_push_buffer_u8_t push;
    uint8_t out[10];
    memset(out, 0, sizeof(out));
    push.is_handle = 0;
    push.ptr = out;
    push.len = 10;

    host_pull_buffer_u8_t pull;
    pull.is_handle = 0;
    uint8_t in[1];
    in[0] = 0;
    pull.ptr = in;
    pull.len = 1;
    uint32_t len = host_buffer_u8(&pull, &push);
    assert(len == 3);
    assert(memcmp(push.ptr, "\x01\x02\x03", 3) == 0);
    assert(memcmp(&push.ptr[3], "\0\0\0\0\0\0\0", 7) == 0);
  }

  {
    host_push_buffer_u32_t push;
    uint32_t out[10];
    memset(out, 0, sizeof(out));
    push.is_handle = 0;
    push.ptr = out;
    push.len = 10;

    host_pull_buffer_u32_t pull;
    pull.is_handle = 0;
    uint32_t in[1];
    in[0] = 0;
    pull.ptr = in;
    pull.len = 1;
    uint32_t len = host_buffer_u32(&pull, &push);
    assert(len == 3);
    assert(push.ptr[0] == 1);
    assert(push.ptr[1] == 2);
    assert(push.ptr[2] == 3);
    assert(push.ptr[3] == 0);
    assert(push.ptr[4] == 0);
    assert(push.ptr[5] == 0);
    assert(push.ptr[6] == 0);
    assert(push.ptr[7] == 0);
    assert(push.ptr[8] == 0);
    assert(push.ptr[9] == 0);
  }

  {
    host_push_buffer_bool_t push;
    host_pull_buffer_bool_t pull;
    push.is_handle = 0;
    push.len = 0;
    pull.is_handle = 0;
    pull.len = 0;
    uint32_t len = host_buffer_bool(&pull, &push);
    assert(len == 0);
  }

  {
    host_push_buffer_bool_t push;
    bool push_ptr[10];
    push.is_handle = 0;
    push.len = 10;
    push.ptr = push_ptr;

    host_pull_buffer_bool_t pull;
    bool pull_ptr[3] = {true, false, true};
    pull.is_handle = 0;
    pull.len = 3;
    pull.ptr = pull_ptr;

    uint32_t len = host_buffer_bool(&pull, &push);
    assert(len == 3);
    assert(push_ptr[0] == false);
    assert(push_ptr[1] == true);
    assert(push_ptr[2] == false);
  }

  {
    host_pull_buffer_bool_t pull;
    bool pull_ptr[5] = {true, false, true, true, false};
    pull.is_handle = 0;
    pull.len = 5;
    pull.ptr = pull_ptr;

    host_list_pull_buffer_bool_t a;
    a.len = 1;
    a.ptr = &pull;
    host_buffer_mutable1(&a);
  }

  {
    host_push_buffer_u8_t push;
    uint8_t push_ptr[10];
    push.is_handle = 0;
    push.len = 10;
    push.ptr = push_ptr;

    host_list_push_buffer_u8_t a;
    a.len = 1;
    a.ptr = &push;
    assert(host_buffer_mutable2(&a) == 4);
    assert(push_ptr[0] == 1);
    assert(push_ptr[1] == 2);
    assert(push_ptr[2] == 3);
    assert(push_ptr[3] == 4);
  }

  {
    host_push_buffer_bool_t push;
    bool push_ptr[10];
    push.is_handle = 0;
    push.len = 10;
    push.ptr = push_ptr;

    host_list_push_buffer_bool_t a;
    a.len = 1;
    a.ptr = &push;
    assert(host_buffer_mutable3(&a) == 3);
    assert(push_ptr[0] == false);
    assert(push_ptr[1] == true);
    assert(push_ptr[2] == false);
  }
}

void wasm_run_import_tests() {
  test_integers();
  test_floats();
  test_chars();
  test_get_set();
  test_records();
  test_variants();
  test_lists();
  test_flavorful();
  test_handles();
  test_buffers();
}
