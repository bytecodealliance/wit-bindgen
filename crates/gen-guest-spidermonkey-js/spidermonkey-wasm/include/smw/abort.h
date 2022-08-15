#ifndef _smw_abort_h
#define _smw_abort_h

struct JSContext;

namespace smw {

/**
 * Print the given error message and abort.
 */
void abort(const char* msg);
void abort(JSContext *cx, const char* msg);

}

#endif // _smw_abort_h
