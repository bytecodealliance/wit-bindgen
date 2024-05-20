
#include <stdint.h>
#include <stdio.h>
#include "w2c2_guest.h"

static guestreleaseInstance* instance;
static guestreleaseInstance app_instance;

void trap(Trap trap) {
  abort();
}

guestreleaseInstance* get_app() {
  if (!instance) {
    guestreleaseInstantiate(&app_instance, NULL);
    instance = &app_instance;
  }
  return instance;
}

__attribute__ ((visibility ("default"))) 
void *cabi_realloc(void *ptr, size_t old_size, size_t align,
                              size_t new_size) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  uint32_t result = guestrelease_cabi_realloc(get_app(), ptr ? (uint8_t*)ptr-linmem : 0, old_size, align, new_size);
  return result+linmem;
}

// Import IF strings
// Func a GuestImport
extern void fooX3AfooX2FstringsX00a(uint8_t *arg0, size_t arg1);
void fooX3AfooX2Fstrings__a(void*app,U32 arg0,U32 arg1) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  fooX3AfooX2FstringsX00a(linmem+arg0, arg1);
}

// Func b GuestImport
extern void fooX3AfooX2FstringsX00b(uint8_t *arg0);
void fooX3AfooX2Fstrings__b(void*app,U32 arg0) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  fooX3AfooX2FstringsX00b(linmem+arg0);
}
// Func c GuestImport
extern void fooX3AfooX2FstringsX00c(uint8_t *arg0, size_t arg1,
                                        uint8_t *arg2, size_t arg3,
                                        uint8_t *arg4);
void fooX3AfooX2Fstrings__c(void*app,U32 arg0,U32 arg1,U32 arg2,U32 arg3,U32 arg4) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  fooX3AfooX2FstringsX00c(linmem+arg0, arg1, linmem+arg2, arg3, linmem+arg4);
}
// Export IF strings
// Func a GuestExport
__attribute__ ((visibility ("default"))) 
void fooX3AfooX2FstringsX23a(uint8_t *arg0, size_t arg1) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  guestrelease_fooX3AfooX2FstringsX23a(get_app(), arg0-linmem, arg1);
}
// Func b GuestExport
__attribute__ ((visibility ("default"))) uint8_t *
fooX3AfooX2FstringsX23b() {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  uint32_t result = guestrelease_fooX3AfooX2FstringsX23b(get_app());
  return result+linmem;
}
__attribute__ ((visibility ("default"))) 
void cabi_post_fooX3AfooX2FstringsX23b(uint8_t * arg0) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  guestrelease_cabi_post_fooX583AfooX582FstringsX5823b(get_app(), arg0-linmem);
}
// Func c GuestExport
__attribute__ ((visibility ("default"))) 
uint8_t * fooX3AfooX2FstringsX23c(uint8_t * arg0, size_t arg1, uint8_t *arg2, size_t arg3) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  uint32_t result = guestrelease_fooX3AfooX2FstringsX23c(get_app(), arg0-linmem, arg1, arg2-linmem, arg3);
  return result+linmem;
}
__attribute__ ((visibility ("default"))) 
extern void
cabi_post_fooX3AfooX2FstringsX23c(uint8_t * arg0) {
  uint8_t *linmem = guestrelease_memory(get_app())->data;
  guestrelease_cabi_post_fooX583AfooX582FstringsX5823c(get_app(), arg0-linmem);
}
