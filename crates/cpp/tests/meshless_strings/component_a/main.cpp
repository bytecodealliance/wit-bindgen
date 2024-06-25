
#include "the_world_cpp.h"
#include <iostream>

void comp_a::exports::foo::foo::strings::A(wit::string &&x) {
  std::cout << x.get_view() << std::endl;
}
wit::string comp_a::exports::foo::foo::strings::B() {
  wit::string b = wit::string::from_view(std::string_view("hello B"));
  return b;
}
wit::string comp_a::exports::foo::foo::strings::C(wit::string &&a,
                                                  wit::string &&b) {
  std::cout << a.get_view() << '|' << b.get_view() << std::endl;
  wit::string c = wit::string::from_view(std::string_view("hello C"));
  return c;
}

int main() {
  comp_a::foo::foo::strings::A(std::string_view("hello A"));

  {
    auto b = comp_a::foo::foo::strings::B();
    std::cout << b.get_view() << std::endl;
    // make sure that b's result is destructed before calling C
  }

  auto c = comp_a::foo::foo::strings::C(std::string_view("hello C1"),
                                        std::string_view("hello C2"));
  std::cout << c.get_view() << std::endl;
  return 0;
}
