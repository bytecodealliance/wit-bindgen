#include <assert.h>
#include <vector>
#include <test_cpp.h>

using exports::test::variant_with_data::to_test::DataVariant;

DataVariant exports::test::variant_with_data::to_test::GetData(uint8_t num) {
  DataVariant variant;
  switch (num) {
    case 0: {
      uint8_t bytes[]{0x01, 0x02, 0x03, 0x04, 0x05};
      variant.variants = DataVariant::Bytes(wit::vector<uint8_t>::from_view(std::span<uint8_t>(bytes)));
      break;
    }
    case 1:
      variant.variants = DataVariant::Number(42);
      break;
    case 2:
      variant.variants = DataVariant::Text(wit::string::from_view("hello"));
      break;
    default:
      variant.variants = DataVariant::Number(0);
      break;
  }
  auto result = std::move(variant);
  return result;
}
