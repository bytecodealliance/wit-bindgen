#include <resources_cpp.h>

template <class T>
static bool equal(T const& a, T const& b) {
    return a==b;
}

exports::test::resources::Z::Owned exports::test::resources::Add(std::reference_wrapper<const Z> a, std::reference_wrapper<const Z> b) {
    return exports::test::resources::Z::New(a.get().GetA() + b.get().GetA());
}

void exports::test::resources::Consume(X::Owned x) {

}

std::expected<void, wit::string> exports::test::resources::TestImports() {
    auto y = ::test::resources::Y(10);
    assert(equal(y.GetA(), 10));
    y.SetA(20);
    assert(equal(y.GetA(), 20));
    auto y2a = ::test::resources::Y::Add(std::move(y), 20);
    assert(equal(y2a.GetA(), 40));

    // test multiple instances
    auto y1 = ::test::resources::Y(1);
    auto y2 = ::test::resources::Y(2);
    assert(equal(y1.GetA(), 1));
    assert(equal(y2.GetA(), 2));
    y1.SetA(10);
    y2.SetA(20);
    assert(equal(y1.GetA(), 10));
    assert(equal(y2.GetA(), 20));
    auto y3 = ::test::resources::Y::Add(std::move(y1), 20);
    auto y4 = ::test::resources::Y::Add(std::move(y2), 30);
    assert(equal(y3.GetA(), 30));
    assert(equal(y4.GetA(), 50));
    return std::expected<void, wit::string>();
}

uint32_t exports::test::resources::Z::num_dropped = 0;
