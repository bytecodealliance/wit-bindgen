//@ args = '--rename my:test/i=test'

#include <runner.h>
#include <assert.h>

int main() {
  {
    test_future_void_writer_t writer;
    test_future_void_t reader = test_future_void_new(&writer);

    runner_waitable_status_t status = test_future_void_write(writer);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

    runner_subtask_status_t subtask = test_async_read_future(reader);
    assert(RUNNER_SUBTASK_STATE(subtask) == RUNNER_SUBTASK_RETURNED);

    runner_waitable_set_t set = runner_waitable_set_new();
    runner_waitable_join(writer, set);
    runner_event_t event;
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_FUTURE_WRITE);
    assert(event.waitable == writer);
    assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_COMPLETED);
    assert(RUNNER_WAITABLE_COUNT(event.code) == 0);

    test_future_void_drop_writable(writer);
    runner_waitable_set_drop(set);
  }

  {
    test_future_void_writer_t writer;
    test_future_void_t reader = test_future_void_new(&writer);

    runner_waitable_status_t status = test_future_void_write(writer);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

    runner_subtask_status_t subtask = test_async_drop_future(reader);
    assert(RUNNER_SUBTASK_STATE(subtask) == RUNNER_SUBTASK_RETURNED);

    runner_waitable_set_t set = runner_waitable_set_new();
    runner_waitable_join(writer, set);
    runner_event_t event;
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_FUTURE_WRITE);
    assert(event.waitable == writer);
    assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_DROPPED);
    assert(RUNNER_WAITABLE_COUNT(event.code) == 0);

    test_future_void_drop_writable(writer);
    runner_waitable_set_drop(set);
  }
}
