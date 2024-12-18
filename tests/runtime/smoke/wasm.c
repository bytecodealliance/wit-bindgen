#include <smoke.h>
#include <stdio.h>

void exports_smoke_thunk() {
  test_smoke_imports_thunk();

  printf("howdy\n");
}
