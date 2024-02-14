#pragma once
// minimal WAMR header mock-up for compilation tests
#include <stdint.h>
struct WASMExecEnv;
typedef WASMExecEnv* wasm_exec_env_t;
struct WASMModuleInstanceCommon;
typedef WASMModuleInstanceCommon* wasm_module_inst_t;
typedef void* wasm_function_inst_t;
typedef uint8_t wasm_valkind_t;
enum wasm_valkind_enum {
    WASM_I32,
    WASM_I64,
    WASM_F32,
    WASM_F64,
};
typedef struct wasm_val_t {
    wasm_valkind_t kind;
    uint8_t __padding[7];
    union {
        int32_t i32;
        int64_t i64;
        float f32;
        double f64;
    } of;
} wasm_val_t;
wasm_module_inst_t wasm_runtime_get_module_inst(wasm_exec_env_t);
void* wasm_runtime_addr_app_to_native(wasm_module_inst_t,int32_t);
struct NativeSymbol {
    const char* name;
    void* func;
    const char* signature;
    void* env;
};
void wasm_runtime_register_natives(char const* module, NativeSymbol const*, unsigned);
bool wasm_runtime_call_wasm_a(wasm_exec_env_t, wasm_function_inst_t, uint32_t, wasm_val_t*, uint32_t, wasm_val_t*);
wasm_function_inst_t wasm_runtime_lookup_function(wasm_module_inst_t, const char*, const char*);
#define WASM_INIT_VAL {.kind = WASM_I32, .of = {.i32 = 0}}
#define WASM_I32_VAL(x) {.kind = WASM_I32, .of = {.i32 =(x)}}
#define WASM_I64_VAL(x) {.kind = WASM_I64, .of = {.i64 =(x)}}
#define WASM_F32_VAL(x) {.kind = WASM_F32, .of = {.f32 =(x)}}
#define WASM_F64_VAL(x) {.kind = WASM_F64, .of = {.f64 =(x)}}
