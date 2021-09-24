/*!
 * JS engine initialization and JS top-level evaluation.
 *
 * This file contains the code to start up the JS engine, define import-able
 * modules from VM functions, and evaluate the user JS.
 */

#include <assert.h>
#include <stdlib.h>

#include "smw/abort.h"
#include "smw/bindgen.h"
#include "smw/cx.h"
#include "smw/wasm.h"

#include "js/AllocPolicy.h"
#include "js/CompilationAndEvaluation.h"
#include "js/GCAPI.h"
#include "js/GCVector.h"
#include "js/Initialization.h"
#include "js/Modules.h"
#include "js/Promise.h"
#include "js/Realm.h"
#include "js/SourceText.h"
#include "js/TypeDecls.h"
#include "js/Warnings.h"
#include "jsapi.h"
#include "jsfriendapi.h"

namespace smw {

using UniqueChars = mozilla::UniquePtr<char[]>;

bool INITIALIZED = false;
JS::PersistentRootedObject GLOBAL;

static JSClass global_class = {
    "global",
    JSCLASS_GLOBAL_FLAGS,
    &JS::DefaultGlobalClassOps
};

/**
 * Compile the given JS source as a module in the context of the given global.
 *
 * Takes ownership of `jsSource`.
 *
 * Does not take ownership of `jsFileName`.
 *
 * Sets `outModule` to the resulting source text module record object.
 */
bool compile_js_module(JSContext *cx,
                       const char *jsFileName,
                       char *jsSource,
                       size_t jsSourceLen,
                       JS::MutableHandleObject outModule) {
    JS::CompileOptions copts(cx);
    copts
        .setFileAndLine(jsFileName, 1)
        .setNoScriptRval(true)
        .setForceFullParse();

    JS::SourceText<mozilla::Utf8Unit> srcBuf;
    if (!srcBuf.init(cx, jsSource, jsSourceLen, JS::SourceOwnership::TakeOwnership)) {
        return false;
    }

    JS::RootedObject module(cx);

    // Disabling generational GC during compilation seems to slightly reduce
    // the number of pages touched post-wizening. (Whereas disabling it
    // during execution meaningfully increases it, which is why this is
    // scoped to just compilation.)
    JS::AutoDisableGenerationalGC noGgc(cx);
    module = JS::CompileModule(cx, copts, srcBuf);
    if (!module) {
        return false;
    }

    outModule.set(module);
    return true;
}

/**
 * A synthesized module that exports `JSNative` functions.
 */
struct SynthesizedModule {
    JS::Heap<JSString*> moduleName;
    JS::Heap<JSObject*> moduleObject;

    SynthesizedModule(JS::HandleString moduleName, JS::HandleObject moduleObject)
        : moduleName(moduleName)
        , moduleObject(moduleObject)
    { }

    void trace(JSTracer* tracer) {
        JS::TraceEdge(tracer, &moduleObject, "SynthesizedModule.moduleObject");
    }
};

JS::PersistentRooted<JS::GCVector<SynthesizedModule, 0, js::SystemAllocPolicy>> MODULES;

JSObject* module_resolve_hook(JSContext *cx,
                              JS::HandleValue referencing_private,
                              JS::HandleObject module_request) {
    JS::RootedString specifier(cx, JS::GetModuleRequestSpecifier(cx, module_request));
    if (!specifier) {
        abort(cx, "failed to get module request specifier");
    }

    size_t len = MODULES.length();
    for (size_t i = 0; i < len; i++) {
        JS::RootedObject it_module(cx, MODULES[i].get().moduleObject);
        JS::RootedString it_name(cx, MODULES[i].get().moduleName);
        int32_t result = 0;
        if (!JS_CompareStrings(cx, it_name, specifier, &result)) {
            abort(cx, "failed to compare module specifier to registered module name");
        }
        if (result == 0) {
            return it_module.get();
        }
    }


    JS::UniqueChars utf8 = JS_EncodeStringToUTF8(cx, specifier);
    if (!utf8) {
        JS_ReportErrorASCII(cx, "failed to find module import");
        return nullptr;
    }
    JS_ReportErrorASCII(cx, "failed to find module import: `%s`", utf8.get());
    return nullptr;
}

JS::RealmOptions make_realm_options() {
    JS::RealmOptions options;
    options
        .creationOptions()
        .setStreamsEnabled(true)
        .setReadableByteStreamsEnabled(true)
        .setBYOBStreamReadersEnabled(true)
        .setReadableStreamPipeToEnabled(true)
        .setWritableStreamsEnabled(true)
        .setIteratorHelpersEnabled(true)
        .setWeakRefsEnabled(JS::WeakRefSpecifier::EnabledWithoutCleanupSome);
    return options;
}

bool init_js(JSContext *cx) {
    if (!js::UseInternalJobQueues(cx)) {
        return false;
    }

    if (!JS::InitSelfHostedCode(cx)) {
        return false;
    }

    JS::RealmOptions options = make_realm_options();

    JS::DisableIncrementalGC(cx);

    JS::RootedObject global(cx, JS_NewGlobalObject(cx, &global_class, nullptr,
                                                   JS::FireOnNewGlobalHook, options));
    if (!global) {
        return false;
    }

    JS::EnterRealm(cx, global);

    if (!JS::InitRealmStandardClasses(cx)) {
        return false;
    }

    // JS::SetPromiseRejectionTrackerCallback(cx, rejection_tracker);

    JS::SetModuleResolveHook(JS_GetRuntime(cx), module_resolve_hook);

    GLOBAL.init(cx, global);
    return true;
}

// static void report_warning(JSContext *cx, JSErrorReport *report) {
//     JS::PrintError(stderr, report, true);
//     if (!report->isWarning()) {
//         ::abort();
//     }
// }

/**
 * Initialize the JS engine and evaluate the top-level of the given JavaScript
 * source.
 *
 * Takes ownership of its parameters.
 */
WASM_EXPORT
void SMW_initialize_engine() {
    assert(!INITIALIZED);

    bool ok = true;

    ok = JS_Init();
    assert(ok && "JS_Init failed");

    JSContext *cx = JS_NewContext(JS::DefaultHeapMaxBytes);
    assert(cx != nullptr && "JS_NewContext failed");
    init_js_context(cx);

    // JS::SetWarningReporter(cx, report_warning);

    if (!init_js(cx)) {
        abort(cx, "initializing the JavaScript engine failed");
    }

    init_operands(cx);

    MODULES.init(cx);
    INITIALIZED = true;
}

class ModuleBuilder {
    JS::PersistentRootedString moduleName;
    JS::PersistentRooted<JS::IdValueVector> exports;

public:
    /**
     * Construct a new `ModuleBuilder` and take ownership of `moduleName`.
     */
    ModuleBuilder(JSContext *cx, JS::HandleString moduleName)
        : moduleName(cx, moduleName)
        , exports(cx, cx)
    {
        assert(moduleName && "moduleName must not be nullptr");
    }

    /**
     * Add an exported function to this module and take ownership of `funcName`.
     */
    void add_export(const char *funcName, size_t funcNameLen, JSNative func, unsigned numArgs) {
        assert(funcName && "function name must not be nullptr");
        assert(funcNameLen > 0 && "function name length must be greater than zero");
        assert(func && "the function must not be nullptr");

        JSContext *cx = get_js_context();

        JS::RootedString jsFuncName(cx, JS_NewStringCopyN(cx, funcName, funcNameLen));
        if (!jsFuncName) {
            abort(cx, "failed to create new JS string");
        }

        JS::RootedId funcNameId(cx);
        if (!JS_StringToId(cx, jsFuncName, &funcNameId)) {
            abort(cx, "failed to convert string to id");
        }

        JS::RootedFunction jsFunc(cx, JS_NewFunction(cx, func, numArgs, 0, funcName));
        if (!jsFunc) {
            abort(cx, "failed to create new JS function");
        }

        JS::RootedObject jsFuncObj(cx, JS_GetFunctionObject(jsFunc));
        assert(jsFuncObj && "getting function object is infallible");
        JS::RootedValue jsFuncVal(cx, JS::ObjectValue(*jsFuncObj));

        if (!exports.append(JS::IdValuePair(funcNameId, jsFuncVal))) {
            abort(cx, "failed to append export to exports list");
        }
    }

    void finish() {
        JSContext *cx = get_js_context();

        JS::RootedObject module(cx, JS::CreateModule(cx, exports));
        if (!module) {
            abort(cx, "failed to create synthetic module");
        }

        if (!MODULES.append(SynthesizedModule(moduleName, module))) {
            abort(cx, "failed to append to MODULES");
        }

        delete this;
    }
};

WASM_EXPORT
ModuleBuilder *SMW_new_module_builder(char *module_name, size_t module_name_len) {
    auto unique_module_name = UniqueChars(module_name);

    JSContext *cx = get_js_context();

    JS::RootedString js_module_name(cx, JS_NewStringCopyN(cx, unique_module_name.get(), module_name_len));
    if (!js_module_name) {
        abort(cx, "failed to allocate JS string");
    }

    auto b = new ModuleBuilder(cx, js_module_name);
    if (!b) {
        abort(cx, "failed to create new ModuleBuilder");
    }

    return b;
}

WASM_EXPORT
void SMW_module_builder_add_export(ModuleBuilder *builder,
                                   char *funcName,
                                   size_t funcNameLen,
                                   JSNative func,
                                   unsigned numArgs) {
    assert(builder && "builder must not be nullptr");
    assert(funcName && "funcName must not be nullptr");
    assert(funcNameLen > 0 && "funcNameLen must be greater than 0");
    assert(func && "func must not be nullptr");

    auto uniqFuncName = UniqueChars(funcName);
    builder->add_export(uniqFuncName.get(), funcNameLen, func, numArgs);
}

WASM_EXPORT
void SMW_finish_module_builder(ModuleBuilder *builder) {
    builder->finish();
}

WASM_EXPORT
void SMW_eval_module(char *jsFileName, char *jsSource, size_t jsSourceLen) {
    JSContext *cx = get_js_context();

    assert(GLOBAL && "GLOBAL should be initialized");
    JS::RootedObject global(cx, GLOBAL);
    JSAutoRealm autoRealm(cx, global);

    JS::RootedObject module(cx);
    if (!compile_js_module(cx, jsFileName, jsSource, jsSourceLen, &module)) {
        abort(cx, "module compilation failed");
    }

    if (!JS::ModuleInstantiate(cx, module)) {
        abort(cx, "failed to instantiate module");
    }

    JS::RootedValue result(cx);
    if (!JS::ModuleEvaluate(cx, module, &result)) {
        abort(cx, "failed to evaluate module");
    }

    // TODO: if `result` is a promise because of top-level await, then don't
    // return until the micro task queue is empty.
    if (result.isObject()) {
        JS::RootedObject resultObj(cx, &result.toObject());
        if (!JS::IsPromiseObject(resultObj)) {
            goto done_handling_promise;
        }
        switch (JS::GetPromiseState(resultObj)) {
        case JS::PromiseState::Fulfilled: {
            JS::RootedValue promiseResolution(cx, JS::GetPromiseResult(resultObj));
            break;
        }
        case JS::PromiseState::Rejected: {
            JS::RootedValue promiseRejection(cx, JS::GetPromiseResult(resultObj));
            JS_SetPendingException(cx, promiseRejection);
            abort(cx, "module evaluation failed");
        }
        case JS::PromiseState::Pending: {
            abort(cx, "module evaluation returned a pending promise, but top-level await isn't enabled yet");
        }
        default:
            abort(cx, "module evaluation returned a promise in an unknown promise state");
        }
    }

  done_handling_promise:

    init_user_module(cx, module);

    JS::PrepareForFullGC(cx);
    JS::NonIncrementalGC(cx, JS::GCOptions::Shrink, JS::GCReason::API);

    free(jsFileName);
}

} // namespace smw
