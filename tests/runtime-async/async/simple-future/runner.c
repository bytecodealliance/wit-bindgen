//@ args = '--rename my:test/i=test'

#include <runner.h>
#include <assert.h>

/* include!(env!("BINDINGS")); */

/* use crate::my::test::i::*; */

/* fn main() { */
/*     wit_bindgen::block_on(async { */
/*         let (tx, rx) = wit_future::new(); */
/*         let (res, ()) = futures::join!(tx.write(()), read_future(rx)); */
/*         assert!(res.is_ok()); */

/*         let (tx, rx) = wit_future::new(); */
/*         let (res, ()) = futures::join!(tx.write(()), close_future(rx)); */
/*         assert!(res.is_err()); */
/*     }); */
/* } */


int main() {
  {
    test_future_void_writer_t writer;
    test_future_void_t reader = test_future_void_new(&writer);

    runner_waitable_status_t status = test_future_void_write(writer);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

    runner_subtask_status_t subtask = test_async_read_future(&reader);
    assert(RUNNER_SUBTASK_STATE(subtask) == RUNNER_SUBTASK_RETURNED);

    runner_waitable_set_t set = runner_waitable_set_new();
    runner_waitable_join(writer, set);
    runner_event_t event;
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_FUTURE_WRITE);
    assert(event.waitable == writer);
    assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_COMPLETED);
    assert(RUNNER_WAITABLE_COUNT(event.code) == 1);

    test_future_void_close_writable(writer);
    runner_waitable_set_drop(set);
  }

  {
    test_future_void_writer_t writer;
    test_future_void_t reader = test_future_void_new(&writer);

    runner_waitable_status_t status = test_future_void_write(writer);
    assert(status == RUNNER_WAITABLE_STATUS_BLOCKED);

    runner_subtask_status_t subtask = test_async_close_future(&reader);
    assert(RUNNER_SUBTASK_STATE(subtask) == RUNNER_SUBTASK_RETURNED);

    runner_waitable_set_t set = runner_waitable_set_new();
    runner_waitable_join(writer, set);
    runner_event_t event;
    runner_waitable_set_wait(set, &event);
    assert(event.event == RUNNER_EVENT_FUTURE_WRITE);
    assert(event.waitable == writer);
    assert(RUNNER_WAITABLE_STATE(event.code) == RUNNER_WAITABLE_CLOSED);
    assert(RUNNER_WAITABLE_COUNT(event.code) == 0);

    test_future_void_close_writable(writer);
    runner_waitable_set_drop(set);
  }
}
