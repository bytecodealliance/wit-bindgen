#include "future_world_cpp.h"
#include <thread>

std::future<uint32_t> exports::test::test::future_test::Create() {
    return std::async(std::launch::async, [](){
        auto value = ::test::test::future_source::Create();
        return value.get() * 2;
    });    
}
