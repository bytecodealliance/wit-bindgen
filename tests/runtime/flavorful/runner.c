//@ args = '--rename test:flavorful/to-test=test'

#include <assert.h>
#include <runner.h>
#include <stdlib.h>
#include <string.h>

int main() {
  {
    test_list_in_record1_t a;
    runner_string_set(&a.a, "list_in_record1");
    test_f_list_in_record1(&a);

    test_list_in_record2_t b;
    test_f_list_in_record2(&b);
    assert(memcmp(b.a.ptr, "list_in_record2", b.a.len) == 0);
    test_list_in_record2_free(&b);
  }

  {
    test_list_in_record3_t a, b;
    runner_string_set(&a.a, "list_in_record3 input");
    test_f_list_in_record3(&a, &b);
    assert(memcmp(b.a.ptr, "list_in_record3 output", b.a.len) == 0);
    test_list_in_record3_free(&b);
  }

  {
    test_list_in_record4_t a, b;
    runner_string_set(&a.a, "input4");
    test_f_list_in_record4(&a, &b);
    assert(memcmp(b.a.ptr, "result4", b.a.len) == 0);
    test_list_in_record4_free(&b);
  }

  {
    test_list_in_variant1_v1_t a;
    test_list_in_variant1_v2_t b;
    a.is_some = true;
    runner_string_set(&a.val, "foo");
    b.is_err = true;
    runner_string_set(&b.val.err, "bar");
    test_f_list_in_variant1(&a.val, &b);
  }

  {
    runner_string_t a;
    assert(test_f_list_in_variant2(&a));
    assert(memcmp(a.ptr, "list_in_variant2", a.len) == 0);
    runner_string_free(&a);
  }

  {
    test_list_in_variant3_t a;
    a.is_some = true;
    runner_string_set(&a.val, "input3");
    runner_string_t b;
    assert(test_f_list_in_variant3(&a.val, &b));
    assert(memcmp(b.ptr, "output3", b.len) == 0);
    runner_string_free(&b);
  }

  {
    test_my_errno_t errno;
    assert(!test_errno_result(&errno));
    assert(errno == TEST_MY_ERRNO_B);
  }

  {
    test_my_errno_t errno;
    assert(test_errno_result(&errno));
  }

  {
    runner_string_t a;
    runner_string_set(&a, "typedef1");
    runner_string_t b_str;
    runner_string_set(&b_str, "typedef2");
    test_list_typedef3_t b;
    b.ptr = &b_str;
    b.len = 1;
    runner_tuple2_list_typedef2_list_typedef3_t ret;
    test_list_typedefs(&a, &b, &ret);

    assert(memcmp(ret.f0.ptr, "typedef3", ret.f0.len) == 0);
    assert(ret.f1.len == 1);
    assert(memcmp(ret.f1.ptr[0].ptr, "typedef4", ret.f1.ptr[0].len) == 0);

    test_list_typedef2_free(&ret.f0);
    test_list_typedef3_free(&ret.f1);
  }

  {
    runner_list_bool_t a;
    bool a_val[] = {true, false};
    a.ptr = a_val;
    a.len = 2;

    test_list_result_void_void_t b;
    test_result_void_void_t b_val[2];
    b_val[0].is_err = false;
    b_val[1].is_err = true;
    b.ptr = b_val;
    b.len = 2;

    test_list_my_errno_t c;
    test_my_errno_t c_val[2];
    c_val[0] = TEST_MY_ERRNO_SUCCESS;
    c_val[1] = TEST_MY_ERRNO_A;
    c.ptr = c_val;
    c.len = 2;

    test_tuple3_list_bool_list_result_void_void_list_my_errno_t ret;
    test_list_of_variants(&a, &b, &c, &ret);

    assert(ret.f0.len == 2);
    assert(ret.f0.ptr[0] == false);
    assert(ret.f0.ptr[1] == true);

    assert(ret.f1.len == 2);
    assert(ret.f1.ptr[0].is_err == true);
    assert(ret.f1.ptr[1].is_err == false);

    assert(ret.f2.len == 2);
    assert(ret.f2.ptr[0] == TEST_MY_ERRNO_A);
    assert(ret.f2.ptr[1] == TEST_MY_ERRNO_B);

    runner_list_bool_free(&ret.f0);
    test_list_result_void_void_free(&ret.f1);
    test_list_my_errno_free(&ret.f2);
  }
}
