#include <assert.h>
#include "runner.h"

int main() {
  test_resource_borrow_to_test_own_thing_t thing;
  thing = test_resource_borrow_to_test_constructor_thing(42);

  test_resource_borrow_to_test_borrow_thing_t borrow;
  borrow = test_resource_borrow_to_test_borrow_thing(thing);

  uint32_t res = test_resource_borrow_to_test_foo(borrow);
  assert(res == 42 + 1 + 2);

  test_resource_borrow_to_test_thing_drop_own(thing);
}
