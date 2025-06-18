# include <test_cpp.h>

std::future<wit::string> exports::a::b::the_test::F() {
    std::promise<wit::string> result;
    result.set_value(wit::string::from_view("Hello"));
    return result.get_future();
}
