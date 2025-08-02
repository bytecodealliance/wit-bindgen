//@ args = '--rename a:b/i=test'
#include <assert.h>
#include <runner.h>

int main() {
  runner_subtask_status_t status = test_async_f();
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
}
