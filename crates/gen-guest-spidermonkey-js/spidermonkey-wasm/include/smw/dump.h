#ifndef _smw_dump_h
#define _smw_dump_h

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Winvalid-offsetof"

#include "js/TypeDecls.h"
#include "js/Value.h"

#pragma clang diagnostic pop

namespace smw {

/**
 * Dump a human-readable representation of the given JS value to the given file.
 */
bool dump_value(JSContext *cx, JS::HandleValue val, FILE* fp);

/**
 * Dump a human-readable representation of the given JS exception stack to the
 * given file.
 */
bool dump_stack(JSContext *cx, JS::HandleObject stack, FILE* fp);

}

#endif // _smw_dump_h
