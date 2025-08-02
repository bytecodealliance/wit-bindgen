//@ args = '--ownership owning'

#include "runner_cpp.h"
#include <array>
int main() {
  std::array<std::string_view, 2> a1 = {"value1", "value2"};
  std::array<std::string_view, 2> a2 = {"value3", "value4"};
  std::array<std::span<std::string_view const>, 2> as = {
      std::span<std::string_view const>(a1.data(), a1.size()),
      std::span<std::string_view const>(a2.data(), a2.size())};
  std::span<std::span<std::string_view const> const> input(as.data(),
                                                           as.size());
  auto res = test::ownership::Foo(input);
  assert(res.size() == 2);
  assert(res[0].size() == 2);
  assert(res[0][0].get_view() == "VALUE1");
  assert(res[0][1].get_view() == "VALUE2");
  assert(res[1].size() == 2);
  assert(res[1][0].get_view() == "VALUE3");
  assert(res[1][1].get_view() == "VALUE4");

  test::ownership::Thing thing1 {
    wit::string::from_view("thing"),
        wit::vector<wit::string>::allocate(2)
  };
  thing1.value.initialize(0, wit::string::from_view("value1"));
  thing1.value.initialize(1, wit::string::from_view("value2"));
  test::ownership::Bar(std::move(thing1));

  test::ownership::Thing thing2 {
    wit::string::from_view("thing"),
        wit::vector<wit::string>::allocate(2)
  };
  thing2.value.initialize(0, wit::string::from_view("value1"));
  thing2.value.initialize(1, wit::string::from_view("value2"));
  auto result = test::ownership::Baz(std::move(thing2));
  assert(result.name.get_view() == "THING");
  assert(result.value.size() == 2);
  assert(result.value[0].get_view() == "VALUE1");
  assert(result.value[1].get_view() == "VALUE2");

  auto v1 = wit::vector<wit::string>::allocate(2);
  v1.initialize(0, wit::string::from_view("value1"));
  v1.initialize(1, wit::string::from_view("value2"));
  std::array<std::string_view, 2> v2 = {"value1", "value2"};
  test::ownership::both_list_and_resource::Thing resource_thing {
    std::move(v1),
        test::ownership::both_list_and_resource::TheResource(std::span<std::string_view const>(v2.data(), v2.size()))
  };
  test::ownership::both_list_and_resource::ListAndResource(
      std::move(resource_thing));
}