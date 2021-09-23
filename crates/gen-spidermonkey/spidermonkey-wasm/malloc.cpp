#include <stdint.h>
#include <stdlib.h>

#include "smw/abort.h"
#include "smw/wasm.h"

namespace smw {

WASM_EXPORT
void* SMW_malloc(size_t size) {
    auto p = malloc(size);
    if (p == nullptr) {
        abort("out of memory");
    }
    return p;
}

} // namespace smw
