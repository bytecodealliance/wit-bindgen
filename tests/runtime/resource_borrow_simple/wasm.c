#include <assert.h>
#include <resource_borrow_simple.h>

bool resource_borrow_simple_test_imports(void) {
    resource_borrow_simple_borrow_r_t r = resource_borrow_simple_borrow_r_t {
        .__handle = 0,
    }
    resource_borrow_simple_test(r);
    resource_borrow_simple_r_drop_borrow(r);
}
