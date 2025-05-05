//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <test.h>

#define PING_S1 0
#define PING_S2 1

struct ping_task {
  int state;
  test_string_t arg;
  test_string_t read_result;
  exports_test_future_string_t future;
  test_waitable_set_t set;
  exports_test_future_string_writer_t writer;
};

test_callback_code_t exports_test_async_ping(exports_test_future_string_t x, test_string_t *y) {
  // Initialize a new task
  struct ping_task *task = malloc(sizeof(struct ping_task));
  assert(task != NULL);
  memset(task, 0, sizeof(struct ping_task));
  task->state = PING_S1;
  task->arg = *y;
  task->future = x;
  task->set = test_waitable_set_new();

  // Start reading the future provided
  test_waitable_status_t status = exports_test_future_string_read(x, &task->read_result);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  // Register ourselves as waiting on the future, then block our task.
  test_waitable_join(task->future, task->set);
  test_context_set(task);
  return TEST_CALLBACK_CODE_WAIT(task->set);
}

test_callback_code_t exports_test_async_ping_callback(test_event_t *event) {
  struct ping_task *task = test_context_get();
  switch (task->state) {
    case PING_S1:
      // Assert that our future read completed and discard the read end of the
      // future.
      assert(event->event == TEST_EVENT_FUTURE_READ);
      assert(event->waitable == task->future);
      assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
      assert(TEST_WAITABLE_COUNT(event->code) == 1);
      test_waitable_join(task->future, 0);
      exports_test_future_string_close_readable(task->future);
      task->future = 0;

      // Create a new future and start the return of our task with this future.
      exports_test_future_string_writer_t writer;
      exports_test_future_string_t reader = exports_test_future_string_new(&writer);
      exports_test_async_ping_return(reader);
      task->writer = writer;

      // Concatenate `task->read_result` and `task->arg`.
      test_string_t concatenated;
      concatenated.len = task->arg.len + task->read_result.len;
      concatenated.ptr = malloc(concatenated.len);
      assert(concatenated.ptr != NULL);
      memcpy(concatenated.ptr, task->read_result.ptr, task->read_result.len);
      memcpy(concatenated.ptr + task->read_result.len, task->arg.ptr, task->arg.len);
      test_string_free(&task->arg);
      test_string_free(&task->read_result);
      task->arg = concatenated;

      // Send `task->arg`, now a concatenated string, along the future created
      // prior.
      test_waitable_status_t status = exports_test_future_string_write(writer, &task->arg);
      assert(status == TEST_WAITABLE_STATUS_BLOCKED);

      // Block and wait on the future write completing.
      task->state = PING_S2;
      test_waitable_join(writer, task->set);
      return TEST_CALLBACK_CODE_WAIT(task->set);

    case PING_S2:
      // Assert that our future write has completed, and discard the write end
      // of the future.
      assert(event->event == TEST_EVENT_FUTURE_WRITE);
      assert(event->waitable == task->writer);
      assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
      assert(TEST_WAITABLE_COUNT(event->code) == 1);
      test_waitable_join(task->writer, 0);
      exports_test_future_string_close_writable(task->writer);
      task->writer = 0;

      // Drop our waitable set, it's no longer needed.
      test_waitable_set_drop(task->set);
      task->set = 0;

      // Deallocate the string that we were sending.
      test_string_free(&task->arg);

      // And finally deallocate the task, exiting afterwards.
      free(task);
      return TEST_CALLBACK_CODE_EXIT;

    default:
      assert(0);
  }

}

struct pong_task {
  test_string_t read_result;
  exports_test_future_string_t future;
  test_waitable_set_t set;
};

test_callback_code_t exports_test_async_pong(exports_test_future_string_t x) {
  struct pong_task *task = malloc(sizeof(struct pong_task));
  assert(task != NULL);
  task->future = x;
  task->set = test_waitable_set_new();

  // Start our future read, assert it's blocked, then add this to our waitable
  // set.
  test_waitable_status_t status = exports_test_future_string_read(x, &task->read_result);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);
  test_waitable_join(task->future, task->set);

  test_context_set(task);
  return TEST_CALLBACK_CODE_WAIT(task->set);
}

test_callback_code_t exports_test_async_pong_callback(test_event_t *event) {
  struct pong_task *task = test_context_get();

  // assert this event is a future read completion
  assert(event->event == TEST_EVENT_FUTURE_READ);
  assert(event->waitable == task->future);
  assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(event->code) == 1);

  // deallocate/destroy our future
  test_waitable_join(task->future, 0);
  exports_test_future_string_close_readable(task->future);
  task->future = 0;

  // deallocate/destroy our waitable set
  test_waitable_set_drop(task->set);
  task->set = 0;

  // return our string
  exports_test_async_pong_return(task->read_result);
  test_string_free(&task->read_result);

  free(task);

  return TEST_CALLBACK_CODE_EXIT;
}
