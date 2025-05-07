//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <string.h>
#include <runner.h>

int main() {
  test_future_string_writer_t writer;
  test_future_string_t reader = test_future_string_new(&writer);

  // Start the "ping" subtask
  test_async_ping_args_t args;
  args.x = reader;
  runner_string_set(&args.y, "world");
  test_future_string_t ping_result;
  runner_subtask_status_t status = test_async_ping(&args, &ping_result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
  runner_subtask_t ping = RUNNER_SUBTASK_HANDLE(status);

  // Issue a write into the future we sent to "ping"
  runner_string_t string_tmp;
  runner_string_set(&string_tmp, "hello");
  runner_waitable_status_t status2 = test_future_string_write(writer, &string_tmp);
  assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status2) == 1);
  test_future_string_close_writable(writer);

  // Wait for the subtask to complete
  runner_waitable_set_t set = runner_waitable_set_new();
  runner_waitable_join(ping, set);
  runner_event_t event;
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_SUBTASK);
  assert(event.waitable == ping);
  assert(RUNNER_SUBTASK_STATE(event.code) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(event.code) == 0);
  runner_waitable_join(ping, 0);
  runner_subtask_drop(ping);

  // Read the result from our future
  status2 = test_future_string_read(ping_result, &string_tmp);
  assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status2) == 1);
  assert(memcmp(string_tmp.ptr, "helloworld", string_tmp.len) == 0);
  test_future_string_close_readable(ping_result);

  // Start the `pong` subtask
  runner_string_t pong_result;
  reader = test_future_string_new(&writer);
  status = test_async_pong(&reader, &pong_result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
  runner_subtask_t pong = RUNNER_SUBTASK_HANDLE(status);

  // Write our string to the "pong" subtask
  status2 = test_future_string_write(writer, &string_tmp);
  assert(RUNNER_WAITABLE_STATE(status2) == RUNNER_WAITABLE_COMPLETED);
  assert(RUNNER_WAITABLE_COUNT(status2) == 1);
  runner_string_free(&string_tmp);
  test_future_string_close_writable(writer);

  // Wait for "pong" to complete
  runner_waitable_join(pong, set);
  runner_waitable_set_wait(set, &event);
  assert(event.event == RUNNER_EVENT_SUBTASK);
  assert(event.waitable == pong);
  assert(RUNNER_SUBTASK_STATE(event.code) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(event.code) == 0);
  runner_waitable_join(pong, 0);
  runner_subtask_drop(pong);

  // Assert the result of "pong"
  assert(memcmp(pong_result.ptr, "helloworld", pong_result.len) == 0);
  runner_string_free(&pong_result);

  runner_waitable_set_drop(set);
}
