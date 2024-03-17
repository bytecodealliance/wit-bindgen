#pragma once

#include <malloc.h>
#include <stdint.h>
#include <string.h>
#include <string_view>
#include <optional>
#include "wit-common.h"

#ifndef WIT_HOST_DIRECT
#define WIT_HOST_WAMR
#endif

// #ifdef WIT_HOST_DIRECT
// #define WIT_WASI64
// #endif

namespace wit {
#ifdef WIT_HOST_DIRECT
typedef uint8_t* guest_address;
typedef size_t guest_size;
extern "C" void *cabi_realloc(void *ptr, size_t old_size, size_t align,
                              size_t new_size);
#elif defined(WIT_WASI64)
typedef uint64_t guest_address;
typedef uint64_t guest_size;
#else
typedef uint32_t guest_address;
typedef uint32_t guest_size;
#endif
} // namespace wit

#ifdef WIT_HOST_WAMR
#include <wasm_export.h>
#include <wasm_c_api.h>
#endif

namespace wit {
#ifdef WIT_HOST_WAMR
typedef void (*guest_cabi_post_t)(WASMExecEnv *, guest_address);
typedef guest_address (*guest_alloc_t)(WASMExecEnv *, guest_size size,
                                       guest_size align);
#endif

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

#if defined(WIT_HOST_DIRECT)
  static string from_view(std::string_view v) {
    void* addr = cabi_realloc(nullptr, 0, 1, v.length());
    memcpy(addr, v.data(), v.length());
    return string((guest_address)addr, v.length());
  }
#endif
#if defined(WIT_HOST_WAMR)
  static string from_view(wasm_exec_env_t exec_env, std::string_view v) {
    void* addr = nullptr;
    wasm_function_inst_t wasm_func = wasm_runtime_lookup_function(wasm_runtime_get_module_inst(exec_env), 
  "cabi_realloc", "(*$ii)*");

    wasm_val_t wasm_results[1] = {
      WASM_INIT_VAL
    };
    wasm_val_t wasm_args[4] = {
      WASM_I32_VAL(0 /*nullptr*/),
      WASM_I32_VAL(0),
      WASM_I32_VAL(1),
      WASM_I32_VAL(0 /*v.length()*/),
    };
    bool wasm_ok;
    wasm_args[3].of.i32 = v.length();
    wasm_ok = wasm_runtime_call_wasm_a(exec_env, wasm_func, 1, wasm_results, 4,
                              wasm_args);
    assert(wasm_ok);
    assert(wasm_results[0].kind==WASM_I32);
    auto ret = wasm_results[0].of.i32;
    addr = (void*)wasm_runtime_addr_app_to_native(wasm_runtime_get_module_inst(exec_env), 
      ret);
    memcpy(addr, v.data(), v.length());
    return string((guest_address)ret, v.length());
  }
#endif
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
          WASM_I32_VAL((int32_t)data_),
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
              void (*f)(guest_address)
#endif
              )
      : T(std::move(t)), data_(a), free_func(f)
#ifdef WIT_HOST_WAMR
        ,
        exec_env(e)
#endif
  {
  }
  T const& inner() const { return *static_cast<T const*>(this); }

#ifdef WIT_HOST_WAMR
  // not necessary? as the only way to get a guest_owned object
  // is to pass exec_env
  // WASMExecEnv* get_exec_env() const {
  //     return exec_env;
  // }
#endif
};

template <class R> class ResourceTable {
  static std::map<int32_t, R> resources;

public:
  static R *lookup_resource(int32_t id) {
    auto result = resources.find(id);
    return result == resources.end() ? nullptr : &result->second;
  }
  static int32_t store_resource(R &&value) {
    auto last = resources.rbegin();
    int32_t id = last == resources.rend() ? 0 : last->first + 1;
    resources.insert(std::pair<int32_t, R>(id, std::move(value)));
    return id;
  }
  static std::optional<R> remove_resource(int32_t id) { 
    auto iter = resources.find(id);
    std::optional<R> result;
    if (iter!=resources.end()) {
      result = std::move(iter->second);
      resources.erase(iter);
    }
    return std::move(result);
  }
};

// guest exported resource
class ResourceExportBase : public ResourceTable<guest_address> {
  protected:
    guest_address rep;
    int32_t index;
  public:
    ResourceExportBase() : rep(0), index(-1) {}
    ResourceExportBase(int32_t i) : rep(*lookup_resource(i)), index(i) {}
    ResourceExportBase(ResourceExportBase &&b) : rep(b.rep), index(b.index) {b.rep=0;}
    ResourceExportBase(ResourceExportBase const&) = delete;
    ResourceExportBase& operator=(ResourceExportBase const&)=delete;
    ResourceExportBase& operator=(ResourceExportBase &&b) {
      assert(rep==0);
      rep = b.rep;
      index = b.index;
      b.rep = 0;
    }
    int32_t get_handle() const { return index; }
    guest_address get_rep() const { return rep; }
    guest_address take_rep() { guest_address res = rep; rep=0; return res; }
};

template <class R>
class ResourceImportBase : public ResourceTable<R*> {
    int32_t index;
  public:
    static const int32_t invalid=-1;
    ResourceImportBase() : index(this->store_resource((R*)this)) {}
    ~ResourceImportBase() {}
    ResourceImportBase(ResourceImportBase &&b) = delete;
    ResourceImportBase(ResourceImportBase const&) = delete;
    ResourceImportBase& operator=(ResourceImportBase const&)=delete;
    ResourceImportBase& operator=(ResourceImportBase &&)=delete;
    int32_t get_handle() {
      return index;
    }
};

} // namespace wit
