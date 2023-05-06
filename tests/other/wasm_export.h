typedef void* wasm_exec_env_t;
typedef void* wasm_module_inst_t;
wasm_module_inst_t wasm_runtime_get_module_inst(wasm_exec_env_t);
void* wasm_runtime_addr_app_to_native(wasm_module_inst_t,int32_t);
struct NativeSymbol {
    const char* name;
    void* func;
    const char* signature;
    void* env;
};
void wasm_runtime_register_natives(char const* module, NativeSymbol const*, unsigned);
