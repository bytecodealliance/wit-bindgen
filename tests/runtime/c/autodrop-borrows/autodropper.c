//@ args = '--autodrop-borrows=yes'

#include "autodropper.h"
#include <assert.h>

void exports_test_resource_borrow_imported_autodrop_borrow_thing_do_borrow(
    test_resource_borrow_imported_test_borrow_thing_t thing) {
    uint32_t result = test_resource_borrow_imported_test_method_thing_get_int(thing);
    assert(result == 42);
    // Intentionally do not drop the borrow, as it will be done automatically
}