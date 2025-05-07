//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <runner.h>
#include <stdio.h>

int main() {
  test_future_void_writer_t writer;
  test_future_void_t reader = test_future_void_new(&writer);
  runner_subtask_status_t status = test_async_pending_import(&reader);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
  runner_subtask_t subtask = RUNNER_SUBTASK_HANDLE(status);
  assert(subtask != 0);

  runner_waitable_status_t status2 = test_future_void_write(writer);
  assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status2) == 1);
  test_future_void_close_writable(writer);

  runner_waitable_set_t set = runner_waitable_set_new();
  runner_waitable_join(subtask, set);

  runner_event_t event;
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_SUBTASK);
  assert(event.waitable == subtask);
  assert(event.code == RUNNER_SUBTASK_RETURNED);

  runner_waitable_join(subtask, 0);
  runner_subtask_drop(subtask);

  runner_waitable_set_drop(set);
}
