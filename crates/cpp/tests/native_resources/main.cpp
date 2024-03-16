
#include "the_world_cpp_native.h"
#include <iostream>

int main() {
    auto obj = foo::foo::resources::Create();
    obj.Add(12);
    foo::foo::resources::Borrows(obj);
    foo::foo::resources::Consume(std::move(obj));
    auto obj2 = foo::foo::resources::R{42};
    return 0;
}
