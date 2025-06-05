//@ args = '--rename a:b/i=test'

#include <assert.h>
#include <runner.h>

int main() {
  runner_subtask_status_t status = test_async_one_argument(1);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);

  uint32_t result = 0xffffffff;
  status = test_async_one_result(&result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
  assert(result == 2);

  result = 0xffffffff;
  status = test_async_one_argument_and_result(3, &result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
  assert(result == 4);

  status = test_async_two_arguments(5, 6);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);

  result = 0xffffffff;
  status = test_async_two_arguments_and_result(7, 8, &result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
  assert(result == 9);
}
