/* remotely defined type example */

#include <stdint.h>
#include <stddef.h>

typedef struct my_inline_foo_string_t {
  uint8_t*ptr;
  size_t len;
} my_inline_foo_string_t;

typedef struct my_inline_foo_msg_t {
  my_inline_foo_string_t   field;
} my_inline_foo_msg_t;

void my_inline_foo_msg_free(my_inline_foo_msg_t *ptr);
void my_inline_foo_string_set(my_inline_foo_string_t *ptr, char const* value);
