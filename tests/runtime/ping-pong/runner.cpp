//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    auto [tx1, rx1] = wit::future_new<wit::string>();
    auto f1 = my::test::i::Ping(std::move(rx1), wit::string::from_view("world"));
    auto f2 = tx1.write(wit::string::from_view("hello"));
    auto rx2 = co_await f1;
    assert(co_await f2);
    auto m2 = co_await rx2.read();
    assert(m2.get_view() == "helloworld");

    auto [tx2, rx3] = wit::future_new<wit::string>();
    auto f3 = my::test::i::Pong(std::move(rx3));
    auto f4 = tx2.write(std::move(m2));
    auto m3 = co_await f3;
    assert(co_await f4);
    assert(m3.get_view() == "helloworld");
}
