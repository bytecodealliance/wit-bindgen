#include <string.h>
#include <assert.h>
#include "test.h"

void exports_my_inline_bar_bar(exports_my_inline_bar_msg_t *m) {
    assert(m->field.len==5);
    assert(memcmp(m->field.ptr, "hello", 5)==0);
    exports_my_inline_bar_msg_free(m);
}
