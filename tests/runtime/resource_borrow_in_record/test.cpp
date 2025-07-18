#include <test_cpp.h>

namespace test_exports = ::exports::test::resource_borrow_in_record::to_test;

wit::vector<test_exports::Thing::Owned> test_exports::Test(wit::vector<test_exports::Foo> list) {
    auto result = wit::vector<test_exports::Thing::Owned>::allocate(list.size()); 
    for (size_t i = 0; i < list.size(); ++i) {
        auto str = wit::string::from_view(std::string_view(list[i].thing.get().Get().to_string() + " test"));
        result.initialize(i, test_exports::Thing::New(std::move(str)));
    }
    return result;
}