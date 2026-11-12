#include <array>
#include <span>
#include <test_cpp.h>

wit::Task<std::expected<std::expected<wit::vector<uint8_t>, wit::string>, wit::Void>>
exports::test::async_ns::callbacks::InvokeAsync(::test::async_ns::types::CallbackId cb) {
  if (cb.id == 0) {
    co_return std::unexpected(wit::Void{});
  }
  if (cb.id == 1) {
    co_return std::expected<wit::vector<uint8_t>, wit::string>(
        std::unexpected(wit::string::from_view("bad-id")));
  }
  std::array<uint8_t, 3> data{static_cast<uint8_t>(cb.id), 0xAB, 0xCD};
  co_return std::expected<wit::vector<uint8_t>, wit::string>(
      wit::vector<uint8_t>::from_view(std::span<uint8_t const>(data)));
}
