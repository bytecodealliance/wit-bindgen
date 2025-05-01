//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <test.h>

// This is a test of a Rust-ism, nothing to do in C.
test_callback_code_t exports_test_async_cancel_before_read(exports_test_future_u32_t x) {
  exports_test_future_u32_close_readable(x);
  exports_test_async_cancel_before_read_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_async_cancel_before_read_callback(test_event_t *event) {
  assert(0);
}

test_callback_code_t exports_test_async_cancel_after_read(exports_test_future_u32_t x) {
  uint32_t result;
  test_waitable_status_t status = exports_test_future_u32_read(x, &result);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  status = exports_test_future_u32_cancel_read(x);
  assert(status == TEST_WAITABLE_CANCELLED);

  exports_test_future_u32_close_readable(x);

  exports_test_async_cancel_after_read_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_async_cancel_after_read_callback(test_event_t *event) {
  assert(0);
}

test_callback_code_t exports_test_async_start_read_then_cancel() {
  exports_test_future_u32_writer_t writer;
  exports_test_future_u32_t reader = exports_test_future_u32_new(&writer);

  uint32_t result;
  test_waitable_status_t status = exports_test_future_u32_read(reader, &result);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  status = exports_test_future_u32_cancel_read(reader);
  assert(status == TEST_WAITABLE_CANCELLED);

  exports_test_future_u32_close_readable(reader);
  exports_test_future_u32_close_writable(writer);

  exports_test_async_start_read_then_cancel_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_async_start_read_then_cancel_callback(test_event_t *event) {
  assert(0);
}
