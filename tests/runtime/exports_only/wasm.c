#include <exports_only.h>
#include <stdio.h>

__attribute__((weak, export_name("cabi_realloc")))
void *cabi_realloc(void *ptr, size_t orig_size, size_t org_align, size_t new_size) {
  return ptr;
}

__attribute__((weak, export_name("cabi_post_thunk")))
void __wasm_export_exports_only_thunk_post_return(int32_t arg0) {
}

static char msg[] = "test";

void exports_only_thunk(exports_only_string_t* ret) {
  exports_only_string_t result = {
    .ptr = &msg[0],
    .len = sizeof(msg) - 1,
  };
  *ret = result;
}
