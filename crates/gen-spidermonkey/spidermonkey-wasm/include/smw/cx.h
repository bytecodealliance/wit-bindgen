#ifndef _smw_cx_h
#define _smw_cx_h

struct JSContext;
class JSObject;

namespace smw {

void init_js_context(JSContext* cx);
JSContext* get_js_context();

void init_user_module(JSContext* cx, JSObject* user_module);
JSObject* get_user_module();

}

#endif // _smw_cx_h
