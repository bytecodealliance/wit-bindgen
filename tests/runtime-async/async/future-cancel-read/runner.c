//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <runner.h>

/* extern runner_subtask_status_t test_async_cancel_before_read(test_future_u32_t *arg); */
/* extern runner_subtask_status_t test_async_cancel_after_read(test_future_u32_t *arg); */
/* extern runner_subtask_status_t test_async_start_read_then_cancel(void); */

int main() {
  {
    test_future_u32_writer_t writer;
    test_future_u32_t reader = test_future_u32_new(&writer);

    runner_subtask_status_t status = test_async_cancel_before_read(&reader);
    assert(status == RUNNER_SUBTASK_RETURNED);
    test_future_u32_close_writable(writer);
  }

  {
    test_future_u32_writer_t writer;
    test_future_u32_t reader = test_future_u32_new(&writer);

    runner_subtask_status_t status = test_async_cancel_after_read(&reader);
    assert(status == RUNNER_SUBTASK_RETURNED);
    test_future_u32_close_writable(writer);
  }

  {
    runner_subtask_status_t status = test_async_start_read_then_cancel();
    assert(status == RUNNER_SUBTASK_RETURNED);
  }
}
