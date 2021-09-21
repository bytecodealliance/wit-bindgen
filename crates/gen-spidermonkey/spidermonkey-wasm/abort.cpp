#include <stdlib.h>

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Winvalid-offsetof"
#include "js/Exception.h"
#pragma clang diagnostic pop

#include "smw/abort.h"
#include "smw/cx.h"
#include "smw/dump.h"

namespace smw {

void abort(const char* msg) {
    abort(get_js_context(), msg);
}

void abort(JSContext *cx, const char* msg) {
    fprintf(stderr, "Error: %s", msg);

    if (JS_IsExceptionPending(cx))  {
        fprintf(stderr, ":");
        JS::ExceptionStack exception(cx);
        if (!JS::GetPendingExceptionStack(cx, &exception)) {
            fprintf(stderr, " failed to get pending exception value and stack\n");
        } else {
            fprintf(stderr, "\n  exception value: ");
            if (!dump_value(cx, exception.exception(), stderr)) {
                fprintf(stderr, "<failed to dump value>");
            }
            fprintf(stderr, "\n  exception stack:\n");
            if (!dump_stack(cx, exception.stack(), stderr)) {
                fprintf(stderr, "<failed to dump stack>\n");
            }
        }
    } else {
        fprintf(stderr, "\n");
    }

    // TODO: check for unhandled promise rejections.

    fflush(stderr);
    ::abort();
}

} // namespace smw
