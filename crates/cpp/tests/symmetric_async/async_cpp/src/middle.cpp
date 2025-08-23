#include "async_module_cpp.h"
#include <thread>

std::future<wit::string> exports::test::test::string_delay::Forward(std::string_view s) {
    if (s[0]=='A') {
        std::promise<wit::string> result;
        result.set_value(wit::string::from_view("directly returned"));
        return result.get_future();
    } else if (s[0]=='B') {
        return std::async(std::launch::async, [](){
            auto delay = ::test::test::wait::Sleep(5ull*1000*1000*1000);
            delay.wait();
            return wit::string::from_view("after five seconds");
        });
    } else {
        return std::async(std::launch::async, [](){
            auto delay = ::test::test::wait::Sleep(1*1000*1000*1000);
            delay.wait();
            return wit::string::from_view("after one second");
        });
    }
}
