//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <runner.h>

int main() {
  // Call an import and cancel it.
  {
    test_future_void_writer_t writer;
    test_future_void_t reader = test_future_void_new(&writer);
    runner_subtask_status_t status = test_async_pending_import(&reader);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
    runner_subtask_t subtask = RUNNER_SUBTASK_HANDLE(status);
    assert(subtask != 0);
    status = runner_subtask_cancel(subtask);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED_CANCELLED);
    assert(RUNNER_SUBTASK_HANDLE(status) == 0);

    runner_waitable_status_t status2 = test_future_void_write(writer);
    assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_CLOSED);
    assert(RUNNER_WAITABLE_COUNT(status2) == 0);
    test_future_void_close_writable(writer);
  }

  // One import in "started", one in "starting", then cancel both.
  {
    test_future_void_writer_t writer1;
    test_future_void_t reader1 = test_future_void_new(&writer1);
    test_future_void_writer_t writer2;
    test_future_void_t reader2 = test_future_void_new(&writer2);

    // start up one task, it'll be in "STARTED"
    runner_subtask_status_t status = test_async_pending_import(&reader1);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
    runner_subtask_t subtask1 = RUNNER_SUBTASK_HANDLE(status);
    assert(subtask1 != 0);

    // Start up a second task after setting the backpressure flag, forcing it
    // to be in the "STARTING" state.
    test_backpressure_set(true);
    status = test_async_pending_import(&reader2);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTING);
    runner_subtask_t subtask2 = RUNNER_SUBTASK_HANDLE(status);
    assert(subtask2 != 0);

    // Now cancel both tasks, witnessing slightly different cancellation codes.
    status = runner_subtask_cancel(subtask1);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED_CANCELLED);
    assert(RUNNER_SUBTASK_HANDLE(status) == 0);
    status = runner_subtask_cancel(subtask2);
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED_CANCELLED);
    assert(RUNNER_SUBTASK_HANDLE(status) == 0);

    // We still own the readable end of `reader2` and `writer2` since the
    // subtask didn't actually start, so close it here.
    test_future_void_close_readable(reader2);

    // Assert both read ends are closed from the POV of the write ends
    runner_waitable_status_t status2 = test_future_void_write(writer1);
    assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_CLOSED);
    assert(RUNNER_WAITABLE_COUNT(status2) == 0);
    test_future_void_close_writable(writer1);

    status2 = test_future_void_write(writer2);
    assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_CLOSED);
    assert(RUNNER_WAITABLE_COUNT(status2) == 0);
    test_future_void_close_writable(writer2);

    // reset the backpressure flag
    test_backpressure_set(false);
  }
}
