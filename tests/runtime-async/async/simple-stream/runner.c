//@ args = '--rename my:test/i=test'

#include <runner.h>
#include <assert.h>

int main() {
  test_stream_void_writer_t writer;
  test_stream_void_t reader = test_stream_void_new(&writer);

  // write 1 item
  runner_waitable_status_t status = test_stream_void_write(writer, 1);
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
  status = test_stream_void_write(writer, 2);
  assert(RUNNER_WAITABLE_STATE(status) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status) == 2);

  // write, but see it closed
  status = test_stream_void_write(writer, 2);
  assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_STREAM_WRITE);
  assert(event.waitable == writer);
  assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_CLOSED);
  assert(RUNNER_WAITABLE_COUNT(event.code) == 0);

  // clean up the writer
  runner_waitable_join(writer, 0);
  test_stream_void_close_writable(writer);

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
