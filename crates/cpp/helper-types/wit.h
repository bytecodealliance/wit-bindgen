#pragma once

#include <assert.h>
#include <map>
#include <optional>
#include <stddef.h> // size_t
#include <stdint.h>
#include <memory> // unique_ptr
#include <stdint.h>
#include <string>
#include <string_view>
#include <string.h> // memcpy
#include <stdlib.h> // free
#include <new>
#include <span>
#include <utility> // pair
#include <vector>
#include <coroutine>

namespace wit {
/// @brief Helper class to map between IDs and resources
/// @tparam R Type of the Resource
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
    if (iter != resources.end()) {
      result = std::move(iter->second);
      resources.erase(iter);
    }
    return std::move(result);
  }
};

/// @brief Replaces void in the error position of a result
struct Void {};

/// A string in linear memory, freed unconditionally using free
///
/// A normal C++ string makes no guarantees about where the characters
/// are stored and how this is freed.
class string {
  uint8_t const *data_;
  size_t length;
  // C++ is horrible!
  //constexpr uint8_t const *const empty_ptr = (uint8_t const *)1;
  static uint8_t const* empty_ptr() { return (uint8_t const *)1; }

public:
  // this constructor is helpful for creating vector<string>
  string(string const &b) : string(string::from_view(b.get_view())) {}
  string(string &&b) : data_(b.data_), length(b.length) { b.data_ = nullptr; }
  string &operator=(string const &) = delete;
  string &operator=(string &&b) {
    if (data_ && data_!=empty_ptr()) {
      free(const_cast<uint8_t *>(data_));
    }
    data_ = b.data_;
    length = b.length;
    b.data_ = nullptr;
    return *this;
  }
  string(char const *d, size_t l) : data_((uint8_t const *)d), length(l) {}
  char const *data() const { return (char const *)data_; }
  size_t size() const { return length; }
  bool empty() const { return !length; }
  ~string() {
    if (data_ && data_!=empty_ptr()) {
      free(const_cast<uint8_t *>(data_));
    }
  }
  // leak the memory
  void leak() { data_ = nullptr; }
  // typically called by post
  static void drop_raw(void *ptr) { free(ptr); }
  std::string_view get_view() const {
    return std::string_view((const char *)data_, length);
  }
  std::string to_string() const {
    return std::string((const char *)data_, length);
  }
  static string from_view(std::string_view v) {
    if (!v.size()) return string((char const*)empty_ptr(), 0);
    char* addr = (char*)malloc(v.size());
    memcpy(addr, v.data(), v.size());
    return string(addr, v.size());
  }
  char* begin() {
    return (char*)data_;
  }
  char* end() {
    return (char*)data_ + length;
  }
  char const* begin() const {
    return (char const*)data_;
  }
  char const* end() const {
    return (char const*)data_ + length;
  }
};

/// A vector in linear memory, freed unconditionally using free
///
/// You can't detach the data memory from a vector, nor create one
/// in a portable way from a buffer and lenght without copying.
template <class T> class vector {
  T *data_;
  size_t length;

  static T* empty_ptr() { return (T*)alignof(T); }

public:
  vector(vector const &) = delete;
  vector(vector &&b) : data_(b.data_), length(b.length) { b.data_ = nullptr; }
  vector &operator=(vector const &) = delete;
  vector &operator=(vector &&b) {
    if (data_ && length>0) {
      free(data_);
    }
    data_ = b.data_;
    length = b.length;
    b.data_ = nullptr;
    return *this;
  }
  vector(T *d, size_t l) : data_(d), length(l) {}
  // Rust needs a nonzero pointer here (alignment is typical)
  vector() : data_(empty_ptr()), length() {}
  T const *data() const { return data_; }
  T *data() { return data_; }
  T &operator[](size_t n) { return data_[n]; }
  T const &operator[](size_t n) const { return data_[n]; }
  size_t size() const { return length; }
  bool empty() const { return !length; }
  ~vector() {
    if (data_ && length>0) {
      for (unsigned i=0;i<length;++i) { data_[i].~T(); }
      free((void*)data_);
    }
  }
  // WARNING: vector contains uninitialized elements
  static vector<T> allocate(size_t len) {
    if (!len) return vector<T>(empty_ptr(), 0);
    return vector<T>((T*)malloc(sizeof(T)*len), len);
  }
  void initialize(size_t n, T&& elem) {
    new ((void*)(data_+n)) T(std::move(elem));
  }
  // leak the memory
  T* leak() { T*result = data_; data_ = nullptr; return result; }
  // typically called by post
  static void drop_raw(void *ptr) { if (ptr!=empty_ptr()) free(ptr); }
  std::span<T> get_view() const { return std::span<T>(data_, length); }
  std::span<const T> get_const_view() const { return std::span<const T>(data_, length); }
  template <class U> static vector<T> from_view(std::span<U> const& a) {
    auto result = vector<T>::allocate(a.size());
    for (uint32_t i=0;i<a.size();++i) {
      new ((void*)(result.data_+i)) T(a[i]);
    }
    return result;
  } 
};

/// @brief A map stored as a contiguous array of key-value pairs in linear
/// memory, freed unconditionally using free.
///
/// Mirrors the canonical ABI representation of `map<K, V>` (`list<tuple<K, V>>`)
/// to enable lift without per-entry tree allocation. The container has no
/// ordering or hashing guarantees and exposes only the subset of the
/// `std::unordered_map` API that's meaningful over a flat pair buffer plus
/// the bindings-construction primitives (`allocate`, `initialize`, `leak`,
/// `drop_raw`, `data`).
template <class K, class V> class unordered_map {
public:
  using entry_type = std::pair<K, V>;

private:
  entry_type *data_;
  size_t length;

  static entry_type* empty_ptr() { return (entry_type*)alignof(entry_type); }

public:
  unordered_map(unordered_map const &) = delete;
  unordered_map(unordered_map &&b) : data_(b.data_), length(b.length) {
    b.data_ = nullptr;
    b.length = 0;
  }
  unordered_map &operator=(unordered_map const &) = delete;
  unordered_map &operator=(unordered_map &&b) {
    if (data_ && length > 0) {
      for (unsigned i = 0; i < length; ++i) { data_[i].~entry_type(); }
      free(data_);
    }
    data_ = b.data_;
    length = b.length;
    b.data_ = nullptr;
    b.length = 0;
    return *this;
  }
  unordered_map(entry_type *d, size_t l) : data_(d), length(l) {}
  unordered_map() : data_(empty_ptr()), length() {}
  entry_type const *data() const { return data_; }
  entry_type *data() { return data_; }
  size_t size() const { return length; }
  bool empty() const { return !length; }
  ~unordered_map() {
    if (data_ && length > 0) {
      for (unsigned i = 0; i < length; ++i) { data_[i].~entry_type(); }
      free((void*)data_);
    }
  }
  // WARNING: unordered_map contains uninitialized entries; caller must
  // construct them via `initialize` before the map is observed or destroyed.
  static unordered_map<K, V> allocate(size_t len) {
    if (!len) return unordered_map<K, V>(empty_ptr(), 0);
    return unordered_map<K, V>(
        (entry_type*)malloc(sizeof(entry_type) * len), len);
  }
  void initialize(size_t n, entry_type&& entry) {
    new ((void*)(data_ + n)) entry_type(std::move(entry));
  }
  entry_type *leak() {
    entry_type *result = data_;
    data_ = nullptr;
    return result;
  }
  static void drop_raw(void *ptr) {
    if (ptr != empty_ptr()) free(ptr);
  }
  entry_type *begin() { return data_; }
  entry_type *end() { return data_ + length; }
  entry_type const *begin() const { return data_; }
  entry_type const *end() const { return data_ + length; }
};

/// @brief  A Resource defined within the guest (guest side)
///
/// It registers with the host and should remain in a static location.
/// Typically referenced by the Owned type
///
/// Note that deregistering will cause the host to call Dtor which
/// in turn frees the object.
template <class R> class ResourceExportBase {
public:
  struct Deregister {
    void operator()(R *ptr) const {
      // probably always true because of unique_ptr wrapping, TODO: check
#ifdef WIT_SYMMETRIC
      if (ptr->handle != nullptr)
#else
      if (ptr->handle >= 0)
#endif
      {
        // we can't deallocate because the host calls Dtor
        R::ResourceDrop(ptr->handle);
      }
    }
  };
  typedef std::unique_ptr<R, Deregister> Owned;

#ifdef WIT_SYMMETRIC
  typedef uint8_t *handle_t;
  static constexpr handle_t invalid = nullptr;
#else
  typedef int32_t handle_t;
  static const handle_t invalid = -1;
#endif

  handle_t handle;

  ResourceExportBase() : handle(R::ResourceNew((R *)this)) {}
  // because this function is called by the host via Dtor we must not deregister
  ~ResourceExportBase() {}
  ResourceExportBase(ResourceExportBase const &) = delete;
  ResourceExportBase(ResourceExportBase &&) = delete;
  ResourceExportBase &operator=(ResourceExportBase &&b) = delete;
  ResourceExportBase &operator=(ResourceExportBase const &) = delete;
  handle_t get_handle() const { return handle; }
  handle_t into_handle() {
    handle_t result = handle;
    handle = invalid;
    return result;
  }
};

/// @brief A Resource imported from the host (guest side)
///
/// Wraps the identifier and can be forwarded but not duplicated
class ResourceImportBase {
public:
#ifdef WIT_SYMMETRIC
  typedef uint8_t *handle_t;
  static constexpr handle_t invalid = nullptr;
#else
  typedef int32_t handle_t;
  static const handle_t invalid = -1;
#endif

protected:
  handle_t handle;

public:
  ResourceImportBase(handle_t h = invalid) : handle(h) {}
  ResourceImportBase(ResourceImportBase &&r) : handle(r.handle) {
    r.handle = invalid;
  }
  ResourceImportBase(ResourceImportBase const &) = delete;
  void set_handle(handle_t h) { handle = h; }
  handle_t get_handle() const { return handle; }
  handle_t into_handle() {
    handle_t h = handle;
    handle = invalid;
    return h;
  }
  ResourceImportBase &operator=(ResourceImportBase &&r) {
    assert(handle == invalid);
    handle = r.handle;
    r.handle = invalid;
    return *this;
  }
  ResourceImportBase &operator=(ResourceImportBase const &r) = delete;
};

enum SubtaskState : uint32_t {
  SUBTASK_STARTING = 0,
  SUBTASK_STARTED = 1,
  SUBTASK_RETURNED = 2,
  SUBTASK_STARTED_CANCELLED = 3,
  SUBTASK_RETURNED_CANCELLED = 4,
};

enum CallbackCode : uint32_t {
  CALLBACK_CODE_EXIT = 0,
  CALLBACK_CODE_YIELD = 1,
  CALLBACK_CODE_WAIT = 2,
};

enum EventCode : uint32_t {
  EVENT_NONE = 0,
  EVENT_SUBTASK = 1,
  EVENT_STREAM_READ = 2,
  EVENT_STREAM_WRITE = 3,
  EVENT_FUTURE_READ = 4,
  EVENT_FUTURE_WRITE = 5,
  EVENT_CANCEL = 6,
};

enum WaitableState : uint32_t {
  WAITABLE_COMPLETED = 0,
  WAITABLE_DROPPED = 1,
  WAITABLE_CANCELLED = 2,
};

inline constexpr uint32_t WAITABLE_STATUS_BLOCKED = 0xffffffffU;

namespace detail {

#ifdef __wasm__
__attribute__((__import_module__("$root"), __import_name__("[waitable-set-new]")))
extern "C" uint32_t wit_async_waitable_set_new();

__attribute__((__import_module__("$root"), __import_name__("[waitable-join]")))
extern "C" void wit_async_waitable_join(uint32_t waitable, uint32_t set);

__attribute__((__import_module__("$root"), __import_name__("[waitable-set-wait]")))
extern "C" uint32_t wit_async_waitable_set_wait(uint32_t set, uint32_t *payload);

__attribute__((__import_module__("$root"), __import_name__("[waitable-set-poll]")))
extern "C" uint32_t wit_async_waitable_set_poll(uint32_t set, uint32_t *payload);

__attribute__((__import_module__("$root"), __import_name__("[waitable-set-drop]")))
extern "C" void wit_async_waitable_set_drop(uint32_t set);

__attribute__((__import_module__("$root"), __import_name__("[subtask-drop]")))
extern "C" void wit_async_subtask_drop(uint32_t subtask);

__attribute__((__import_module__("$root"), __import_name__("[subtask-cancel]")))
extern "C" uint32_t wit_async_subtask_cancel(uint32_t subtask);

__attribute__((__import_module__("$root"), __import_name__("[thread-yield]")))
extern "C" uint32_t wit_async_thread_yield();

__attribute__((__import_module__("$root"), __import_name__("[backpressure-inc]")))
extern "C" void wit_async_backpressure_inc();

__attribute__((__import_module__("$root"), __import_name__("[backpressure-dec]")))
extern "C" void wit_async_backpressure_dec();

__attribute__((__import_module__("[export]$root"), __import_name__("[task-cancel]")))
extern "C" void wit_async_task_cancel();

#ifdef __wasm_libcall_thread_context__
inline void *&wit_async_tls_slot() {
  static _Thread_local void *slot = nullptr;
  return slot;
}
inline void *wit_async_context_get() { return wit_async_tls_slot(); }
inline void wit_async_context_set(void *val) { wit_async_tls_slot() = val; }
#else
__attribute__((__import_module__("$root"), __import_name__("[context-get-0]")))
extern "C" void *wit_async_context_get_0();

__attribute__((__import_module__("$root"), __import_name__("[context-set-0]")))
extern "C" void wit_async_context_set_0(void *val);

inline void *wit_async_context_get() { return wit_async_context_get_0(); }
inline void wit_async_context_set(void *val) { wit_async_context_set_0(val); }
#endif

#else
inline uint32_t wit_async_waitable_set_new() { return 1; }
inline void wit_async_waitable_join(uint32_t, uint32_t) {}
inline uint32_t wit_async_waitable_set_wait(uint32_t, uint32_t *) { return 0; }
inline uint32_t wit_async_waitable_set_poll(uint32_t, uint32_t *) { return 0; }
inline void wit_async_waitable_set_drop(uint32_t) {}
inline void wit_async_subtask_drop(uint32_t) {}
inline uint32_t wit_async_subtask_cancel(uint32_t) { return 0; }
inline uint32_t wit_async_thread_yield() { return 0; }
inline void wit_async_backpressure_inc() {}
inline void wit_async_backpressure_dec() {}
inline void wit_async_task_cancel() {}
inline void *&wit_async_tls_slot() {
  static void *slot = nullptr;
  return slot;
}
inline void *wit_async_context_get() { return wit_async_tls_slot(); }
inline void wit_async_context_set(void *val) { wit_async_tls_slot() = val; }
#endif

struct WaitableEntry {
  uint32_t waitable = 0;
  std::coroutine_handle<> continuation = nullptr;
  uint32_t *out_code = nullptr;
  bool is_subtask = false;
  bool active = false;
  uint32_t (*cancel_fn)(uint32_t) = nullptr;
};

struct AsyncTaskContext {
  std::coroutine_handle<> root_handle = nullptr;
  bool owns_root = true;
  uint32_t waitable_set = 0;
  bool should_yield = false;
  bool cancelled = false;
  std::vector<WaitableEntry> entries;
  std::vector<std::coroutine_handle<>> ready_queue;
  std::vector<std::coroutine_handle<>> spawned_handles;

  AsyncTaskContext() = default;
  ~AsyncTaskContext() {
    destroy_coroutines();
    for (auto &entry : entries) {
      if (entry.active) {
        entry.active = false;
        wit_async_waitable_join(entry.waitable, 0);
        if (entry.cancel_fn) {
          entry.cancel_fn(entry.waitable);
        }
        if (entry.is_subtask) {
          wit_async_subtask_drop(entry.waitable);
        }
      }
    }
    if (waitable_set != 0) {
      wit_async_waitable_set_drop(waitable_set);
      waitable_set = 0;
    }
  }

  AsyncTaskContext(const AsyncTaskContext &) = delete;
  AsyncTaskContext &operator=(const AsyncTaskContext &) = delete;

  void destroy_coroutines() {
    for (auto h : spawned_handles) {
      if (h) {
        h.destroy();
      }
    }
    spawned_handles.clear();
    if (owns_root && root_handle) {
      auto h = root_handle;
      root_handle = nullptr;
      h.destroy();
    }
  }

  void cleanup_finished() {
    for (size_t i = 0; i < spawned_handles.size();) {
      auto h = spawned_handles[i];
      if (!h || h.done()) {
        if (h) {
          h.destroy();
        }
        spawned_handles.erase(spawned_handles.begin() + i);
      } else {
        ++i;
      }
    }
    if (owns_root && root_handle && root_handle.done()) {
      auto h = root_handle;
      root_handle = nullptr;
      h.destroy();
    }
  }

  bool has_pending() const {
    if (!ready_queue.empty()) {
      return true;
    }
    for (const auto &entry : entries) {
      if (entry.active) {
        return true;
      }
    }
    return false;
  }

  bool is_done() const {
    if (cancelled) {
      return true;
    }
    bool root_done = !root_handle || root_handle.done();
    return root_done && spawned_handles.empty() && !has_pending();
  }

  void register_waitable(uint32_t waitable, std::coroutine_handle<> cont,
                         uint32_t *out_code, bool is_subtask,
                         uint32_t (*cancel_fn)(uint32_t) = nullptr) {
    if (waitable_set == 0) {
      waitable_set = wit_async_waitable_set_new();
    }
    wit_async_waitable_join(waitable, waitable_set);
    for (auto &entry : entries) {
      if (!entry.active) {
        entry = WaitableEntry{waitable, cont, out_code, is_subtask, true, cancel_fn};
        return;
      }
    }
    entries.push_back(
        WaitableEntry{waitable, cont, out_code, is_subtask, true, cancel_fn});
  }

  void cancel_ready(std::coroutine_handle<> cont) {
    for (size_t i = 0; i < ready_queue.size();) {
      if (ready_queue[i] == cont) {
        ready_queue.erase(ready_queue.begin() + i);
      } else {
        ++i;
      }
    }
  }

  void cancel_waitable(uint32_t waitable, std::coroutine_handle<> cont) {
    cancel_ready(cont);
    for (auto &entry : entries) {
      if (entry.active && entry.waitable == waitable &&
          entry.continuation == cont) {
        entry.active = false;
        wit_async_waitable_join(waitable, 0);
        uint32_t code = entry.cancel_fn ? entry.cancel_fn(waitable) : 0;
        if (entry.out_code) {
          *entry.out_code = code;
        }
        if (entry.is_subtask) {
          wit_async_subtask_drop(waitable);
        }
        return;
      }
    }
  }

  void deliver_event(uint32_t event0, uint32_t event1, uint32_t event2) {
    if (event0 == EVENT_NONE) {
      return;
    }
    if (event0 == EVENT_CANCEL) {
      cancelled = true;
      ready_queue.clear();
      destroy_coroutines();
      for (auto &entry : entries) {
        if (entry.active) {
          entry.active = false;
          wit_async_waitable_join(entry.waitable, 0);
          if (entry.cancel_fn) {
            entry.cancel_fn(entry.waitable);
          }
          if (entry.is_subtask) {
            wit_async_subtask_drop(entry.waitable);
          }
        }
      }
      wit_async_task_cancel();
      return;
    }
    for (auto &entry : entries) {
      if (entry.active && entry.waitable == event1) {
        if (entry.is_subtask) {
          if (entry.out_code) {
            *entry.out_code = event2;
          }
          uint32_t state = event2 & 0xfU;
          if (state != SUBTASK_RETURNED &&
              state != SUBTASK_STARTED_CANCELLED &&
              state != SUBTASK_RETURNED_CANCELLED) {
            return;
          }
        }
        entry.active = false;
        wit_async_waitable_join(event1, 0);
        if (entry.is_subtask) {
          wit_async_subtask_drop(event1);
        }
        if (entry.out_code) {
          *entry.out_code = event2;
        }
        if (entry.continuation) {
          ready_queue.push_back(entry.continuation);
        }
        return;
      }
    }
  }

  uint32_t step() {
    cleanup_finished();
    if (is_done()) {
      return CALLBACK_CODE_EXIT;
    }
    while (!ready_queue.empty() && !should_yield) {
      auto h = ready_queue.front();
      ready_queue.erase(ready_queue.begin());
      h.resume();
      cleanup_finished();
      if (is_done()) {
        return CALLBACK_CODE_EXIT;
      }
    }
    if (is_done()) {
      return CALLBACK_CODE_EXIT;
    }
    if (should_yield) {
      should_yield = false;
      return CALLBACK_CODE_YIELD;
    }
    assert(waitable_set != 0);
    return CALLBACK_CODE_WAIT | (waitable_set << 4);
  }
};

inline AsyncTaskContext *&current_context() {
  static AsyncTaskContext *ctx = nullptr;
  return ctx;
}

inline AsyncTaskContext *ensure_current_context() {
  auto *&ctx = current_context();
  if (!ctx) {
    ctx = new AsyncTaskContext();
  }
  return ctx;
}

struct SubtaskAwaiter {
  using CleanupList = std::vector<std::pair<void (*)(uint32_t), uint32_t>>;
  uint32_t packed_status;
  uint32_t final_status = 0;
  CleanupList cleanup;
  AsyncTaskContext *ctx = nullptr;
  std::coroutine_handle<> handle = nullptr;
  bool pending = false;

  explicit SubtaskAwaiter(uint32_t status, CleanupList c = {})
      : packed_status(status), final_status(status), cleanup(std::move(c)) {
    if ((packed_status & 0xfU) != SUBTASK_STARTING) {
      cleanup.clear();
    }
  }
  SubtaskAwaiter(const SubtaskAwaiter &) = delete;
  SubtaskAwaiter &operator=(const SubtaskAwaiter &) = delete;

  ~SubtaskAwaiter() {
    if (pending && ctx) {
      ctx->cancel_waitable(packed_status >> 4, handle);
      if ((final_status & 0xfU) == SUBTASK_STARTED_CANCELLED) {
        for (auto &item : cleanup) {
          item.first(item.second);
        }
      }
    }
  }

  bool await_ready() const noexcept {
    return (packed_status & 0xfU) == SUBTASK_RETURNED;
  }
  void await_suspend(std::coroutine_handle<> h) noexcept {
    ctx = ensure_current_context();
    handle = h;
    pending = true;
    uint32_t subtask = packed_status >> 4;
    ctx->register_waitable(
        subtask, h, &final_status, /*is_subtask=*/true, wit_async_subtask_cancel);
  }
  uint32_t await_resume() noexcept {
    pending = false;
    return final_status;
  }
};

struct WaitableAwaiter {
  uint32_t waitable;
  uint32_t (*cancel_fn)(uint32_t) = nullptr;
  uint32_t code = 0;
  AsyncTaskContext *ctx = nullptr;
  std::coroutine_handle<> handle = nullptr;
  bool pending = false;

  WaitableAwaiter(uint32_t w, uint32_t (*cf)(uint32_t) = nullptr)
      : waitable(w), cancel_fn(cf) {}
  WaitableAwaiter(const WaitableAwaiter &) = delete;
  WaitableAwaiter &operator=(const WaitableAwaiter &) = delete;

  ~WaitableAwaiter() {
    if (pending && ctx) {
      ctx->cancel_waitable(waitable, handle);
    }
  }

  bool await_ready() const noexcept { return false; }
  void await_suspend(std::coroutine_handle<> h) noexcept {
    ctx = ensure_current_context();
    handle = h;
    pending = true;
    ctx->register_waitable(
        waitable, h, &code, /*is_subtask=*/false, cancel_fn);
  }
  uint32_t await_resume() noexcept {
    pending = false;
    return code;
  }
};

struct YieldAwaiter {
  AsyncTaskContext *ctx = nullptr;
  std::coroutine_handle<> handle = nullptr;
  bool pending = false;

  YieldAwaiter() = default;
  YieldAwaiter(const YieldAwaiter &) = delete;
  YieldAwaiter &operator=(const YieldAwaiter &) = delete;

  ~YieldAwaiter() {
    if (pending && ctx) {
      ctx->cancel_ready(handle);
    }
  }

  bool await_ready() const noexcept { return false; }
  void await_suspend(std::coroutine_handle<> h) noexcept {
    ctx = ensure_current_context();
    handle = h;
    pending = true;
    ctx->should_yield = true;
    ctx->ready_queue.push_back(h);
  }
  void await_resume() noexcept {
    pending = false;
  }
};

inline void drive_task_synchronously(std::coroutine_handle<> handle) {
  if (!handle || handle.done()) {
    return;
  }
  auto *&ctx = current_context();
  if (!ctx) {
    ctx = new AsyncTaskContext();
  }
  ctx->owns_root = false;
  ctx->root_handle = handle;
  uint32_t code = ctx->step();
  while (code != CALLBACK_CODE_EXIT && !ctx->is_done()) {
    if (code == CALLBACK_CODE_YIELD) {
      wit_async_thread_yield();
      code = ctx->step();
    } else {
      uint32_t ws = code >> 4;
      uint32_t payload[2] = {0, 0};
      uint32_t ev0 = wit_async_waitable_set_wait(ws, payload);
      ctx->deliver_event(ev0, payload[0], payload[1]);
      code = ctx->step();
    }
  }
  delete ctx;
  ctx = nullptr;
}

} // namespace detail

inline detail::YieldAwaiter yield_async() noexcept { return {}; }

inline bool yield_blocking() { return detail::wit_async_thread_yield() == 0; }

inline void backpressure_inc() { detail::wit_async_backpressure_inc(); }

inline void backpressure_dec() { detail::wit_async_backpressure_dec(); }

/// @brief Represents an ongoing asynchronous computation in C++20 coroutines.
template <typename T = void>
class Task {
public:
  struct promise_type;
  using handle_type = std::coroutine_handle<promise_type>;

  struct promise_type {
    std::optional<T> value;
    std::coroutine_handle<> continuation = nullptr;

    Task get_return_object() {
      return Task(handle_type::from_promise(*this));
    }
    std::suspend_never initial_suspend() noexcept { return {}; }
    auto final_suspend() noexcept {
      struct FinalAwaiter {
        bool await_ready() noexcept { return false; }
        std::coroutine_handle<> await_suspend(handle_type h) noexcept {
          auto cont = h.promise().continuation;
          if (cont) return cont;
          return std::noop_coroutine();
        }
        void await_resume() noexcept {}
      };
      return FinalAwaiter{};
    }
    void return_value(T v) {
      value = std::move(v);
    }
    void unhandled_exception() {
      assert(false && "unhandled exception in wit::Task");
    }
  };

  handle_type handle = nullptr;

  Task() = default;
  explicit Task(handle_type h) : handle(h) {}
  Task(Task &&other) noexcept : handle(other.handle) { other.handle = nullptr; }
  Task &operator=(Task &&other) noexcept {
    if (this != &other) {
      if (handle) handle.destroy();
      handle = other.handle;
      other.handle = nullptr;
    }
    return *this;
  }
  ~Task() {
    if (handle) handle.destroy();
  }
  Task(const Task &) = delete;
  Task &operator=(const Task &) = delete;

  bool await_ready() const noexcept {
    return !handle || handle.done();
  }
  void await_suspend(std::coroutine_handle<> awaiting) noexcept {
    handle.promise().continuation = awaiting;
  }
  T await_resume() {
    assert(handle && handle.promise().value.has_value());
    return std::move(*handle.promise().value);
  }

  bool done() const noexcept {
    return !handle || handle.done();
  }

  void wait() {
    if (handle && !handle.done()) {
      detail::drive_task_synchronously(handle);
    }
  }

  T get() {
    wait();
    assert(handle && handle.promise().value.has_value());
    return std::move(*handle.promise().value);
  }

  handle_type release() noexcept {
    auto h = handle;
    handle = nullptr;
    return h;
  }
};

template <>
class Task<void> {
public:
  struct promise_type;
  using handle_type = std::coroutine_handle<promise_type>;

  struct promise_type {
    std::coroutine_handle<> continuation = nullptr;

    Task get_return_object() {
      return Task(handle_type::from_promise(*this));
    }
    std::suspend_never initial_suspend() noexcept { return {}; }
    auto final_suspend() noexcept {
      struct FinalAwaiter {
        bool await_ready() noexcept { return false; }
        std::coroutine_handle<> await_suspend(handle_type h) noexcept {
          auto cont = h.promise().continuation;
          if (cont) return cont;
          return std::noop_coroutine();
        }
        void await_resume() noexcept {}
      };
      return FinalAwaiter{};
    }
    void return_void() {}
    void unhandled_exception() {
      assert(false && "unhandled exception in wit::Task");
    }
  };

  handle_type handle = nullptr;

  Task() = default;
  explicit Task(handle_type h) : handle(h) {}
  Task(Task &&other) noexcept : handle(other.handle) { other.handle = nullptr; }
  Task &operator=(Task &&other) noexcept {
    if (this != &other) {
      if (handle) handle.destroy();
      handle = other.handle;
      other.handle = nullptr;
    }
    return *this;
  }
  ~Task() {
    if (handle) handle.destroy();
  }
  Task(const Task &) = delete;
  Task &operator=(const Task &) = delete;

  bool await_ready() const noexcept {
    return !handle || handle.done();
  }
  void await_suspend(std::coroutine_handle<> awaiting) noexcept {
    handle.promise().continuation = awaiting;
  }
  void await_resume() {}

  bool done() const noexcept {
    return !handle || handle.done();
  }

  void wait() {
    if (handle && !handle.done()) {
      detail::drive_task_synchronously(handle);
    }
  }

  void get() { wait(); }

  handle_type release() noexcept {
    auto h = handle;
    handle = nullptr;
    return h;
  }
};

inline void spawn(Task<void> task) {
  if (!task.done()) {
    detail::ensure_current_context()->spawned_handles.push_back(task.release());
  }
}

template <typename F>
  requires(!std::is_same_v<std::decay_t<F>, Task<void>>)
inline void spawn(F &&func) {
  auto wrapper = [](std::decay_t<F> f) -> Task<void> { co_await f(); };
  spawn(wrapper(std::forward<F>(func)));
}

namespace detail {

template <typename F>
inline int32_t start_task(F &&make_task) {
  AsyncTaskContext *prev_ctx = current_context();
  current_context() = nullptr;
  void *saved_slot = wit_async_context_get();
  wit_async_context_set(nullptr);

  Task<void> task = make_task();
  AsyncTaskContext *ctx = current_context();

  if (!ctx) {
    assert(task.done());
    current_context() = prev_ctx;
    wit_async_context_set(saved_slot);
    return static_cast<int32_t>(CALLBACK_CODE_EXIT);
  }

  ctx->root_handle = task.release();
  uint32_t code = ctx->step();
  current_context() = prev_ctx;

  if (code == CALLBACK_CODE_EXIT) {
    delete ctx;
    wit_async_context_set(saved_slot);
    return static_cast<int32_t>(CALLBACK_CODE_EXIT);
  }

  assert(saved_slot == nullptr);
  wit_async_context_set(ctx);
  return static_cast<int32_t>(code);
}

inline int32_t callback(uint32_t event0, uint32_t event1, uint32_t event2) {
  auto *ctx = static_cast<AsyncTaskContext *>(wit_async_context_get());
  assert(ctx != nullptr);
  wit_async_context_set(nullptr);

  AsyncTaskContext *prev_ctx = current_context();
  current_context() = ctx;

  ctx->deliver_event(event0, event1, event2);
  uint32_t code = ctx->step();

  current_context() = prev_ctx;

  if (code == CALLBACK_CODE_EXIT) {
    delete ctx;
    return static_cast<int32_t>(CALLBACK_CODE_EXIT);
  }

  wit_async_context_set(ctx);
  return static_cast<int32_t>(code);
}

template <typename T>
struct FutureVtable {
  size_t size;
  size_t align;
  void (*lower)(T value, uint8_t *dst);
  void (*dealloc_lists)(uint8_t *dst);
  T (*lift)(uint8_t *src);
  uint32_t (*start_write)(uint32_t future, const uint8_t *val);
  uint32_t (*start_read)(uint32_t future, uint8_t *val);
  uint32_t (*cancel_write)(uint32_t future);
  uint32_t (*cancel_read)(uint32_t future);
  void (*drop_writable)(uint32_t future);
  void (*drop_readable)(uint32_t future);
  uint64_t (*new_future)();
};

template <>
struct FutureVtable<void> {
  size_t size;
  size_t align;
  void (*lower)(uint8_t *dst);
  void (*dealloc_lists)(uint8_t *dst);
  void (*lift)(uint8_t *src);
  uint32_t (*start_write)(uint32_t future, const uint8_t *val);
  uint32_t (*start_read)(uint32_t future, uint8_t *val);
  uint32_t (*cancel_write)(uint32_t future);
  uint32_t (*cancel_read)(uint32_t future);
  void (*drop_writable)(uint32_t future);
  void (*drop_readable)(uint32_t future);
  uint64_t (*new_future)();
};

template <typename T>
struct StreamVtable {
  size_t size;
  size_t align;
  void (*lower)(T value, uint8_t *dst);
  void (*dealloc_lists)(uint8_t *dst);
  T (*lift)(uint8_t *src);
  uint32_t (*start_write)(uint32_t stream, const uint8_t *val, size_t amt);
  uint32_t (*start_read)(uint32_t stream, uint8_t *val, size_t amt);
  uint32_t (*cancel_write)(uint32_t stream);
  uint32_t (*cancel_read)(uint32_t stream);
  void (*drop_writable)(uint32_t stream);
  void (*drop_readable)(uint32_t stream);
  uint64_t (*new_stream)();
};

template <>
struct StreamVtable<void> {
  size_t size;
  size_t align;
  void (*lower)(uint8_t *dst);
  void (*dealloc_lists)(uint8_t *dst);
  void (*lift)(uint8_t *src);
  uint32_t (*start_write)(uint32_t stream, const uint8_t *val, size_t amt);
  uint32_t (*start_read)(uint32_t stream, uint8_t *val, size_t amt);
  uint32_t (*cancel_write)(uint32_t stream);
  uint32_t (*cancel_read)(uint32_t stream);
  void (*drop_writable)(uint32_t stream);
  void (*drop_readable)(uint32_t stream);
  uint64_t (*new_stream)();
};

template <typename T>
const FutureVtable<T> *get_future_vtable();
template <>
const FutureVtable<void> *get_future_vtable<void>();

template <typename T>
const StreamVtable<T> *get_stream_vtable();
template <>
const StreamVtable<void> *get_stream_vtable<void>();

} // namespace detail

template <typename T = void>
class FutureReader {
public:
  int32_t handle = -1;
  FutureReader() = default;
  explicit FutureReader(int32_t h) : handle(h) {}
  FutureReader(FutureReader &&other) noexcept : handle(other.handle) {
    other.handle = -1;
  }
  FutureReader &operator=(FutureReader &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_future_vtable<T>()->drop_readable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      other.handle = -1;
    }
    return *this;
  }
  ~FutureReader() {
    if (handle >= 0) {
      detail::get_future_vtable<T>()->drop_readable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  FutureReader(const FutureReader &) = delete;
  FutureReader &operator=(const FutureReader &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    return h;
  }
  int32_t into_subtask_handle(
      detail::SubtaskAwaiter::CleanupList &cleanup) {
    int32_t h = into_handle();
    if (h >= 0) {
      cleanup.emplace_back(detail::get_future_vtable<T>()->drop_readable,
                           static_cast<uint32_t>(h));
    }
    return h;
  }

  Task<T> read() {
    assert(handle >= 0);
    FutureReader<T> self(into_handle());
    const auto *vt = detail::get_future_vtable<T>();
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{vt->size > 0 ? static_cast<uint8_t *>(malloc(vt->size)) : nullptr};
    uint32_t h = static_cast<uint32_t>(self.get_handle());
    uint32_t status = vt->start_read(h, buf.ptr);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_read};
    }
    assert((status & 0xfU) == WAITABLE_COMPLETED);
    co_return vt->lift(buf.ptr);
  }

  Task<T> operator co_await() { return read(); }
};

template <>
class FutureReader<void> {
public:
  int32_t handle = -1;
  FutureReader() = default;
  explicit FutureReader(int32_t h) : handle(h) {}
  FutureReader(FutureReader &&other) noexcept : handle(other.handle) {
    other.handle = -1;
  }
  FutureReader &operator=(FutureReader &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_future_vtable<void>()->drop_readable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      other.handle = -1;
    }
    return *this;
  }
  ~FutureReader() {
    if (handle >= 0) {
      detail::get_future_vtable<void>()->drop_readable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  FutureReader(const FutureReader &) = delete;
  FutureReader &operator=(const FutureReader &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    return h;
  }
  int32_t into_subtask_handle(
      detail::SubtaskAwaiter::CleanupList &cleanup) {
    int32_t h = into_handle();
    if (h >= 0) {
      cleanup.emplace_back(detail::get_future_vtable<void>()->drop_readable,
                           static_cast<uint32_t>(h));
    }
    return h;
  }

  Task<void> read() {
    assert(handle >= 0);
    FutureReader<void> self(into_handle());
    const auto *vt = detail::get_future_vtable<void>();
    uint32_t h = static_cast<uint32_t>(self.get_handle());
    uint32_t status = vt->start_read(h, nullptr);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_read};
    }
    assert((status & 0xfU) == WAITABLE_COMPLETED);
    co_return;
  }

  Task<void> operator co_await() { return read(); }
};

template <typename T = void>
class FutureWriter {
public:
  int32_t handle = -1;
  FutureWriter() = default;
  explicit FutureWriter(int32_t h) : handle(h) {}
  FutureWriter(FutureWriter &&other) noexcept : handle(other.handle) {
    other.handle = -1;
  }
  FutureWriter &operator=(FutureWriter &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_future_vtable<T>()->drop_writable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      other.handle = -1;
    }
    return *this;
  }
  ~FutureWriter() {
    if (handle >= 0) {
      detail::get_future_vtable<T>()->drop_writable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  FutureWriter(const FutureWriter &) = delete;
  FutureWriter &operator=(const FutureWriter &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    return h;
  }

  Task<bool> write(T value) {
    assert(handle >= 0);
    FutureWriter<T> self(into_handle());
    const auto *vt = detail::get_future_vtable<T>();
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{vt->size > 0 ? static_cast<uint8_t *>(malloc(vt->size)) : nullptr};
    vt->lower(std::move(value), buf.ptr);
    uint32_t h = static_cast<uint32_t>(self.get_handle());
    uint32_t status = vt->start_write(h, buf.ptr);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_write};
    }
    uint32_t state = status & 0xfU;
    if (state == WAITABLE_COMPLETED) {
      vt->dealloc_lists(buf.ptr);
      co_return true;
    } else {
      T unused = vt->lift(buf.ptr);
      (void)unused;
      co_return false;
    }
  }
};

template <>
class FutureWriter<void> {
public:
  int32_t handle = -1;
  FutureWriter() = default;
  explicit FutureWriter(int32_t h) : handle(h) {}
  FutureWriter(FutureWriter &&other) noexcept : handle(other.handle) {
    other.handle = -1;
  }
  FutureWriter &operator=(FutureWriter &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_future_vtable<void>()->drop_writable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      other.handle = -1;
    }
    return *this;
  }
  ~FutureWriter() {
    if (handle >= 0) {
      detail::get_future_vtable<void>()->drop_writable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  FutureWriter(const FutureWriter &) = delete;
  FutureWriter &operator=(const FutureWriter &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    return h;
  }

  Task<bool> write() {
    assert(handle >= 0);
    FutureWriter<void> self(into_handle());
    const auto *vt = detail::get_future_vtable<void>();
    uint32_t h = static_cast<uint32_t>(self.get_handle());
    uint32_t status = vt->start_write(h, nullptr);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_write};
    }
    uint32_t state = status & 0xfU;
    co_return state == WAITABLE_COMPLETED;
  }
};

template <typename T = void>
inline std::pair<FutureWriter<T>, FutureReader<T>> future_new() {
  const auto *vt = detail::get_future_vtable<T>();
  uint64_t handles = vt->new_future();
  int32_t reader = static_cast<int32_t>(handles & 0xffffffffULL);
  int32_t writer = static_cast<int32_t>(handles >> 32);
  return {FutureWriter<T>(writer), FutureReader<T>(reader)};
}

enum class StreamState {
  Complete,
  Dropped,
  Cancelled,
};

struct StreamStatus {
  StreamState state;
  size_t count;
};

template <typename T = void>
class StreamReader {
public:
  int32_t handle = -1;
  bool peer_dropped = false;
  StreamReader() = default;
  explicit StreamReader(int32_t h) : handle(h) {}
  StreamReader(StreamReader &&other) noexcept
      : handle(other.handle), peer_dropped(other.peer_dropped) {
    other.handle = -1;
    other.peer_dropped = false;
  }
  StreamReader &operator=(StreamReader &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_stream_vtable<T>()->drop_readable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      peer_dropped = other.peer_dropped;
      other.handle = -1;
      other.peer_dropped = false;
    }
    return *this;
  }
  ~StreamReader() {
    if (handle >= 0) {
      detail::get_stream_vtable<T>()->drop_readable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  StreamReader(const StreamReader &) = delete;
  StreamReader &operator=(const StreamReader &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    peer_dropped = false;
    return h;
  }
  int32_t into_subtask_handle(
      detail::SubtaskAwaiter::CleanupList &cleanup) {
    int32_t h = into_handle();
    if (h >= 0) {
      cleanup.emplace_back(detail::get_stream_vtable<T>()->drop_readable,
                           static_cast<uint32_t>(h));
    }
    return h;
  }

  Task<std::optional<T>> next() {
    assert(handle >= 0);
    if (peer_dropped) {
      co_return std::nullopt;
    }
    const auto *vt = detail::get_stream_vtable<T>();
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{vt->size > 0 ? static_cast<uint8_t *>(malloc(vt->size)) : nullptr};
    uint32_t h = static_cast<uint32_t>(handle);
    uint32_t status = vt->start_read(h, buf.ptr, 1);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_read};
    }
    uint32_t state = status & 0xfU;
    uint32_t amt = status >> 4;
    if (state == WAITABLE_DROPPED) {
      peer_dropped = true;
    }
    if (amt == 0 && (state == WAITABLE_DROPPED || state == WAITABLE_CANCELLED)) {
      co_return std::nullopt;
    }
    assert(amt == 1);
    co_return vt->lift(buf.ptr);
  }

  Task<std::pair<StreamStatus, std::vector<T>>> read_chunk(size_t max_count) {
    assert(handle >= 0);
    if (peer_dropped) {
      co_return std::make_pair(StreamStatus{StreamState::Dropped, 0},
                               std::vector<T>{});
    }
    const auto *vt = detail::get_stream_vtable<T>();
    size_t alloc_size = vt->size * max_count;
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{alloc_size > 0 ? static_cast<uint8_t *>(malloc(alloc_size)) : nullptr};
    uint32_t h = static_cast<uint32_t>(handle);
    uint32_t status = vt->start_read(h, buf.ptr, max_count);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_read};
    }
    uint32_t state = status & 0xfU;
    uint32_t amt = status >> 4;
    if (state == WAITABLE_DROPPED) {
      peer_dropped = true;
    }
    StreamState s = amt > 0                     ? StreamState::Complete
                    : state == WAITABLE_DROPPED ? StreamState::Dropped
                                                : StreamState::Cancelled;
    std::vector<T> result;
    result.reserve(amt);
    for (uint32_t i = 0; i < amt; ++i) {
      result.push_back(vt->lift(buf.ptr + i * vt->size));
    }
    co_return std::make_pair(StreamStatus{s, amt}, std::move(result));
  }

  Task<std::vector<T>> read(size_t max_count) {
    auto [st, vec] = co_await read_chunk(max_count);
    (void)st;
    co_return std::move(vec);
  }

  Task<std::vector<T>> collect() {
    std::vector<T> result;
    while (true) {
      auto item = co_await next();
      if (!item.has_value()) {
        break;
      }
      result.push_back(std::move(*item));
    }
    co_return result;
  }
};

template <>
class StreamReader<void> {
public:
  int32_t handle = -1;
  bool peer_dropped = false;
  StreamReader() = default;
  explicit StreamReader(int32_t h) : handle(h) {}
  StreamReader(StreamReader &&other) noexcept
      : handle(other.handle), peer_dropped(other.peer_dropped) {
    other.handle = -1;
    other.peer_dropped = false;
  }
  StreamReader &operator=(StreamReader &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_stream_vtable<void>()->drop_readable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      peer_dropped = other.peer_dropped;
      other.handle = -1;
      other.peer_dropped = false;
    }
    return *this;
  }
  ~StreamReader() {
    if (handle >= 0) {
      detail::get_stream_vtable<void>()->drop_readable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  StreamReader(const StreamReader &) = delete;
  StreamReader &operator=(const StreamReader &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    peer_dropped = false;
    return h;
  }
  int32_t into_subtask_handle(
      detail::SubtaskAwaiter::CleanupList &cleanup) {
    int32_t h = into_handle();
    if (h >= 0) {
      cleanup.emplace_back(detail::get_stream_vtable<void>()->drop_readable,
                           static_cast<uint32_t>(h));
    }
    return h;
  }

  Task<StreamStatus> read(size_t max_count = 1) {
    assert(handle >= 0);
    if (peer_dropped) {
      co_return StreamStatus{StreamState::Dropped, 0};
    }
    const auto *vt = detail::get_stream_vtable<void>();
    uint32_t h = static_cast<uint32_t>(handle);
    uint32_t status = vt->start_read(h, nullptr, max_count);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_read};
    }
    uint32_t state = status & 0xfU;
    uint32_t amt = status >> 4;
    if (state == WAITABLE_DROPPED) {
      peer_dropped = true;
    }
    StreamState s = amt > 0                     ? StreamState::Complete
                    : state == WAITABLE_DROPPED ? StreamState::Dropped
                                                : StreamState::Cancelled;
    co_return StreamStatus{s, amt};
  }

  Task<bool> next() {
    auto st = co_await read(1);
    co_return st.count > 0;
  }
};

template <typename T = void>
class StreamWriter {
public:
  int32_t handle = -1;
  bool peer_dropped = false;
  StreamWriter() = default;
  explicit StreamWriter(int32_t h) : handle(h) {}
  StreamWriter(StreamWriter &&other) noexcept
      : handle(other.handle), peer_dropped(other.peer_dropped) {
    other.handle = -1;
    other.peer_dropped = false;
  }
  StreamWriter &operator=(StreamWriter &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_stream_vtable<T>()->drop_writable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      peer_dropped = other.peer_dropped;
      other.handle = -1;
      other.peer_dropped = false;
    }
    return *this;
  }
  ~StreamWriter() {
    if (handle >= 0) {
      detail::get_stream_vtable<T>()->drop_writable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  StreamWriter(const StreamWriter &) = delete;
  StreamWriter &operator=(const StreamWriter &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    peer_dropped = false;
    return h;
  }

  Task<StreamStatus> write(std::vector<T> values) {
    assert(handle >= 0);
    if (peer_dropped) {
      co_return StreamStatus{StreamState::Dropped, 0};
    }
    const auto *vt = detail::get_stream_vtable<T>();
    size_t count = values.size();
    size_t alloc_size = vt->size * count;
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{alloc_size > 0 ? static_cast<uint8_t *>(malloc(alloc_size)) : nullptr};
    for (size_t i = 0; i < count; ++i) {
      vt->lower(std::move(values[i]), buf.ptr + i * vt->size);
    }
    uint32_t h = static_cast<uint32_t>(handle);
    uint32_t status = vt->start_write(h, buf.ptr, count);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_write};
    }
    uint32_t state = status & 0xfU;
    uint32_t amt = status >> 4;
    if (state == WAITABLE_DROPPED) {
      peer_dropped = true;
    }
    for (uint32_t i = 0; i < amt; ++i) {
      vt->dealloc_lists(buf.ptr + i * vt->size);
    }
    for (size_t i = amt; i < count; ++i) {
      T unused = vt->lift(buf.ptr + i * vt->size);
      (void)unused;
    }
    StreamState s = amt > 0                     ? StreamState::Complete
                    : state == WAITABLE_DROPPED ? StreamState::Dropped
                                                : StreamState::Cancelled;
    co_return StreamStatus{s, amt};
  }

  Task<bool> write_one(T value) {
    assert(handle >= 0);
    if (peer_dropped) {
      co_return false;
    }
    const auto *vt = detail::get_stream_vtable<T>();
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{vt->size > 0 ? static_cast<uint8_t *>(malloc(vt->size)) : nullptr};
    vt->lower(std::move(value), buf.ptr);
    uint32_t h = static_cast<uint32_t>(handle);
    uint32_t status = vt->start_write(h, buf.ptr, 1);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_write};
    }
    uint32_t state = status & 0xfU;
    uint32_t amt = status >> 4;
    if (state == WAITABLE_DROPPED) {
      peer_dropped = true;
    }
    if (amt == 1) {
      vt->dealloc_lists(buf.ptr);
      co_return true;
    } else {
      T unused = vt->lift(buf.ptr);
      (void)unused;
      co_return false;
    }
  }

  Task<size_t> write_all(std::vector<T> values) {
    assert(handle >= 0);
    if (values.empty() || peer_dropped) {
      co_return 0;
    }
    const auto *vt = detail::get_stream_vtable<T>();
    size_t count = values.size();
    size_t alloc_size = vt->size * count;
    struct BufferGuard {
      uint8_t *ptr;
      ~BufferGuard() {
        if (ptr) free(ptr);
      }
    } buf{alloc_size > 0 ? static_cast<uint8_t *>(malloc(alloc_size)) : nullptr};
    for (size_t i = 0; i < count; ++i) {
      vt->lower(std::move(values[i]), buf.ptr + i * vt->size);
    }
    size_t cursor = 0;
    uint32_t h = static_cast<uint32_t>(handle);
    while (cursor < count) {
      uint32_t status =
          vt->start_write(h, buf.ptr + cursor * vt->size, count - cursor);
      if (status == WAITABLE_STATUS_BLOCKED) {
        status = co_await detail::WaitableAwaiter{h, vt->cancel_write};
      }
      uint32_t state = status & 0xfU;
      uint32_t amt = status >> 4;
      for (uint32_t i = 0; i < amt; ++i) {
        vt->dealloc_lists(buf.ptr + (cursor + i) * vt->size);
      }
      cursor += amt;
      if (state == WAITABLE_DROPPED) {
        peer_dropped = true;
        break;
      }
    }
    for (size_t i = cursor; i < count; ++i) {
      T unused = vt->lift(buf.ptr + i * vt->size);
      (void)unused;
    }
    co_return cursor;
  }

  Task<size_t> write_all(wit::vector<T> values) {
    std::vector<T> vec;
    vec.reserve(values.size());
    for (size_t i = 0; i < values.size(); ++i) {
      vec.push_back(std::move(values[i]));
    }
    co_return co_await write_all(std::move(vec));
  }
};

template <>
class StreamWriter<void> {
public:
  int32_t handle = -1;
  bool peer_dropped = false;
  StreamWriter() = default;
  explicit StreamWriter(int32_t h) : handle(h) {}
  StreamWriter(StreamWriter &&other) noexcept
      : handle(other.handle), peer_dropped(other.peer_dropped) {
    other.handle = -1;
    other.peer_dropped = false;
  }
  StreamWriter &operator=(StreamWriter &&other) noexcept {
    if (this != &other) {
      if (handle >= 0) {
        detail::get_stream_vtable<void>()->drop_writable(
            static_cast<uint32_t>(handle));
      }
      handle = other.handle;
      peer_dropped = other.peer_dropped;
      other.handle = -1;
      other.peer_dropped = false;
    }
    return *this;
  }
  ~StreamWriter() {
    if (handle >= 0) {
      detail::get_stream_vtable<void>()->drop_writable(
          static_cast<uint32_t>(handle));
      handle = -1;
    }
  }
  StreamWriter(const StreamWriter &) = delete;
  StreamWriter &operator=(const StreamWriter &) = delete;

  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    peer_dropped = false;
    return h;
  }

  Task<StreamStatus> write(size_t count = 1) {
    assert(handle >= 0);
    if (peer_dropped) {
      co_return StreamStatus{StreamState::Dropped, 0};
    }
    const auto *vt = detail::get_stream_vtable<void>();
    uint32_t h = static_cast<uint32_t>(handle);
    uint32_t status = vt->start_write(h, nullptr, count);
    if (status == WAITABLE_STATUS_BLOCKED) {
      status = co_await detail::WaitableAwaiter{h, vt->cancel_write};
    }
    uint32_t state = status & 0xfU;
    uint32_t amt = status >> 4;
    if (state == WAITABLE_DROPPED) {
      peer_dropped = true;
    }
    StreamState s = amt > 0                     ? StreamState::Complete
                    : state == WAITABLE_DROPPED ? StreamState::Dropped
                                                : StreamState::Cancelled;
    co_return StreamStatus{s, amt};
  }

  Task<bool> write_one() {
    auto st = co_await write(1);
    co_return st.count > 0;
  }
};

template <typename T = void>
inline std::pair<StreamWriter<T>, StreamReader<T>> stream_new() {
  const auto *vt = detail::get_stream_vtable<T>();
  uint64_t handles = vt->new_stream();
  int32_t reader = static_cast<int32_t>(handles & 0xffffffffULL);
  int32_t writer = static_cast<int32_t>(handles >> 32);
  return {StreamWriter<T>(writer), StreamReader<T>(reader)};
}

class ErrorContext {
public:
  int32_t handle = -1;
  ErrorContext() = default;
  explicit ErrorContext(int32_t h) : handle(h) {}
  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = -1;
    return h;
  }
};
} // namespace wit
