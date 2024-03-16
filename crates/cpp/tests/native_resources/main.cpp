
#include "the_world_cpp_native.h"
#include <iostream>

int main() {
    auto obj = exports::foo::foo::resources::Create();
    obj.Add(12);
    exports::foo::foo::resources::Borrows(obj);
    exports::foo::foo::resources::Consume(std::move(obj));
    auto obj2 = exports::foo::foo::resources::R{42};
    return 0;
}
