//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <stdlib.h>
#include <test.h>

struct my_task {
  test_waitable_set_t set;
  exports_test_future_void_t future;
};

#ifdef __wasm_libcall_thread_context__
static _Thread_local struct my_task *current_task = NULL;

static void set_task(struct my_task *task) {
  current_task = task;
}

static struct my_task *get_task(void) {
  return current_task;
}
#else
static void set_task(struct my_task *task) {
  test_context_set_0(task);
}

static struct my_task *get_task(void) {
  return (struct my_task*) test_context_get_0();
}
#endif

test_callback_code_t exports_test_pending_import(exports_test_future_void_t x) {
  struct my_task *task = (struct my_task*) malloc(sizeof(struct my_task));
  assert(task != NULL);
  test_waitable_status_t status = exports_test_future_void_read(x);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);
  task->future = x;
  task->set = test_waitable_set_new();
  test_waitable_join(task->future, task->set);

  assert(get_task() == NULL);
  set_task(task);
  return TEST_CALLBACK_CODE_WAIT(task->set);
}

test_callback_code_t exports_test_pending_import_callback(test_event_t *event) {
  struct my_task *task = get_task();
  set_task(NULL);
  test_waitable_join(task->future, 0);
  if (event->event == TEST_EVENT_CANCEL) {
    assert(event->waitable == 0);
    assert(event->code == 0);

    test_waitable_status_t status = exports_test_future_void_cancel_read(task->future);
    assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_CANCELLED);
    assert(TEST_WAITABLE_COUNT(status) == 0);
    test_task_cancel();
  } else {
    assert(event->event == TEST_EVENT_FUTURE_READ);
    assert(event->waitable == task->future);
    assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
    assert(TEST_WAITABLE_COUNT(event->code) == 0);
    exports_test_pending_import_return();
  }

  exports_test_future_void_drop_readable(task->future);
  test_waitable_set_drop(task->set);

  free(task);

  return TEST_CALLBACK_CODE_EXIT;
}

void exports_test_backpressure_set(bool x) {
  if (x) {
    test_backpressure_inc();
  } else {
    test_backpressure_dec();
  }
}
