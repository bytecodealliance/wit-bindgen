#include <runner_cpp.h>

namespace test_imports = ::test::resource_borrow_in_record::to_test;
#include <iostream>
int main() {
    auto thing1 = test_imports::Thing("Bonjour");
    auto thing2 = test_imports::Thing("mon cher");
    std::cout << thing1.Get().to_string() << ' ' << thing1.get_handle() << std::endl;
    std::cout << thing2.Get().to_string() << ' ' << thing2.get_handle() << std::endl;
    std::array things {test_imports::Foo{thing1}, test_imports::Foo{thing2}};
    auto result = test_imports::Test(things);
}
