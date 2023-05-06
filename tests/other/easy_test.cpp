#include "my_world_cpp.h"

int main() {
    test::example::my_interface::MyObject o(42);
    o.Set(o.Get()+1);
    return 0;
}
