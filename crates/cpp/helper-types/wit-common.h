#pragma once

#include <assert.h>
#include <stdint.h>
#include <map>
#if __cplusplus > 202001L
#include <span>
#endif

namespace wit {
#if __cplusplus > 202001L
using std::span;
#else
// minimal implementation to get things going
template <class T> class span {
  T const *address;
  size_t length;

public:
  T const *data() const { return address; }
  size_t size() const { return length; }

  typedef T const *const_iterator;

  const_iterator begin() const { return address; }
  const_iterator end() const { return address + length; }
  T const &operator[](size_t index) { return address[index]; }
};
#endif

class ResourceImportBase {
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

template <typename T> struct Owned {
  T *ptr;
};
} // namespace wit
