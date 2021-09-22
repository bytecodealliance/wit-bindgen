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
