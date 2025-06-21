#include "exporter.h"
#include <stdlib.h>

struct exports_test_resource_borrow_imported_test_thing_t {
  uint32_t my_state;
};

uint32_t exports_test_resource_borrow_imported_test_method_thing_get_int(
    exports_test_resource_borrow_imported_test_borrow_thing_t thing) {
  return thing->my_state;
}

void exports_test_resource_borrow_imported_test_thing_destructor(
    exports_test_resource_borrow_imported_test_thing_t *rep) {
  free(rep);
}

exports_test_resource_borrow_imported_test_own_thing_t exports_test_resource_borrow_imported_test_constructor_thing(void) {
    exports_test_resource_borrow_imported_test_thing_t *rep = malloc(sizeof(exports_test_resource_borrow_imported_test_thing_t));
    rep->my_state = 42;
    return exports_test_resource_borrow_imported_test_thing_new(rep);
}