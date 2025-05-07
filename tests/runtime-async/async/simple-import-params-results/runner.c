//@ args = '--rename a:b/i=test'

#include <assert.h>
#include <runner.h>

int main() {
  uint32_t argument = 1;
  runner_subtask_status_t status = test_async_one_argument(&argument);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);

  uint32_t result = 0xffffffff;
  status = test_async_one_result(&result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
  assert(result == 2);

  argument = 3;
  result = 0xffffffff;
  status = test_async_one_argument_and_result(&argument, &result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
  assert(result == 4);

  test_async_two_arguments_args_t arguments;
  arguments.x = 5;
  arguments.y = 6;
  status = test_async_two_arguments(&arguments);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);

  test_async_two_arguments_and_result_args_t arguments2;
  arguments2.x = 7;
  arguments2.y = 8;
  result = 0xffffffff;
  status = test_async_two_arguments_and_result(&arguments2, &result);
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
  assert(result == 9);
}
