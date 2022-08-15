#include "smw/dump.h"

#include <assert.h>
#include "jsapi.h"
#include "smw/wasm.h"

namespace smw {

static JS::UniqueChars stringify_value(JSContext *cx, JS::HandleValue val) {
    JS::RootedString str(cx, JS_ValueToSource(cx, val));
    if (!str) {
        return nullptr;
    }
    return JS_EncodeStringToUTF8(cx, str);
}

bool dump_value(JSContext *cx, JS::HandleValue val, FILE* fp) {
    JS::UniqueChars str = stringify_value(cx, val);
    if (!str) {
        return false;
    }
    fprintf(fp, "%s\n", str.get());
    return true;
}

bool dump_stack(JSContext *cx, JS::HandleObject stack, FILE* fp) {
    JS::RootedString str(cx);
    size_t indent = 4;
    if (!JS::BuildStackString(cx, nullptr, stack, &str, indent)) {
        return false;
    }

    JS::UniqueChars utf8 = JS_EncodeStringToUTF8(cx, str);
    if (!utf8) {
        return false;
    }

    fprintf(fp, "%s\n", utf8.get());
    return true;
}

WASM_EXPORT
int32_t dump_i32(int32_t x) {
    fprintf(stderr, "dump_i32: %d\n", x);
    return x;
}

} // namespace smw
