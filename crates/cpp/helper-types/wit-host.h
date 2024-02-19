#pragma once

#include <malloc.h>
#include <stdint.h>
#include <string_view>
#include "wit-common.h"

#ifndef WIT_HOST_DIRECT
#define WIT_HOST_WAMR
#endif

#ifdef WIT_HOST_DIRECT
#define WIT_WASI64
#endif

namespace wit {
#ifdef WIT_WASI64
typedef uint64_t guest_address;
typedef uint64_t guest_size;
#else
typedef uint32_t guest_address;
typedef uint32_t guest_size;
#endif
} // namespace wit

#ifdef WIT_HOST_WAMR
#include <wasm_export.h>
#endif

namespace wit {
typedef void (*guest_cabi_post_t)(WASMExecEnv *, guest_address);
typedef guest_address (*guest_alloc_t)(WASMExecEnv *, guest_size size,
                                       guest_size align);

// host code never de-allocates directly
class string {
  guest_address data_;
  guest_size length;

public:
#ifdef WIT_HOST_WAMR
  std::string_view get_view(WASMExecEnv *inst) const {
    return std::string_view((char const *)wasm_runtime_addr_app_to_native(
                                wasm_runtime_get_module_inst(inst), data_),
                            length);
  }
#elif defined(WIT_HOST_DIRECT)
  std::string_view get_view() const {
    return std::string_view((char const *)data_, length);
  }
#endif
  string(guest_address a, guest_size s) : data_(a), length(s) {}
  guest_address data() const { return data_; }
  guest_size size() const { return length; }
  // add a convenient way to create a string
};

template <class T>
class vector {
  guest_address data_;
  guest_size length;

public:
#ifdef WIT_HOST_WAMR
  std::string_view get_view(WASMExecEnv *inst) const {
    return wit::span((T const *)wasm_runtime_addr_app_to_native(
                                wasm_runtime_get_module_inst(inst), data_),
                            length);
  }
#elif defined(WIT_HOST_DIRECT)
  std::string_view get_view() const {
    return wit::span((T const *)data_, length);
  }
#endif
  vector(guest_address a, guest_size s) : data_(a), length(s) {}
  guest_address data() const { return data_; }
  guest_size size() const { return length; }
};

template <class T> class guest_owned : public T {
  guest_address data_;
#ifdef WIT_HOST_WAMR
  wasm_function_inst_t free_func;
  WASMExecEnv *exec_env;
#elif defined(WIT_HOST_DIRECT)
  void (*free_func)(guest_address);
#endif
public:
  guest_owned(guest_owned const &) = delete;
  guest_owned &operator=(guest_owned const &) = delete;
  ~guest_owned() {
    if (data_) {
#ifdef WIT_HOST_WAMR
      wasm_val_t *wasm_results = nullptr;
      wasm_val_t wasm_args[1] = {
          WASM_I32_VAL(data_),
      };
      wasm_runtime_call_wasm_a(exec_env, free_func, 0, wasm_results, 1,
                               wasm_args);
#elif defined(WIT_HOST_DIRECT)
      (*free_func)(data_);
#endif
    }
  }
  guest_owned(guest_owned &&b)
      : T(b), data_(b.data_), free_func(b.free_func)
#ifdef WIT_HOST_WAMR
        ,
        exec_env(b.exec_env)
#endif
  {
    b.data_ = nullptr;
  }
  guest_owned(T &&t, guest_address a,
#ifdef WIT_HOST_WAMR
              wasm_function_inst_t f, WASMExecEnv *e
#elif defined(WIT_HOST_DIRECT)
              , void (*f)(guest_address)
#endif
              )
      : T(std::move(t)), data_(a), free_func(f)
#ifdef WIT_HOST_WAMR
        ,
        exec_env(e)
#endif
  {
  }

#ifdef WIT_HOST_WAMR
  // not necessary? as the only way to get a guest_owned object
  // is to pass exec_env
  // WASMExecEnv* get_exec_env() const {
  //     return exec_env;
  // }
#endif
};
} // namespace wit
