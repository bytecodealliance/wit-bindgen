#include <assert.h>
#include <runner_cpp.h>

template <class T>
static bool equal(T const& a, T const& b) {
    return a == b;
}

template <class T>
static bool equal(wit::vector<T> const& a, wit::vector<T> const& b) {
    if (a.size() != b.size()) return false;
    for (size_t i = 0; i < a.size(); i++) {
        if (!equal(a[i], b[i])) return false;
    }
    return true;
}

static bool equal(wit::string const& a, wit::string const& b) {
    return a.get_view() == b.get_view();
}

static bool equal(::test::variant_with_data::to_test::DataVariant const& a, ::test::variant_with_data::to_test::DataVariant const& b) {
    if (a.variants.index() != b.variants.index()) return false;
    switch (a.variants.index()) {
        case 0: return equal(std::get<::test::variant_with_data::to_test::DataVariant::Bytes>(a.variants).value, std::get<::test::variant_with_data::to_test::DataVariant::Bytes>(b.variants).value);
        case 1: return equal(std::get<::test::variant_with_data::to_test::DataVariant::Number>(a.variants).value, std::get<::test::variant_with_data::to_test::DataVariant::Number>(b.variants).value);
        case 2: return equal(std::get<::test::variant_with_data::to_test::DataVariant::Text>(a.variants).value, std::get<::test::variant_with_data::to_test::DataVariant::Text>(b.variants).value);
    }
    return false;
}

int main() {
  using namespace ::test::variant_with_data::to_test;

  // Test bytes variant
  auto bytes_variant = GetData(0);
  uint8_t expected_bytes[]{0x01, 0x02, 0x03, 0x04, 0x05};
  DataVariant expected_bytes_variant;
  expected_bytes_variant.variants = DataVariant::Bytes(wit::vector<uint8_t>::from_view(std::span<uint8_t>(expected_bytes)));
  assert(equal(bytes_variant, expected_bytes_variant));

  // Test number variant
  auto number_variant = GetData(1);
  DataVariant expected_number_variant;
  expected_number_variant.variants = DataVariant::Number(42);
  assert(equal(number_variant, expected_number_variant));

  // Test text variant
  auto text_variant = GetData(2);
  DataVariant expected_text_variant;
  expected_text_variant.variants = DataVariant::Text(wit::string::from_view("hello"));
  assert(equal(text_variant, expected_text_variant));

  return 0;
}
