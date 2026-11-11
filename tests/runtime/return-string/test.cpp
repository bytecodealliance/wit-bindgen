#include <test_cpp.h>

wit::Task<wit::string> exports::my::test::i::ReturnString() {
    co_return wit::string::from_view("hello");
}
