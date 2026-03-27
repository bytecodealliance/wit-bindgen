//@ args = '--rename a:b/i=test --async=-run'
//@ wasmtime-flags = '-Wcomponent-model-async -Wcomponent-model-threading -Wcomponent-model-async-stackful'

#include <assert.h>
#include <runner.h>

void exports_runner_run()
{
    runner_subtask_status_t status = test_f();
    assert(RUNNER_SUBTASK_STATE(status) == RUNNER_SUBTASK_STARTED);
    runner_subtask_t task = RUNNER_SUBTASK_HANDLE(status);

    runner_waitable_set_t set = runner_waitable_set_new();
    runner_waitable_join(task, set);
    runner_event_t event;
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_SUBTASK);
    assert(event.waitable == task);
    assert(event.code == RUNNER_SUBTASK_RETURNED);
    runner_waitable_join(task, 0);
    runner_waitable_set_drop(set);
}
