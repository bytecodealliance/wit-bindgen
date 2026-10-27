//@ args = '--rename my:test/i=test'

#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <test.h>

// Checks that `s` holds the one letter `c`, then frees it: the callee owns
// the bytes of every string it receives.
static void expect_letter(test_string_t *s, char c) {
  assert(s->len == 1);
  assert(s->ptr[0] == (uint8_t) c);
  test_string_free(s);
}

test_callback_code_t exports_test_many_arguments(
    test_string_t *a, test_string_t *b, test_string_t *c, test_string_t *d,
    test_string_t *e, test_string_t *f, test_string_t *g, test_string_t *h,
    test_string_t *i) {
  expect_letter(a, 'a');
  expect_letter(b, 'b');
  expect_letter(c, 'c');
  expect_letter(d, 'd');
  expect_letter(e, 'e');
  expect_letter(f, 'f');
  expect_letter(g, 'g');
  expect_letter(h, 'h');
  expect_letter(i, 'i');
  exports_test_many_arguments_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_many_arguments_callback(test_event_t *event) {
  assert(0);
}

struct my_task {
  test_waitable_set_t set;
  exports_test_future_void_t future;
};

#ifdef __wasm_libcall_thread_context__
static _Thread_local struct my_task *current_task = NULL;

static void set_task(struct my_task *task) {
  current_task = task;
}

static struct my_task *get_task(void) {
  return current_task;
}
#else
static void set_task(struct my_task *task) {
  test_context_set_0(task);
}

static struct my_task *get_task(void) {
  return (struct my_task*) test_context_get_0();
}
#endif

test_callback_code_t exports_test_many_arguments_pending(
    exports_test_future_void_t x, test_string_t *a, test_string_t *b,
    test_string_t *c, test_string_t *d, test_string_t *e, test_string_t *f,
    test_string_t *g, test_string_t *h) {
  expect_letter(a, 'a');
  expect_letter(b, 'b');
  expect_letter(c, 'c');
  expect_letter(d, 'd');
  expect_letter(e, 'e');
  expect_letter(f, 'f');
  expect_letter(g, 'g');
  expect_letter(h, 'h');

  struct my_task *task = (struct my_task*) malloc(sizeof(struct my_task));
  assert(task != NULL);
  test_waitable_status_t status = exports_test_future_void_read(x);
  assert(status == TEST_WAITABLE_STATUS_BLOCKED);
  task->future = x;
  task->set = test_waitable_set_new();
  test_waitable_join(task->future, task->set);

  assert(get_task() == NULL);
  set_task(task);
  return TEST_CALLBACK_CODE_WAIT(task->set);
}

test_callback_code_t exports_test_many_arguments_pending_callback(test_event_t *event) {
  struct my_task *task = get_task();
  set_task(NULL);
  test_waitable_join(task->future, 0);
  if (event->event == TEST_EVENT_CANCEL) {
    assert(event->waitable == 0);
    assert(event->code == 0);

    test_waitable_status_t status = exports_test_future_void_cancel_read(task->future);
    assert(TEST_WAITABLE_STATE(status) == TEST_WAITABLE_CANCELLED);
    assert(TEST_WAITABLE_COUNT(status) == 0);
    test_task_cancel();
  } else {
    assert(event->event == TEST_EVENT_FUTURE_READ);
    assert(event->waitable == task->future);
    assert(TEST_WAITABLE_STATE(event->code) == TEST_WAITABLE_COMPLETED);
    assert(TEST_WAITABLE_COUNT(event->code) == 0);
    exports_test_many_arguments_pending_return();
  }

  exports_test_future_void_drop_readable(task->future);
  test_waitable_set_drop(task->set);

  free(task);

  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_many_arguments_cases(
    uint32_t *maybe_a, exports_test_result_u32_string_t *b, test_string_t *c,
    test_string_t *d, test_string_t *e, test_string_t *f, test_string_t *g,
    test_string_t *h) {
  bool some_and_err = maybe_a != NULL && *maybe_a == 42 && b->is_err &&
                      b->val.err.len == 5 &&
                      memcmp(b->val.err.ptr, "error", 5) == 0;
  bool none_and_ok = maybe_a == NULL && !b->is_err && b->val.ok == 7;
  assert(some_and_err || none_and_ok);
  // Frees the error string when there is one.
  exports_test_result_u32_string_free(b);
  expect_letter(c, 'c');
  expect_letter(d, 'd');
  expect_letter(e, 'e');
  expect_letter(f, 'f');
  expect_letter(g, 'g');
  expect_letter(h, 'h');
  exports_test_many_arguments_cases_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_many_arguments_cases_callback(test_event_t *event) {
  assert(0);
}

test_callback_code_t exports_test_one_big_record(exports_test_big_record_t *r) {
  assert(r->f0 == 0);
  assert(r->f8 == 8);
  assert(r->f16 == 16);
  exports_test_one_big_record_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_one_big_record_callback(test_event_t *event) {
  assert(0);
}

test_callback_code_t exports_test_direct_values(
    test_string_t *a, test_string_t *b, test_string_t *c, test_string_t *d,
    test_string_t *e, test_string_t *f, test_string_t *g, uint32_t h) {
  expect_letter(a, 'a');
  expect_letter(b, 'b');
  expect_letter(c, 'c');
  expect_letter(d, 'd');
  expect_letter(e, 'e');
  expect_letter(f, 'f');
  expect_letter(g, 'g');
  assert(h == 8);
  exports_test_direct_values_return();
  return TEST_CALLBACK_CODE_EXIT;
}

test_callback_code_t exports_test_direct_values_callback(test_event_t *event) {
  assert(0);
}

// Memory size in pages. Without the leak it stops growing once the warm-up
// calls are done.
uint32_t exports_test_heap_mark(void) {
  return __builtin_wasm_memory_size(0);
}

bool exports_test_supports_cancel(void) {
  return true;
}
