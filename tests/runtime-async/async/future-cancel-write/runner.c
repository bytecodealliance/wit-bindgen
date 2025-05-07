//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <runner.h>

int main() {
  runner_event_t event;
  runner_waitable_set_t set = runner_waitable_set_new();
  runner_string_t string;
  runner_string_set(&string, "hello");

  {
    test_future_string_writer_t writer;
    test_future_string_t reader = test_future_string_new(&writer);

    runner_waitable_status_t status = test_future_string_write(writer, &string);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);
    test_take_then_close(reader);

    runner_waitable_join(writer, set);
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_FUTURE_WRITE);
    assert(event.waitable == writer);
    assert(event.code == RUNNER_WAITABLE_CLOSED);

    runner_waitable_join(writer, 0);
    test_future_string_close_writable(writer);
  }

  {
    test_future_string_writer_t writer;
    test_future_string_t reader = test_future_string_new(&writer);

    runner_waitable_status_t status = test_future_string_write(writer, &string);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

    status = test_future_string_cancel_write(writer);
    assert(RUNNER_WAITABLE_STATE(status) == RUNNER_WAITABLE_CANCELLED);
    assert(RUNNER_WAITABLE_COUNT(status) == 0);

    test_future_string_close_writable(writer);
    test_future_string_close_readable(reader);
  }

  {
    test_future_string_writer_t writer;
    test_future_string_t reader = test_future_string_new(&writer);

    runner_waitable_status_t status = test_future_string_write(writer, &string);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

    runner_subtask_status_t status2 = test_async_read_and_drop(&reader);
    assert(status2 == RUNNER_SUBTASK_RETURNED);

    status = test_future_string_cancel_write(writer);
    assert(RUNNER_WAITABLE_STATE(status) == RUNNER_WAITABLE_COMPLETED);
    assert(RUNNER_WAITABLE_COUNT(status) == 1);

    test_future_string_close_writable(writer);
  }

  runner_waitable_set_drop(set);
}
