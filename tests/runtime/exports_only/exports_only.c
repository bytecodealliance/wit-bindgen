#include "exports_only.h"

__attribute__((weak, export_name("cabi_post_thunk")))
void __wasm_export_exports_only_thunk_post_return(int32_t arg0) {
  if ((*((int32_t*) (arg0 + 4))) > 0) {
    free((void*) (*((int32_t*) (arg0 + 0))));
  }
}

__attribute__((weak, export_name("cabi_realloc")))
void *cabi_realloc(void *ptr, size_t orig_size, size_t org_align, size_t new_size) {
  void *ret = realloc(ptr, new_size);
  if (!ret) abort();
  return ret;
}

// Helper Functions

void exports_only_string_set(exports_only_string_t *ret, const char*s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void exports_only_string_dup(exports_only_string_t *ret, const char*s) {
  ret->len = strlen(s);
  ret->ptr = cabi_realloc(NULL, 0, 1, ret->len * 1);
  memcpy(ret->ptr, s, ret->len * 1);
}

void exports_only_string_free(exports_only_string_t *ret) {
  if (ret->len > 0) {
    free(ret->ptr);
  }
  ret->ptr = NULL;
  ret->len = 0;
}

// Component Adapters

__attribute__((aligned(4)))
static uint8_t RET_AREA[8];

__attribute__((export_name("thunk")))
int32_t __wasm_export_exports_only_thunk(void) {
  exports_only_string_t ret;
  exports_only_thunk(&ret);
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 4)) = (int32_t) (ret).len;
  *((int32_t*)(ptr + 0)) = (int32_t) (ret).ptr;
  return ptr;
}

extern void __component_type_object_force_link_exports_only(void);
void __component_type_object_force_link_exports_only_public_use_in_this_compilation_unit(void) {
  __component_type_object_force_link_exports_only();
}
