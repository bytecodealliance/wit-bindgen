//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <runner.h>

int main() {
  {
    test_future_u32_writer_t writer;
    test_future_u32_t reader = test_future_u32_new(&writer);

    runner_subtask_status_t status = test_async_cancel_before_read(reader);
    assert(status == RUNNER_SUBTASK_RETURNED);
    uint32_t value = 0;
    runner_waitable_status_t wstatus = test_future_u32_write(writer, &value);
    assert(RUNNER_WAITABLE_STATE(wstatus) == RUNNER_WAITABLE_DROPPED);
    assert(RUNNER_WAITABLE_COUNT(wstatus) == 0);
    test_future_u32_drop_writable(writer);
  }

  {
    test_future_u32_writer_t writer;
    test_future_u32_t reader = test_future_u32_new(&writer);

    runner_subtask_status_t status = test_async_cancel_after_read(reader);
    assert(status == RUNNER_SUBTASK_RETURNED);

    uint32_t value = 0;
    runner_waitable_status_t wstatus = test_future_u32_write(writer, &value);
    assert(RUNNER_WAITABLE_STATE(wstatus) == RUNNER_WAITABLE_DROPPED);
    assert(RUNNER_WAITABLE_COUNT(wstatus) == 0);
    test_future_u32_drop_writable(writer);
  }

  {
    test_future_u32_writer_t data_writer;
    test_future_u32_t data_reader = test_future_u32_new(&data_writer);
    test_future_void_writer_t signal_writer;
    test_future_void_t signal_reader = test_future_void_new(&signal_writer);
    runner_subtask_status_t status = test_async_start_read_then_cancel(data_reader, signal_reader);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
    runner_subtask_t task = RUNNER_SUBTASK_HANDLE(status);

    uint32_t to_write = 4;
    runner_waitable_status_t wstatus = test_future_u32_write(data_writer, &to_write);
    assert(RUNNER_WAITABLE_STATE(wstatus) == RUNNER_WAITABLE_COMPLETED);
    assert(RUNNER_WAITABLE_COUNT(wstatus) == 0);

    wstatus = test_future_void_write(signal_writer);
    assert(RUNNER_WAITABLE_STATE(wstatus) == RUNNER_WAITABLE_COMPLETED);
    assert(RUNNER_WAITABLE_COUNT(wstatus) == 0);

    runner_waitable_set_t set = runner_waitable_set_new();
    runner_waitable_join(task, set);

    runner_event_t event;
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_SUBTASK);
    assert(event.waitable == task);
    assert(event.code == RUNNER_SUBTASK_RETURNED);

    runner_waitable_join(task, 0);
    runner_subtask_drop(task);
    runner_waitable_set_drop(set);
  }
}
