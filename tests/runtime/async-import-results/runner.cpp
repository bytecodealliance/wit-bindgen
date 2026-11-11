//@ wasmtime-flags = '-Wcomponent-model-async'

#include <assert.h>
#include <runner_cpp.h>

wit::Task<void> exports::runner::Run() {
    auto echoed = co_await my::test::i::Echo(wit::string::from_view("hi"));
    assert(echoed.get_view() == "hi!");

    auto p = co_await my::test::i::MakePoint(3, 4);
    assert(p.x == 3);
    assert(p.y == 4);
    assert(p.label.get_view() == "(3, 4)");

    auto words_in = wit::vector<wit::string>::allocate(3);
    words_in.initialize(0, wit::string::from_view("a"));
    words_in.initialize(1, wit::string::from_view("bb"));
    words_in.initialize(2, wit::string::from_view("ccc"));

    auto rx = co_await my::test::i::Words(std::move(words_in));
    auto collected = co_await rx.collect();
    assert(collected.size() == 3);
    assert(collected[0].get_view() == "a");
    assert(collected[1].get_view() == "bb");
    assert(collected[2].get_view() == "ccc");
}
