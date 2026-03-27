//@ args = '--autodrop-borrows=no'

#include "borrower.h"
#include <assert.h>

void exports_test_resource_borrow_imported_borrow_thing_do_borrow(
    test_resource_borrow_imported_test_borrow_thing_t thing) {
    uint32_t result = test_resource_borrow_imported_test_method_thing_get_int(thing);
    assert(result == 42);
    // We must explicitly drop the borrow, because autodrop borrows is turned off
    test_resource_borrow_imported_test_thing_drop_borrow(thing);
}