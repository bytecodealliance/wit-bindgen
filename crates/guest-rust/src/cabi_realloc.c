#include <stdint.h>

extern void *cabi_realloc_wit_bindgen_0_18_0(void *ptr, size_t old_size, size_t align, size_t new_size);

__attribute__((__weak__, __export_name__("cabi_realloc")))
void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {
  return cabi_realloc_wit_bindgen_0_18_0(ptr, old_size, align, new_size);
}
