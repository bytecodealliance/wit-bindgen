//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <test.h>
#include <stdlib.h>

// This is a test of a Rust-ism, nothing to do in C.
test_callback_code_t exports_test_async_cancel_before_read(exports_test_future_u32_t x) {
  exports_test_future_u32_drop_readable(x);
  exports_test_async_cancel_before_read_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_async_cancel_before_read_callback(test_event_t *event) {
  assert(0);
}

test_callback_code_t exports_test_async_cancel_after_read(exports_test_future_u32_t x) {
  uint32_t result;
  test_waitable_status_t status = exports_test_future_u32_read(x, &result);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  status = exports_test_future_u32_cancel_read(x);
  assert(status == TEST_WAITABLE_CANCELLED);

  exports_test_future_u32_drop_readable(x);

  exports_test_async_cancel_after_read_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_async_cancel_after_read_callback(test_event_t *event) {
  assert(0);
}

struct start_read_then_cancel_state {
  exports_test_future_u32_t data;
  exports_test_future_void_t signal;
  test_waitable_set_t set;
  uint32_t result;
};

test_callback_code_t exports_test_async_start_read_then_cancel(
  exports_test_future_u32_t data,
  exports_test_future_void_t signal
) {
  struct start_read_then_cancel_state *state = malloc(sizeof(struct start_read_then_cancel_state));
  assert(state != NULL);
  state->data = data;
  state->signal = signal;
  state->set = test_waitable_set_new();;
  test_waitable_status_t status = exports_test_future_u32_read(data, &state->result);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  status = exports_test_future_void_read(signal);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);

  test_waitable_join(signal, state->set);

  test_context_set(state);
  return TEST_CALLBACK_CODE_WAIT(state->set);
}

test_callback_code_t exports_test_async_start_read_then_cancel_callback(test_event_t *event) {
  struct start_read_then_cancel_state *state = test_context_get();
  assert(event->event == TEST_EVENT_FUTURE_READ);
  assert(event->waitable == state->signal);
  assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(event->code) == 0);

  test_waitable_status_t status = exports_test_future_u32_cancel_read(state->data);
  assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_COMPLETED);
  assert(TEST_WAITABLE_COUNT(status) == 0);
  assert(state->result == 4);

  test_waitable_join(state->signal, 0);
  exports_test_future_u32_drop_readable(state->data);
  exports_test_future_void_drop_readable(state->signal);
  test_waitable_set_drop(state->set);

  exports_test_async_start_read_then_cancel_return();
  return TEST_CALLBACK_CODE_EXIT;
}
