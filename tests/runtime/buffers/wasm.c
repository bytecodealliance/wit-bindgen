#include <assert.h>
#include <imports.h>
#include <exports.h>
#include <string.h>

void exports_test_imports() {
  {
    imports_push_buffer_u8_t push;
    uint8_t out[10];
    memset(out, 0, sizeof(out));
    push.is_handle = 0;
    push.ptr = out;
    push.len = 10;

    imports_pull_buffer_u8_t pull;
    pull.is_handle = 0;
    uint8_t in[1];
    in[0] = 0;
    pull.ptr = in;
    pull.len = 1;
    uint32_t len = imports_buffer_u8(&pull, &push);
    assert(len == 3);
    assert(memcmp(push.ptr, "\x01\x02\x03", 3) == 0);
    assert(memcmp(&push.ptr[3], "\0\0\0\0\0\0\0", 7) == 0);
  }

  {
    imports_push_buffer_u32_t push;
    uint32_t out[10];
    memset(out, 0, sizeof(out));
    push.is_handle = 0;
    push.ptr = out;
    push.len = 10;

    imports_pull_buffer_u32_t pull;
    pull.is_handle = 0;
    uint32_t in[1];
    in[0] = 0;
    pull.ptr = in;
    pull.len = 1;
    uint32_t len = imports_buffer_u32(&pull, &push);
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
    imports_push_buffer_bool_t push;
    imports_pull_buffer_bool_t pull;
    push.is_handle = 0;
    push.len = 0;
    pull.is_handle = 0;
    pull.len = 0;
    uint32_t len = imports_buffer_bool(&pull, &push);
    assert(len == 0);
  }

  {
    imports_push_buffer_bool_t push;
    bool push_ptr[10];
    push.is_handle = 0;
    push.len = 10;
    push.ptr = push_ptr;

    imports_pull_buffer_bool_t pull;
    bool pull_ptr[3] = {true, false, true};
    pull.is_handle = 0;
    pull.len = 3;
    pull.ptr = pull_ptr;

    uint32_t len = imports_buffer_bool(&pull, &push);
    assert(len == 3);
    assert(push_ptr[0] == false);
    assert(push_ptr[1] == true);
    assert(push_ptr[2] == false);
  }

  {
    imports_pull_buffer_bool_t pull;
    bool pull_ptr[5] = {true, false, true, true, false};
    pull.is_handle = 0;
    pull.len = 5;
    pull.ptr = pull_ptr;

    imports_list_pull_buffer_bool_t a;
    a.len = 1;
    a.ptr = &pull;
    imports_buffer_mutable1(&a);
  }

  {
    imports_push_buffer_u8_t push;
    uint8_t push_ptr[10];
    push.is_handle = 0;
    push.len = 10;
    push.ptr = push_ptr;

    imports_list_push_buffer_u8_t a;
    a.len = 1;
    a.ptr = &push;
    assert(imports_buffer_mutable2(&a) == 4);
    assert(push_ptr[0] == 1);
    assert(push_ptr[1] == 2);
    assert(push_ptr[2] == 3);
    assert(push_ptr[3] == 4);
  }

  {
    imports_push_buffer_bool_t push;
    bool push_ptr[10];
    push.is_handle = 0;
    push.len = 10;
    push.ptr = push_ptr;

    imports_list_push_buffer_bool_t a;
    a.len = 1;
    a.ptr = &push;
    assert(imports_buffer_mutable3(&a) == 3);
    assert(push_ptr[0] == false);
    assert(push_ptr[1] == true);
    assert(push_ptr[2] == false);
  }
}

