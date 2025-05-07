//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <test.h>

void exports_test_take_then_close(exports_test_future_string_t x) {
  exports_test_future_string_close_readable(x);
}

test_callback_code_t exports_test_async_read_and_drop(exports_test_future_string_t x) {
  test_string_t string;
  test_waitable_status_t status = exports_test_future_string_read(x, &string);
  assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(status) == 1);

  exports_test_future_string_close_readable(x);
  test_string_free(&string);

  exports_test_async_read_and_drop_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_async_read_and_drop_callback(test_event_t *event) {
  assert(0);
}
