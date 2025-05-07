//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <test.h>

test_subtask_status_t exports_test_async_read_future(exports_test_future_void_t future) {
  test_waitable_status_t status = exports_test_future_void_read(future);
  assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(status) == 1);
  exports_test_future_void_close_readable(future);
  exports_test_async_read_future_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_read_future_callback(test_event_t *event) {
  assert(0);
}

test_subtask_status_t exports_test_async_close_future(exports_test_future_void_t future) {
  exports_test_future_void_close_readable(future);
  exports_test_async_close_future_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_close_future_callback(test_event_t *event) {
  assert(0);
}
