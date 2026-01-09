//@ args = '--rename a:b/i=test --generate-threading-helpers'
//@ [lang]
//@ cflags = "-O2"
//@ ldflags = "-Wl,--export-table"

#include <assert.h>
#include <test.h>
#include <stdio.h>

test_subtask_status_t exports_test_f() {
    return TEST_CALLBACK_CODE_YIELD;
}

uint32_t main_tid = 0;
uint32_t spawned_tid = 0;

void thread_start(void* arg) {
    // Call all the threading builtins; the main thread will do the right thing
    // to resume us.
    test_thread_yield();
    test_thread_yield_cancellable();
    test_thread_suspend();
    test_thread_suspend_cancellable();
    test_thread_yield_to(main_tid);
    test_thread_yield_to_cancellable(main_tid);
    test_thread_switch_to(main_tid);
    test_thread_switch_to_cancellable(main_tid);
    test_thread_resume_later(main_tid);
}

test_subtask_status_t exports_test_f_callback(test_event_t *event) {
    assert(event->event == TEST_EVENT_NONE);
    assert(event->waitable == 0);
    assert(event->code == 0);
    main_tid = test_thread_index();
    spawned_tid = test_thread_new_indirect(thread_start, &main_tid);

    // Now drive the other thread to completion by switching/yielding to it
    test_thread_yield_to(spawned_tid);  // other yields
    test_thread_yield();              // other yields
    test_thread_yield();              // other suspends
    test_thread_yield_to(spawned_tid);  // other suspends
    test_thread_switch_to(spawned_tid); // other yields to me
    test_thread_suspend();            // other yields to me
    test_thread_suspend();            // other switches to me
    test_thread_switch_to(spawned_tid); // other switches to me
    test_thread_switch_to(spawned_tid); // other resumes me later and terminates
    exports_test_f_return();
    return TEST_CALLBACK_CODE_EXIT;
}
