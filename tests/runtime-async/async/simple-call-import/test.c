//@ args = '--rename a:b/i=test'

#include <assert.h>
#include <test.h>

test_subtask_status_t exports_test_async_f() {
  exports_test_async_f_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_f_callback(test_event_t *event) {
  assert(0);
}
