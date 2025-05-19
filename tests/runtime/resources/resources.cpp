#include <resources_cpp.h>

exports::test::resources::Z::Owned exports::test::resources::Add(std::reference_wrapper<const Z> a, std::reference_wrapper<const Z> b) {
    return exports::test::resources::Z::New(a.get().GetA() + b.get().GetA());
}

void exports::test::resources::Consume(X::Owned x) {

}

std::expected<void, wit::string> exports::test::resources::TestImports() {
    auto y = ::test::resources::Y(10);
    return std::expected<void, wit::string>();
}
