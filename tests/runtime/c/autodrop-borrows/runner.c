#include <assert.h>
#include "runner.h"

int main() {
    test_resource_borrow_imported_test_own_thing_t thing = test_resource_borrow_imported_test_constructor_thing();
    assert(thing.__handle != 0);

    test_resource_borrow_imported_borrow_thing_do_borrow(
        test_resource_borrow_imported_test_borrow_thing(thing)
    );

    test_resource_borrow_imported_autodrop_borrow_thing_do_borrow(
        test_resource_borrow_imported_test_borrow_thing(thing)
    );

    test_resource_borrow_imported_test_thing_drop_own(thing);
}