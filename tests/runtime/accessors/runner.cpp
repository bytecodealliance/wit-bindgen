//@ wasmtime-flags = '-Wcomponent-model-accessors'

#include <assert.h>
#include <limits.h>
#include <vector>
#include <runner_cpp.h>

namespace test_imports = ::test::accessors::i;

static bool is_err(std::expected<void, wit::string> const& r, std::string_view msg) {
    return !r.has_value() && r.error().get_view() == msg;
}

static bool equal(wit::vector<uint8_t> const& a, std::vector<uint8_t> const& b) {
    auto view = a.get_const_view();
    return std::vector<uint8_t>(view.begin(), view.end()) == b;
}

void exports::runner::Run() {
    assert(test_imports::Counter() == 0);
    test_imports::SetCounter(5);
    assert(test_imports::Counter() == 5);
    test_imports::SetCounter(UINT32_MAX);
    assert(test_imports::Counter() == UINT32_MAX);

    assert(test_imports::ReadOnly().get_view() == "read only");

    assert(test_imports::Bounded() == 0);
    assert(test_imports::SetBounded(10).has_value());
    assert(test_imports::Bounded() == 10);
    assert(is_err(test_imports::SetBounded(101), "101 is out of bounds"));
    assert(test_imports::Bounded() == 10);

    std::vector<uint8_t> a_init{1, 2, 3};
    std::vector<uint8_t> b_init{4, 5};
    auto a = test_imports::Blob(std::span<const uint8_t>(a_init));
    auto b = test_imports::Blob(std::span<const uint8_t>(b_init));
    assert(equal(a.Contents(), a_init));
    assert(a.Position() == 0);
    assert(b.Position() == 0);
    a.SetPosition(2);
    assert(a.Position() == 2);
    assert(b.Position() == 0);
    b.SetPosition(1);
    assert(a.Position() == 2);
    assert(b.Position() == 1);

    assert(a.Label().get_view() == "");
    assert(a.SetLabel("hello").has_value());
    assert(a.Label().get_view() == "hello");
    assert(b.Label().get_view() == "");
    assert(is_err(a.SetLabel(""), "label must not be empty"));
    assert(a.Label().get_view() == "hello");

    assert(test_imports::Blob::MaxSize() == 1024);
    assert(test_imports::Blob::SetMaxSize(2048).has_value());
    assert(test_imports::Blob::MaxSize() == 2048);
    assert(is_err(test_imports::Blob::SetMaxSize(0), "max size must be nonzero"));
    assert(test_imports::Blob::MaxSize() == 2048);
}
