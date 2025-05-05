//@ args = '--rename my:test/i=test'

#include <runner.h>
#include <assert.h>

int main() {
  test_stream_u8_writer_t writer;
  test_stream_u8_t reader = test_stream_u8_new(&writer);
  uint8_t buf[2];

  // write 1 item
  buf[0] = 0;
  runner_waitable_status_t status = test_stream_u8_write(writer, buf, 1);
  assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

  // Start the subtask
  runner_subtask_status_t subtask_status = test_async_read_stream(&reader);
  assert(RUNNER_SUBTASK_STATE(subtask_status) == RUNNER_SUBTASK_STARTED);
  runner_subtask_t subtask = RUNNER_SUBTASK_HANDLE(subtask_status);

  // wait for the write to complete
  runner_waitable_set_t set = runner_waitable_set_new();
  runner_waitable_join(writer, set);
  runner_event_t event;
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_STREAM_WRITE);
  assert(event.waitable == writer);
  assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(event.code) == 1);

  // write 2 items
  buf[0] = 1;
  buf[1] = 2;
  status = test_stream_u8_write(writer, buf, 2);
  assert(RUNNER_WAITABLE_STATE(status) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status) == 2);

  // write 1/2 items
  buf[0] = 3;
  buf[1] = 4;
  status = test_stream_u8_write(writer, buf, 2);
  assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_STREAM_WRITE);
  assert(event.waitable == writer);
  assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(event.code) == 1);

  // write the second item
  status = test_stream_u8_write(writer, buf + 1, 1);
  assert(RUNNER_WAITABLE_STATE(status) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status) == 1);

  // clean up the writer
  runner_waitable_join(writer, 0);
  test_stream_u8_close_writable(writer);

  // wait for the subtask to complete
  runner_waitable_join(subtask, set);
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_SUBTASK);
  assert(event.waitable == subtask);
  assert(RUNNER_SUBTASK_STATE(event.code) == RUNNER_SUBTASK_RETURNED);
  runner_waitable_join(subtask, 0);
  runner_subtask_drop(subtask);

  runner_waitable_set_drop(set);
}
