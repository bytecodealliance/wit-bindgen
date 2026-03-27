//@ args = '--rename a:b/i=test --async=-run'
//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner.h>

void exports_runner_run() {
  runner_subtask_status_t status = test_f();
  assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_RETURNED);
  assert(RUNNER_SUBTASK_HANDLE(status) == 0);
}
