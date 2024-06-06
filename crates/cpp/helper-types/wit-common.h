#pragma once

#include <assert.h>
#include <map>
#include <stdint.h>
#include <stddef.h> // size_t
#if __cplusplus > 202001L
#include <span>
#else
#include <vector>
#endif

namespace wit {
#if __cplusplus > 202001L
using std::span;
#else
/// Minimal span (vector view) implementation for older C++ environments
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
  // create from any compatible vector (borrows data!)
  template <class U>
  span(std::vector<U> const &vec) : address(vec.data()), length(vec.size()) {}
};
#endif
} // namespace wit
