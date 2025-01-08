#include <assert.h>
#include <resource_borrow_simple.h>

void exports_resource_borrow_simple_test_imports(void) {
    resource_borrow_simple_own_r_t r = resource_borrow_simple_constructor_r();
    resource_borrow_simple_borrow_r_t b = resource_borrow_simple_borrow_r(r);
    resource_borrow_simple_test(b);
    resource_borrow_simple_r_drop_own(r);
}
