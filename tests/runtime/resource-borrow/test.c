//@ args = '--rename test:resource-borrow/to-test=test'

#include <assert.h>
#include <stdlib.h>
#include "test.h"

struct exports_test_thing_t {
  uint32_t my_state;
};

exports_test_own_thing_t exports_test_constructor_thing(uint32_t v) {
  exports_test_thing_t *rep = malloc(sizeof(exports_test_thing_t));
  assert(rep != NULL);
  rep->my_state = v + 1;
  return exports_test_thing_new(rep);
}

uint32_t exports_test_foo(exports_test_borrow_thing_t v) {
  return v->my_state + 2;
}

void exports_test_thing_destructor(exports_test_thing_t *rep) {
  free(rep);
}
