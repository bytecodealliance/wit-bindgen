#include <assert.h>
#include <host.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <string.h>
#include <wasm.h>

static void test_integers() {
  assert(host_roundtrip_u8(1) == 1);
  assert(host_roundtrip_u8(0) == 0);
  assert(host_roundtrip_u8(UCHAR_MAX) == UCHAR_MAX);

  assert(host_roundtrip_s8(1) == 1);
  assert(host_roundtrip_s8(SCHAR_MIN) == SCHAR_MIN);
  assert(host_roundtrip_s8(SCHAR_MAX) == SCHAR_MAX);

  assert(host_roundtrip_u16(1) == 1);
  assert(host_roundtrip_u16(0) == 0);
  assert(host_roundtrip_u16(USHRT_MAX) == USHRT_MAX);

  assert(host_roundtrip_s16(1) == 1);
  assert(host_roundtrip_s16(SHRT_MIN) == SHRT_MIN);
  assert(host_roundtrip_s16(SHRT_MAX) == SHRT_MAX);

  assert(host_roundtrip_u32(1) == 1);
  assert(host_roundtrip_u32(0) == 0);
  assert(host_roundtrip_u32(UINT_MAX) == UINT_MAX);

  assert(host_roundtrip_s32(1) == 1);
  assert(host_roundtrip_s32(INT_MIN) == INT_MIN);
  assert(host_roundtrip_s32(INT_MAX) == INT_MAX);

  assert(host_roundtrip_u64(1) == 1);
  assert(host_roundtrip_u64(0) == 0);
  assert(host_roundtrip_u64(ULONG_MAX) == ULONG_MAX);

  assert(host_roundtrip_s64(1) == 1);
  assert(host_roundtrip_s64(LONG_MIN) == LONG_MIN);
  assert(host_roundtrip_s64(LONG_MAX) == LONG_MAX);

  uint8_t a;
  uint16_t b;
  host_multiple_results(&a, &b);
  assert(a == 4);
  assert(b == 5);
}

static void test_floats() {
  assert(host_roundtrip_f32(1.0) == 1.0);
  assert(host_roundtrip_f32(INFINITY) == INFINITY);
  assert(host_roundtrip_f32(-INFINITY) == -INFINITY);
  assert(isnan(host_roundtrip_f32(NAN)));

  assert(host_roundtrip_f64(1.0) == 1.0);
  assert(host_roundtrip_f64(INFINITY) == INFINITY);
  assert(host_roundtrip_f64(-INFINITY) == -INFINITY);
  assert(isnan(host_roundtrip_f64(NAN)));
}

static void test_chars() {
  assert(host_roundtrip_char('a') == 'a');
  assert(host_roundtrip_char(' ') == ' ');
  assert(host_roundtrip_char(U'🚩') == U'🚩');
}

static void test_get_set() {
  host_set_scalar(2);
  assert(host_get_scalar() == 2);
  host_set_scalar(4);
  assert(host_get_scalar() == 4);
}

static void test_records() {
  host_tuple2_u8_u32_t input;
  input.f0 = 1;
  input.f1 = 2;
  uint32_t a;
  uint8_t b;
  host_swap_tuple(&input, &a, &b);
  assert(a == 2);
  assert(b == 1);

  assert(host_roundtrip_flags1(HOST_F1_A) == HOST_F1_A);
  assert(host_roundtrip_flags1(0) == 0);
  assert(host_roundtrip_flags1(HOST_F1_B) == HOST_F1_B);
  assert(host_roundtrip_flags1(HOST_F1_A | HOST_F1_B) == (HOST_F1_A | HOST_F1_B));

  assert(host_roundtrip_flags2(HOST_F2_C) == HOST_F2_C);
  assert(host_roundtrip_flags2(0) == 0);
  assert(host_roundtrip_flags2(HOST_F2_D) == HOST_F2_D);
  assert(host_roundtrip_flags2(HOST_F2_C | HOST_F2_E) == (HOST_F2_C | HOST_F2_E));

  host_flag8_t flag8;
  host_flag16_t flag16;
  host_flag32_t flag32;
  host_flag64_t flag64;
  host_roundtrip_flags3(HOST_FLAG8_B0, HOST_FLAG16_B1, HOST_FLAG32_B2, HOST_FLAG64_B3,
      &flag8, &flag16, &flag32, &flag64);
  assert(flag8 == HOST_FLAG8_B0);
  assert(flag16 == HOST_FLAG16_B1);
  assert(flag32 == HOST_FLAG32_B2);
  assert(flag64 == HOST_FLAG64_B3);

  {
    host_r1_t a, b;
    a.a = 8;
    a.b = 0;
    host_roundtrip_record1(&a, &b);
    assert(b.a == 8);
    assert(b.b == 0);
  }

  {
    host_r1_t a, b;
    a.a = 0;
    a.b = HOST_F1_A | HOST_F1_B;
    host_roundtrip_record1(&a, &b);
    assert(b.a == 0);
    assert(b.b == (HOST_F1_A | HOST_F1_B));
  }

  host_tuple0_t t0;
  host_tuple0(&t0);

  host_tuple1_u8_t t1;
  t1.f0 = 1;
  uint8_t ret;
  host_tuple1(&t1, &ret);
  assert(ret == 1);
}

static void test_variants() {
  {
    host_option_f32_t a;
    uint8_t r;
    a.tag = 1;
    a.val = 1;
    assert(host_roundtrip_option(&a, &r) && r == 1);
    assert(r == 1);
    a.tag = 0;
    assert(!host_roundtrip_option(&a, &r));
    a.tag = 2;
    a.val = 2;
    assert(host_roundtrip_option(&a, &r) && r == 2);
  }


  {
    host_expected_u32_f32_t a;
    host_expected_f64_u8_t b;

    a.tag = 0;
    a.val.ok = 2;
    host_roundtrip_result(&a, &b);
    assert(b.tag == 0);
    assert(b.val.ok == 2.0);

    a.val.ok = 4;
    host_roundtrip_result(&a, &b);
    assert(b.tag == 0);
    assert(b.val.ok == 4);

    a.tag = 1;
    a.val.err = 5.3;
    host_roundtrip_result(&a, &b);
    assert(b.tag == 1);
    assert(b.val.err == 5);
  }

  assert(host_roundtrip_enum(HOST_E1_A) == HOST_E1_A);
  assert(host_roundtrip_enum(HOST_E1_B) == HOST_E1_B);

  assert(host_invert_bool(true) == false);
  assert(host_invert_bool(false) == true);

  {
    host_casts_t c;
    host_c1_t r1;
    host_c2_t r2;
    host_c3_t r3;
    host_c4_t r4;
    host_c5_t r5;
    host_c6_t r6;
    c.f0.tag = HOST_C1_A;
    c.f0.val.a = 1;
    c.f1.tag = HOST_C2_A;
    c.f1.val.a = 2;
    c.f2.tag = HOST_C3_A;
    c.f2.val.a = 3;
    c.f3.tag = HOST_C4_A;
    c.f3.val.a = 4;
    c.f4.tag = HOST_C5_A;
    c.f4.val.a = 5;
    c.f5.tag = HOST_C6_A;
    c.f5.val.a = 6;
    host_variant_casts(&c, &r1, &r2, &r3, &r4, &r5, &r6);
    assert(r1.tag == HOST_C1_A && r1.val.a == 1);
    assert(r2.tag == HOST_C2_A && r2.val.a == 2);
    assert(r3.tag == HOST_C3_A && r3.val.a == 3);
    assert(r4.tag == HOST_C4_A && r4.val.a == 4);
    assert(r5.tag == HOST_C5_A && r5.val.a == 5);
    assert(r6.tag == HOST_C6_A && r6.val.a == 6);
  }

  {
    host_casts_t c;
    host_c1_t r1;
    host_c2_t r2;
    host_c3_t r3;
    host_c4_t r4;
    host_c5_t r5;
    host_c6_t r6;
    c.f0.tag = HOST_C1_B;
    c.f0.val.b = 1;
    c.f1.tag = HOST_C2_B;
    c.f1.val.b = 2;
    c.f2.tag = HOST_C3_B;
    c.f2.val.b = 3;
    c.f3.tag = HOST_C4_B;
    c.f3.val.b = 4;
    c.f4.tag = HOST_C5_B;
    c.f4.val.b = 5;
    c.f5.tag = HOST_C6_B;
    c.f5.val.b = 6;
    host_variant_casts(&c, &r1, &r2, &r3, &r4, &r5, &r6);
    assert(r1.tag == HOST_C1_B && r1.val.b == 1);
    assert(r2.tag == HOST_C2_B && r2.val.b == 2);
    assert(r3.tag == HOST_C3_B && r3.val.b == 3);
    assert(r4.tag == HOST_C4_B && r4.val.b == 4);
    assert(r5.tag == HOST_C5_B && r5.val.b == 5);
    assert(r6.tag == HOST_C6_B && r6.val.b == 6);
  }

  {
    host_zeros_t c;
    host_z1_t r1;
    host_z2_t r2;
    host_z3_t r3;
    host_z4_t r4;
    c.f0.tag = HOST_Z1_A;
    c.f0.val.a = 1;
    c.f1.tag = HOST_Z2_A;
    c.f1.val.a = 2;
    c.f2.tag = HOST_Z3_A;
    c.f2.val.a = 3;
    c.f3.tag = HOST_Z4_A;
    c.f3.val.a = 4;
    host_variant_zeros(&c, &r1, &r2, &r3, &r4);
    assert(r1.tag == HOST_Z1_A && r1.val.a == 1);
    assert(r2.tag == HOST_Z2_A && r2.val.a == 2);
    assert(r3.tag == HOST_Z3_A && r3.val.a == 3);
    assert(r4.tag == HOST_Z4_A && r4.val.a == 4);
  }

  {
    host_zeros_t c;
    host_z1_t r1;
    host_z2_t r2;
    host_z3_t r3;
    host_z4_t r4;
    c.f0.tag = HOST_Z1_B;
    c.f1.tag = HOST_Z2_B;
    c.f2.tag = HOST_Z3_B;
    c.f3.tag = HOST_Z4_B;
    host_variant_zeros(&c, &r1, &r2, &r3, &r4);
    assert(r1.tag == HOST_Z1_B);
    assert(r2.tag == HOST_Z2_B);
    assert(r3.tag == HOST_Z3_B);
    assert(r4.tag == HOST_Z4_B);
  }

  {
    host_option_typedef_t a;
    a.tag = 0;
    bool b = false;
    host_result_typedef_t c;
    c.tag = 1;
    host_variant_typedefs(&a, b, &c);
  }

  {
    bool a;
    host_expected_void_void_t b;
    host_my_errno_t c;
    host_variant_enums(true, 0, HOST_MY_ERRNO_SUCCESS, &a, &b, &c);
    assert(a == false);
    assert(b == 1);
    assert(c == HOST_MY_ERRNO_A);
  }
}

static void test_lists() {
  {
    uint8_t list[] = {1, 2, 3, 4};
    host_list_u8_t a;
    a.ptr = list;
    a.len = 4;
    host_list_param(&a);
  }

  {
    host_string_t a;
    host_string_set(&a, "foo");
    host_list_param2(&a);
  }

  {
    host_string_t list[3];
    host_string_set(&list[0], "foo");
    host_string_set(&list[1], "bar");
    host_string_set(&list[2], "baz");
    host_list_string_t a;
    a.ptr = list;
    a.len = 3;
    host_list_param3(&a);
  }

  {
    host_string_t list1[2];
    host_string_t list2[1];
    host_string_set(&list1[0], "foo");
    host_string_set(&list1[1], "bar");
    host_string_set(&list2[0], "baz");
    host_list_list_string_t a;
    a.ptr[0].len = 2;
    a.ptr[0].ptr = list1;
    a.ptr[1].len = 1;
    a.ptr[1].ptr = list2;
    a.len = 2;
    host_list_param4(&a);
  }

  {
    host_list_u8_t a;
    host_list_result(&a);
    assert(a.len == 5);
    assert(memcmp(a.ptr, "\x01\x02\x03\x04\x05", 5) == 0);
    host_list_u8_free(&a);
  }

  {
    host_string_t a;
    host_list_result2(&a);
    assert(a.len == 6);
    assert(memcmp(a.ptr, "hello!", 6) == 0);
    host_string_free(&a);
  }

  {
    host_list_string_t a;
    host_list_result3(&a);
    assert(a.len == 2);
    assert(a.ptr[0].len == 6);
    assert(a.ptr[1].len == 6);
    assert(memcmp(a.ptr[0].ptr, "hello,", 6) == 0);
    assert(memcmp(a.ptr[1].ptr, "world!", 6) == 0);
    host_list_string_free(&a);
  }

  {
    host_string_t a, b;
    host_string_set(&a, "x");
    host_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    host_string_free(&b);

    host_string_set(&a, "");
    host_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    host_string_free(&b);

    host_string_set(&a, "hello");
    host_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    host_string_free(&b);

    host_string_set(&a, "hello ⚑ world");
    host_string_roundtrip(&a, &b);
    assert(b.len == a.len);
    assert(memcmp(b.ptr, a.ptr, a.len) == 0);
    host_string_free(&b);
  }
}

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
