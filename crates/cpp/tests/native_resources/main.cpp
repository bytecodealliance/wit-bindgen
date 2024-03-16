
#include "the_world_cpp_native.h"
#include <iostream>

foo::foo::resources::R::Owned foo::foo::resources::Create() { abort();}
void foo::foo::resources::Borrows(std::reference_wrapper<R const>) { abort(); }
void foo::foo::resources::Consume(R::Owned o) { abort(); }

int main() {
    auto obj = exports::foo::foo::resources::Create();
    obj.Add(12);
    exports::foo::foo::resources::Borrows(obj);
    exports::foo::foo::resources::Consume(std::move(obj));
    auto obj2 = exports::foo::foo::resources::R{42};
    return 0;
}
