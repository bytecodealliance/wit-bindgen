/*!
 * This module implements the intrinsics used by code emitted in the
 * `wit_bindgen_gen_spidermonkey::Bindgen` trait implementation.
 */

#include <assert.h>
#include <cmath>
#include <stdlib.h>

#include "smw/abort.h"
#include "smw/cx.h"
#include "smw/logging.h"
#include "smw/wasm.h"

#include "mozilla/UniquePtr.h"
#include "jsapi.h"
#include "js/Array.h"
#include "js/Conversions.h"
#include "js/ForOfIterator.h"
#include "js/Modules.h"

#ifdef LOGGING
#include "js/friend/DumpFunctions.h"
#endif

namespace smw {

using UniqueChars = mozilla::UniquePtr<char[]>;

using PersistentRootedValueVector = JS::PersistentRooted<JS::GCVector<JS::Value>>;

// Used for general Wasm<-->JS conversions.
static PersistentRootedValueVector* OPERANDS;

// Used for holding arguments to JS calls.
static PersistentRootedValueVector* ARGS;

// Used for holding returns from Wasm calls.
static PersistentRootedValueVector* RETS;

void init_operands(JSContext* cx) {
    assert(!OPERANDS && "OPERANDS must only be initialized once");
    OPERANDS = new PersistentRootedValueVector(cx, cx);
    if (!OPERANDS) {
        abort(cx, "failed to allocate OPERANDS");
    }

    assert(!ARGS && "ARGS must only be initialized once");
    ARGS = new PersistentRootedValueVector(cx, cx);
    if (!ARGS) {
        abort(cx, "failed to allocate ARGS");
    }

    assert(!RETS && "RETS must only be initialized once");
    RETS = new PersistentRootedValueVector(cx, cx);
    if (!RETS) {
        abort(cx, "failed to allocate RETS");
    }
}

PersistentRootedValueVector& operands() {
    assert(OPERANDS && OPERANDS->initialized() && "OPERANDS must be initialized");
    return *OPERANDS;
}

void save_operand(size_t dest, JS::HandleValue val) {
#if LOGGING==1
    SMW_LOG("operands[%zu] = ", dest);
    js::DumpValue(val, stderr);
#endif // LOGGING==1

    JSContext* cx = get_js_context();

    if (operands().length() <= dest) {
        size_t needed_capacity = 1 + dest - operands().length();
        if (!operands().reserve(needed_capacity)) {
            abort("failed to reserve capacity for the OPERANDS vector");
        }
        if (dest == operands().length()) {
            bool ok = operands().append(val);
            assert(ok && "already reserved space");
            return;
        }
        JS::RootedValue placeholder(cx, JS::UndefinedValue());
        for (size_t i = 0; i < needed_capacity; i++) {
            bool ok = operands().append(placeholder);
            assert(ok && "already reserved space");
        }
    }

    operands()[dest].set(val);
}

PersistentRootedValueVector& args() {
    assert(ARGS && ARGS->initialized() && "ARGS must be initialized");
    return *ARGS;
}

PersistentRootedValueVector& rets() {
    assert(RETS && RETS->initialized() && "RETS must be initialized");
    return *RETS;
}

WASM_EXPORT
void canonical_abi_free(void* ptr, size_t size, size_t align) {
    (void) size;
    (void) align;
    free(ptr);
}

WASM_EXPORT
void* canonical_abi_realloc(void* ptr, size_t old_size, size_t align, size_t new_size) {
    (void) old_size;
    (void) align;
    return realloc(ptr, new_size);
}

WASM_EXPORT
void SMW_fill_operands(unsigned argc, JS::Value* vp) {
    SMW_LOG("SMW_fill_operands(argc = %d, vp = %p)\n", argc, vp);

    JS::CallArgs args = JS::CallArgsFromVp(argc, vp);

    if (!operands().reserve(size_t(args.length()))) {
        abort(get_js_context(), "failed to reserve space in the operands vector");
    }
    for (unsigned i = 0; i < args.length(); i++) {
#if LOGGING==1
        SMW_LOG("operands[%d] = ", i);
        js::DumpValue(args.get(i), stderr);
#endif // LOGGING==1

        bool ok = operands().append(args.get(i));
        assert(ok && "already reserved space");
    }
}

WASM_EXPORT
void SMW_clear_operands() {
    SMW_LOG("SMW_clear_operands\n");
    operands().clear();
}

WASM_EXPORT
void SMW_push_arg(size_t i) {
    SMW_LOG("SMW_push_arg(i = %zu)\n", i);
    if (!args().append(operands()[i])) {
        abort("failed to push arg");
    }
}

WASM_EXPORT
void SMW_call(char *funcName, size_t funcNameLen, size_t numResults, size_t dest) {
#ifdef LOGGING
    SMW_LOG("SMW_call(funcName = %p \"", funcName);
    for (size_t i = 0; i < funcNameLen; i++) {
        SMW_LOG("%c", funcName[i]);
    }
    SMW_LOG("\", funcNameLen = %zu, numResults = %zu, dest = %zu)\n",
            funcNameLen,
            numResults,
            dest);
#endif

    UniqueChars uniqFuncName(funcName);

    JSContext *cx = get_js_context();

    JS::RootedString funcNameAtom(cx, JS_AtomizeStringN(cx, uniqFuncName.get(), funcNameLen));
    if (!funcNameAtom) {
        abort(cx, "failed to atomize function name");
    }

    JS::RootedObject module(cx, get_user_module());
    JS::RootedValue exportVal(cx);
    bool hasExport = false;
    if (!JS::GetModuleExport(cx, module, funcNameAtom, &exportVal, &hasExport)) {
        abort(cx, "failed to get module export");
    }
    if (!hasExport) {
        // TODO: include the export name in this message to help users debug
        // which export they're missing.
        abort(cx, "user module does not have the requested export");
    }

    JS::RootedFunction exportFunc(cx, JS_ValueToFunction(cx, exportVal));
    if (!exportFunc) {
        // TODO: include the export name in this message.
        abort(cx, "exported value is not a function");
    }

    // XXX: we have to copy ARGS into a `JS::RootedVector<JS::Value>` because
    // `JS::Call` takes a `JS::HandleValueArray` and you can't construct that
    // from a `JS::PersistentRooted<JS::GCVector<JS::Value>>`, only a
    // `JS::RootedVector<JS::Value>`. And we can't make `ARGS` a
    // `JS::RootedVector<JS::Value>` because it is a global, not an on-stack
    // RAII value as required by `JS::RootedVector<JS::Value>`. Gross!
    JS::RootedVector<JS::Value> argsVector(cx);
    if (!argsVector.reserve(args().length())) {
        abort(cx, "failed to reserve space for arguments vector");
    }
    for (size_t i = 0; i < args().length(); i++) {
        bool ok = argsVector.append(args()[i]);
        assert(ok && "already reserved space");
    }

    JS::RootedObject thisObj(cx);
    JS::RootedValue result(cx);
    if (!JS::Call(cx, thisObj, exportFunc, argsVector, &result)) {
        // TODO: include the export name in this message.
        abort(cx, "calling export function failed");
    }

    args().clear();

    if (numResults == 0) {
        // Nothing to push onto the operands vector.
    } else if (numResults == 1) {
        save_operand(dest, result);
    } else {
        // Treat the "physical" return value as an iterator and unpack the
        // "logical" return values from within it. This allows JS to return
        // multiple WIT values as an array or any other iterable.
        JS::ForOfIterator iter(cx);
        if (!iter.init(result)) {
            // TODO: include the export name in this message.
            abort(cx, "failed to convert return value to iterable");
        }
        JS::RootedValue val(cx);
        bool done = false;
        for (size_t i = 0; i < numResults; i++) {
            if (done) {
                // TODO: include the export name in this message.
                abort(cx, "function's returned iterator did not yield enough return values");
            }
            if (!iter.next(&val, &done)) {
                // TODO: include the export name in this message.
                abort(cx, "failed to get the next value out of the return values iterator");
            }
            save_operand(dest + i, val);
        }
    }
}

WASM_EXPORT
void SMW_push_return_value(size_t i) {
    SMW_LOG("SMW_push_return_value(i = %zu)\n", i);
    if (!rets().append(operands()[i])) {
        abort(get_js_context(), "failed to push arg");
    }
}

WASM_EXPORT
void SMW_finish_returns(unsigned argc, JS::Value* vp) {
    SMW_LOG("SMW_finish_returns(argc = %d, vp = %p)\n", argc, vp);

    JS::CallArgs args = JS::CallArgsFromVp(argc, vp);
    switch (rets().length()) {
    case 0: {
        break;
    }
    case 1: {
        args.rval().set(rets().back());
        break;
    }
    default: {
        JSContext* cx = get_js_context();
        JS::RootedVector<JS::Value> elems(cx);
        if (!elems.reserve(rets().length())) {
            abort(cx, "failed to reserve space for results vector");
        }
        for (size_t i = 0; i < rets().length(); i++) {
            bool ok = elems.append(rets()[i]);
            assert(ok && "already reserved space");
        }
        JS::RootedObject arr(cx, JS::NewArrayObject(cx, elems));
        if (!arr) {
            abort(cx, "failed to allocate array for function's return values");
        }
        args.rval().setObject(*arr.get());
        break;
    }
    }

    rets().clear();
}

WASM_EXPORT
uint32_t SMW_i32_from_u32(size_t i) {
    SMW_LOG("SMW_i32_from_u32(i = %zu)\n", i);

    JSContext* cx = get_js_context();
    JS::RootedValue val(cx, operands()[i]);
    double number = 0.0;
    if (!JS::ToNumber(cx, val, &number)) {
        abort(cx, "failed to convert value to number");
    }
    number = std::round(number);
    return uint32_t(number);
}

WASM_EXPORT
void SMW_u32_from_i32(uint32_t x, size_t dest) {
    SMW_LOG("SMW_u32_from_i32(x = %ull, dest = %zu)\n", x, dest);

    JSContext* cx = get_js_context();
    JS::RootedValue val(cx, JS::NumberValue(x));
    save_operand(dest, val);
}

WASM_EXPORT
void SMW_string_canon_lower(uint32_t* ret_ptr, size_t i) {
    SMW_LOG("SMW_string_canon_lower(ret_ptr = %p, i = %zu)\n", ret_ptr, i);

    JSContext* cx = get_js_context();
    JS::RootedValue strVal(cx, operands()[i]);
    if (!strVal.isString()) {
        abort(cx, "value is not a string");
    }
    JS::RootedString str(cx, strVal.toString());
    JS::Rooted<JSLinearString*> linearStr(cx, JS_EnsureLinearString(cx, str));
    if (!linearStr) {
        abort(cx, "failed to linearize JS string");
    }

    size_t len = JS::GetDeflatedUTF8StringLength(linearStr);
    char* ptr = static_cast<char*>(malloc(len));
    if (!ptr) {
        abort(cx, "out of memory");
    }

    size_t num_written = JS::DeflateStringToUTF8Buffer(linearStr, mozilla::Span(ptr, len));
    assert(num_written == len);

    ret_ptr[0] = reinterpret_cast<uint32_t>(ptr);
    ret_ptr[1] = static_cast<uint32_t>(len);
}

WASM_EXPORT
void SMW_string_canon_lift(char* ptr, size_t len, size_t dest) {
    SMW_LOG("SMW_string_canon_lift(ptr = %p, len = %zu, dest = %zu)\n", ptr, len, dest);

    JSContext* cx = get_js_context();
    JS::RootedString str(cx, JS_NewStringCopyUTF8N(cx, JS::UTF8Chars(ptr, len)));
    if (!str) {
        abort(cx, "failed to create JS string from UTF-8 buffer");
    }
    JS::RootedValue strVal(cx, JS::StringValue(str));
    save_operand(dest, strVal);
}

WASM_EXPORT
uint32_t SMW_spread_into_array(size_t i) {
    SMW_LOG("SMW_spread_into_array; i = %zu\n", i);

    JSContext* cx = get_js_context();

    JS::RootedValue iterable(cx, operands()[i]);
    bool is_array = false;
    if (!JS::IsArrayObject(cx, iterable, &is_array)) {
        abort(cx, "failed to check if object is an array");
    }

    if (is_array) {
        JS::RootedObject arr(cx, &iterable.toObject());
        uint32_t length = 0;
        if (!JS::GetArrayLength(cx, arr, &length)) {
            abort(cx, "failed to get array length");
        }
        return length;
    }

    JS::RootedVector<JS::Value> elems(cx);
    JS::ForOfIterator iter(cx);
    if (!iter.init(iterable)) {
        abort(cx, "failed to convert operand value to iterable");
    }
    JS::RootedValue val(cx);
    bool done = false;
    while (!done) {
        if (!iter.next(&val, &done)) {
            abort(cx, "failed to get the next value out of iterator");
        }
        if (done) {
            break;
        }
        if (!elems.append(val)) {
            abort(cx, "failed to append value to vector");
        }
    }

    JS::RootedObject arr(cx, JS::NewArrayObject(cx, elems));
    if (!arr) {
        abort(cx, "failed to allocate JS array object");
    }
    operands()[i].setObject(*arr);

    return elems.length();
}

WASM_EXPORT
void SMW_get_array_element(size_t array, size_t index, size_t dest) {
    SMW_LOG("SMW_get_array_element(array = %zu, index = %zu, dest = %zu)\n", array, index, dest);

    JSContext* cx = get_js_context();

    JS::RootedValue array_val(cx, operands()[array]);
    assert(array_val.isObject());
    JS::RootedObject array_obj(cx, &array_val.toObject());
    JS::RootedValue elem(cx);
    if (!JS_GetElement(cx, array_obj, index, &elem)) {
        abort(cx, "failed to get array element");
    }

    save_operand(dest, elem);
}

WASM_EXPORT
void SMW_new_array(size_t dest) {
    SMW_LOG("SMW_new_array(dest = %zu)\n", dest);

    JSContext* cx = get_js_context();
    JS::RootedObject arr(cx, JS::NewArrayObject(cx, 0));
    if (!arr) {
        abort(cx, "failed to allocate a new JS array object");
    }
    JS::RootedValue arr_val(cx, JS::ObjectValue(*arr));
    save_operand(dest, arr_val);
}

WASM_EXPORT
void SMW_array_push(size_t array, size_t elem) {
    SMW_LOG("SMW_array_push(array = %zu, elem = %zu)\n", array, elem);

    JSContext* cx = get_js_context();

    JS::RootedValue array_val(cx, operands()[array]);
    assert(array_val.isObject());
    JS::RootedObject array_obj(cx, &array_val.toObject());

    uint32_t length = 0;
    if (!JS::GetArrayLength(cx, array_obj, &length)) {
        abort(cx, "failed to get JS array object length");
    }

    JS::RootedValue elem_val(cx, operands()[elem]);
    if (!JS_SetElement(cx, array_obj, length, elem_val)) {
        abort(cx, "failed to set JS array element");
    }
}

} // namespace smw
