//@ args = '--rename a:b/i=test'

#include <assert.h>
#include <test.h>

test_subtask_status_t exports_test_async_one_argument(uint32_t x) {
  assert(x == 1);
  exports_test_async_one_argument_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_one_argument_callback(test_event_t *event) {
  assert(0);
}

test_subtask_status_t exports_test_async_one_result() {
  exports_test_async_one_result_return(2);
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_one_result_callback(test_event_t *event) {
  assert(0);
}

test_subtask_status_t exports_test_async_one_argument_and_result(uint32_t x) {
  assert(x == 3);
  exports_test_async_one_argument_and_result_return(4);
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_one_argument_and_result_callback(test_event_t *event) {
  assert(0);
}

test_subtask_status_t exports_test_async_two_arguments(uint32_t x, uint32_t y) {
  assert(x == 5);
  assert(y == 6);
  exports_test_async_two_arguments_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_two_arguments_callback(test_event_t *event) {
  assert(0);
}

test_subtask_status_t exports_test_async_two_arguments_and_result(uint32_t x, uint32_t y) {
  assert(x == 7);
  assert(y == 8);
  exports_test_async_two_arguments_and_result_return(9);
  return TEST_CALLBACK_CODE_EXIT;
}

test_subtask_status_t exports_test_async_two_arguments_and_result_callback(test_event_t *event) {
  assert(0);
}
