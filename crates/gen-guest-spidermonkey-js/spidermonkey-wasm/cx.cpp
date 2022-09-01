#include <assert.h>

#include "smw/cx.h"

#include "jsapi.h"

namespace smw {

static JSContext* CONTEXT = nullptr;

void init_js_context(JSContext *cx) {
    assert(!CONTEXT && "CONTEXT should only be initialized once");
    CONTEXT = cx;
}

JSContext *get_js_context() {
    assert(CONTEXT && "CONTEXT should be initialized");
    return CONTEXT;
}

static JS::PersistentRooted<JSObject*> USER_MODULE;

void init_user_module(JSContext* cx, JSObject* user_module) {
    assert(!USER_MODULE && "USER_MODULE should only be initialized once");
    USER_MODULE.init(cx, user_module);
}

JSObject* get_user_module() {
    assert(USER_MODULE && "USER_MODULE should be initialized");
    return USER_MODULE;
}

} // namespace smw
