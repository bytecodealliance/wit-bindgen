//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <test.h>

test_waitable_set_t SET = 0;
exports_test_stream_void_t STREAM = 0;

test_subtask_status_t exports_test_async_read_stream(exports_test_stream_void_t stream) {
  test_waitable_status_t status = exports_test_stream_void_read(stream, 1);
  assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(status) == 1);

  status = exports_test_stream_void_read(stream, 2);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  SET = test_waitable_set_new();
  STREAM = stream;
  test_waitable_join(STREAM, SET);

  return TEST_CALLBACK_CODE_WAIT(SET);
}

test_subtask_status_t exports_test_async_read_stream_callback(test_event_t *event) {
  assert(event->event == TEST_EVENT_STREAM_READ);
  assert(event->waitable == STREAM);
  assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(event->code) == 2);

  test_waitable_join(STREAM, 0);
  exports_test_stream_void_close_readable(STREAM);

  test_waitable_set_drop(SET);

  exports_test_async_read_stream_return();
  return TEST_CALLBACK_CODE_EXIT;
}
