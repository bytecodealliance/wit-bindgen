#include <malloc.h>
#include <stdint.h>
#include <string_view>

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
};
} // namespace wit
