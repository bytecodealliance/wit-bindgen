#include <smoke.h>
#include <stdio.h>

void exports_thunk() {
  imports_thunk();

  printf("howdy\n");
}
