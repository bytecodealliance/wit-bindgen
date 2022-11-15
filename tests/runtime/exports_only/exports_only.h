#ifndef __BINDINGS_EXPORTS_ONLY_H
#define __BINDINGS_EXPORTS_ONLY_H
#ifdef __cplusplus
extern "C" {
#endif

#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

typedef struct {
  char*ptr;
  size_t len;
} exports_only_string_t;

// Exported Functions from `exports-only`
void exports_only_thunk(exports_only_string_t *ret);

// Helper Functions

void exports_only_string_set(exports_only_string_t *ret, const char*s);
void exports_only_string_dup(exports_only_string_t *ret, const char*s);
void exports_only_string_free(exports_only_string_t *ret);

#ifdef __cplusplus
}
#endif
#endif
