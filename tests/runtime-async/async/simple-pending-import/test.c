//@ args = '--rename a:b/i=test'

#include <assert.h>
#include <test.h>

test_subtask_status_t exports_test_async_f() {
  return TEST_CALLBACK_CODE_YIELD;
}

test_subtask_status_t exports_test_async_f_callback(test_event_t *event) {
  assert(event->event == TEST_EVENT_NONE);
  assert(event->waitable == 0);
  assert(event->code == 0);
  exports_test_async_f_return();
  return TEST_CALLBACK_CODE_EXIT;
}
