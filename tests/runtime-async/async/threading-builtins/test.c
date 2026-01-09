//@ args = '--rename a:b/i=test'
//@ [lang]
//@ ldflags = "-Wl,--export-table"

#include <assert.h>
#include <test.h>
#include <stdio.h>

test_subtask_status_t exports_test_f()
{
    return TEST_CALLBACK_CODE_YIELD;
}

void thread_start(void *arg)
{
    uint32_t other_tid = *(uint32_t *)arg;
    // Call all the threading builtins; the main thread will do the right thing
    // to resume us.
    test_thread_yield();
    test_thread_yield_cancellable();
    test_thread_suspend();
    test_thread_suspend_cancellable();
    test_thread_yield_to(other_tid);
    test_thread_yield_to_cancellable(other_tid);
    test_thread_switch_to(other_tid);
    test_thread_switch_to_cancellable(other_tid);
    test_thread_resume_later(other_tid);
}

test_subtask_status_t exports_test_f_callback(test_event_t *event)
{
    assert(event->event == TEST_EVENT_NONE);
    assert(event->waitable == 0);
    assert(event->code == 0);
    uint32_t my_tid = test_thread_index();
    uint32_t other_tid = test_thread_new_indirect(thread_start, &my_tid);

    // Now drive the other thread to completion by switching/yielding to it
    test_thread_yield_to(other_tid);  // other yields
    test_thread_yield();              // other yields
    test_thread_yield();              // other suspends
    test_thread_yield_to(other_tid);  // other suspends
    test_thread_switch_to(other_tid); // other yields to me
    test_thread_suspend();            // other yields to me
    test_thread_suspend();            // other switches to me
    test_thread_switch_to(other_tid); // other switches to me
    test_thread_switch_to(other_tid); // other resumes me later and terminates

    exports_test_f_return();
    return TEST_CALLBACK_CODE_EXIT;
}
