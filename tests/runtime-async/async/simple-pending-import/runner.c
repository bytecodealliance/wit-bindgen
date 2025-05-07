//@ args = '--rename a:b/i=test'
#include <assert.h>
#include <runner.h>

int main() {
  runner_subtask_status_t status = test_async_f();
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
  runner_subtask_t handle = RUNNER_SUBTASK_HANDLE(status);
  assert(handle != 0);

  runner_waitable_set_t set = runner_waitable_set_new();
  runner_waitable_join(handle, set);

  runner_event_t event;
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_SUBTASK);
  assert(event.waitable == handle);
  assert(event.code == RUNNER_SUBTASK_RETURNED);

  runner_waitable_join(handle, 0);
  runner_subtask_drop(handle);

  runner_waitable_set_poll(set, &event);
  assert(event.event == RUNNER_EVENT_NONE);
  assert(event.waitable == 0);
  assert(event.code == 0);

  runner_waitable_set_drop(set);
}
