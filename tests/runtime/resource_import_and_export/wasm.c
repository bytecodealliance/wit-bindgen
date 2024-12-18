#include <assert.h>
#include <resource_import_and_export.h>
/* #include <float.h> */
/* #include <limits.h> */
/* #include <math.h> */
/* #include <stdalign.h> */
#include <stdlib.h>
#include <string.h>

struct exports_test_resource_import_and_export_test_thing_t {
  test_resource_import_and_export_test_own_thing_t thing;
};

resource_import_and_export_own_thing_t
exports_resource_import_and_export_toplevel_export(resource_import_and_export_own_thing_t a) {
  return resource_import_and_export_toplevel_import(a);
}

exports_test_resource_import_and_export_test_own_thing_t
exports_test_resource_import_and_export_test_constructor_thing(uint32_t v) {
  exports_test_resource_import_and_export_test_thing_t *val =
    (exports_test_resource_import_and_export_test_thing_t *)
    malloc(sizeof(exports_test_resource_import_and_export_test_thing_t));
  assert(val != NULL);
  val->thing = test_resource_import_and_export_test_constructor_thing(v + 1);
  return exports_test_resource_import_and_export_test_thing_new(val);
}

uint32_t
exports_test_resource_import_and_export_test_method_thing_foo(exports_test_resource_import_and_export_test_borrow_thing_t self) {
  test_resource_import_and_export_test_borrow_thing_t borrow =
    test_resource_import_and_export_test_borrow_thing(self->thing);
  return test_resource_import_and_export_test_method_thing_foo(borrow) + 2;
}

void
exports_test_resource_import_and_export_test_method_thing_bar(exports_test_resource_import_and_export_test_borrow_thing_t self, uint32_t v) {
  test_resource_import_and_export_test_borrow_thing_t borrow =
    test_resource_import_and_export_test_borrow_thing(self->thing);
  test_resource_import_and_export_test_method_thing_bar(borrow, v + 3);
}

exports_test_resource_import_and_export_test_own_thing_t
exports_test_resource_import_and_export_test_static_thing_baz(exports_test_resource_import_and_export_test_own_thing_t a, exports_test_resource_import_and_export_test_own_thing_t b) {
  exports_test_resource_import_and_export_test_thing_t *a_rep =
    exports_test_resource_import_and_export_test_thing_rep(a);
  exports_test_resource_import_and_export_test_thing_t *b_rep =
    exports_test_resource_import_and_export_test_thing_rep(b);

  test_resource_import_and_export_test_own_thing_t tmp =
    test_resource_import_and_export_test_static_thing_baz(a_rep->thing, b_rep->thing);
  test_resource_import_and_export_test_borrow_thing_t tmp_borrow =
    test_resource_import_and_export_test_borrow_thing(tmp);
  uint32_t ret = test_resource_import_and_export_test_method_thing_foo(tmp_borrow) + 4;
  test_resource_import_and_export_test_thing_drop_own(tmp);

  return exports_test_resource_import_and_export_test_constructor_thing(ret);
}

void
exports_test_resource_import_and_export_test_thing_destructor(exports_test_resource_import_and_export_test_thing_t *rep) {
  free(rep);
}
