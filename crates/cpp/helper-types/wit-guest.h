#include <malloc.h>
#include <stdint.h>
#include <string_view>
#include <string>
#include <memory> // unique_ptr
#include "wit-common.h"

namespace wit {
class string {
  uint8_t const *data_;
  size_t length;

public:
  string(string const &) = delete;
  string(string &&b) : data_(b.data_), length(b.length) { b.data_ = nullptr; }
  string &operator=(string const &) = delete;
  string &operator=(string &&b) {
    if (data_) {
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
  ~string() {
    if (data_) {
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
};

template <class T>
class vector {
  T *data_;
  size_t length;

public:
  vector(vector const &) = delete;
  vector(vector &&b) : data_(b.data_), length(b.length) { b.data_ = nullptr; }
  vector &operator=(vector const &) = delete;
  vector &operator=(vector &&b) {
    if (data_) {
      free(const_cast<uint8_t *>(data_));
    }
    data_ = b.data_;
    length = b.length;
    b.data_ = nullptr;
    return *this;
  }
  vector(T *d, size_t l) : data_(d), length(l) {}
  T const *data() const { return data_; }
  T *data() { return data_; }
  T& operator[](size_t n) { return data_[n]; }
  T const& operator[](size_t n) const { return data_[n]; }
  size_t size() const { return length; }
  ~vector() {
    if (data_) {
      free(data_);
    }
  }
  // leak the memory
  void leak() { data_ = nullptr; }
  // typically called by post
  static void drop_raw(void *ptr) { free(ptr); }
  wit::span<T> get_view() const {
    return wit::span<T>(data_, length);
  }
};

template <class R> class ResourceExportBase {
  public:
    struct Deleter {
      void operator()(R* ptr) const { R::Dtor(ptr); }
    };
    typedef std::unique_ptr<R, Deleter> Owned;

    static const int32_t invalid = -1;

    int32_t handle;

    ResourceExportBase() : handle(R::ResourceNew((R*)this)) {}
    ~ResourceExportBase() { if (handle>=0) { R::ResourceDrop(handle); } }
    ResourceExportBase(ResourceExportBase const&) = delete;
    ResourceExportBase(ResourceExportBase &&) = delete;
    ResourceExportBase& operator=(ResourceExportBase &&b) = delete;
    ResourceExportBase& operator=(ResourceExportBase const&) = delete;
    int32_t get_handle() const { return handle; }
    int32_t into_handle() { int32_t result = handle; handle=invalid; return result; }
};

class ResourceImportBase {
public:
  static const int32_t invalid = -1;

protected:
  int32_t handle;

public:
  ResourceImportBase(int32_t h = invalid) : handle(h) {}
  ResourceImportBase(ResourceImportBase &&r) : handle(r.handle) {
    r.handle = invalid;
  }
  ResourceImportBase(ResourceImportBase const &) = delete;
  void set_handle(int32_t h) { handle = h; }
  int32_t get_handle() const { return handle; }
  int32_t into_handle() {
    int32_t h = handle;
    handle = invalid;
    return h;
  }
  ResourceImportBase &operator=(ResourceImportBase &&r) {
    assert(handle < 0);
    handle = r.handle;
    r.handle = invalid;
    return *this;
  }
  ResourceImportBase &operator=(ResourceImportBase const &r) = delete;
};
} // namespace wit
