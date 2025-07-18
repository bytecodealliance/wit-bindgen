#include "test_cpp.h"
#include <algorithm>
#include <cassert>
#include <string_view>
namespace exports::test::ownership {
wit::vector<wit::vector<wit::string>>
Foo(wit::vector<wit::vector<wit::string>> a) {
  for (size_t i = 0; i < a.size(); ++i) {
    for (size_t j = 0; j < a[i].size(); ++j) {
      for (char &c : a[i][j]) {
        c = std::toupper(c);
      }
    }
  }
  return a;
}

void Bar(Thing a) {
  assert(a.name.get_view() == "thing");
  assert(a.value.size() == 2);
  assert(a.value[0].get_view() == "value1");
  assert(a.value[1].get_view() == "value2");
}
Thing Baz(Thing a) {
  for (char &c : a.name) {
    c = std::toupper(c);
  }
  for (size_t i = 0; i < a.value.size(); ++i) {
    for (char &c : a.value[i]) {
      c = std::toupper(c);
    }
  }
  return a;
}
namespace both_list_and_resource {
void ListAndResource(Thing a) {
  auto upper = a.b->ToUpper();
  assert(upper.size() == a.a.size());
  for (size_t i = 0; i < a.a.size(); ++i) {
    auto v1 = a.a[i].get_view();
    auto v2 = upper[i].get_view();
    assert(std::equal(v1.begin(), v1.end(), v2.begin(), v2.end(),
                      [](char c1, char c2) { return std::toupper(c1) == c2; }));
  }
}
} // namespace both_list_and_resource
} // namespace exports::test::ownership