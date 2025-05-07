//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <test.h>

static test_waitable_set_t SET = 0;
static exports_test_stream_u8_t STREAM = 0;
static uint8_t BUF[2];
static uint8_t STATE = 0;

test_subtask_status_t exports_test_async_read_stream(exports_test_stream_u8_t stream) {
  test_waitable_status_t status = exports_test_stream_u8_read(stream, BUF, 1);
  assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(status) == 1);
  assert(BUF[0] == 0);

  status = exports_test_stream_u8_read(stream, BUF, 2);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  SET = test_waitable_set_new();
  STREAM = stream;
  test_waitable_join(STREAM, SET);

  return TEST_CALLBACK_CODE_WAIT(SET);
}

test_subtask_status_t exports_test_async_read_stream_callback(test_event_t *event) {
  switch (STATE++) {
    case 0:
      assert(event->event == TEST_EVENT_STREAM_READ);
      assert(event->waitable == STREAM);
      assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
      assert(TEST_WAITABLE_COUNT(event->code) == 2);

      assert(BUF[0] == 1);
      assert(BUF[1] == 2);

      // read 1/2 items
      test_waitable_status_t status = exports_test_stream_u8_read(STREAM, BUF, 1);
      assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_COMPLETED);
      assert(TEST_WAITABLE_COUNT(status) == 1);
      assert(BUF[0] == 3);

      // start the read of item 2/2
      status = exports_test_stream_u8_read(STREAM, BUF + 1, 1);
      assert(status == TEST_WAITABLE_STATUS_BLOCKED);

      return TEST_CALLBACK_CODE_WAIT(SET);

    case 1:
      // complete the read of item 2/2
      assert(event->event == TEST_EVENT_STREAM_READ);
      assert(event->waitable == STREAM);
      assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
      assert(TEST_WAITABLE_COUNT(event->code) == 1);
      assert(BUF[1] == 4);

      // clean up resources
      test_waitable_join(STREAM, 0);
      exports_test_stream_u8_close_readable(STREAM);

      test_waitable_set_drop(SET);

      exports_test_async_read_stream_return();
      return TEST_CALLBACK_CODE_EXIT;

    default:
      assert(0);
  }
}
