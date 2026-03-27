#include <assert.h>
#include <leaf_cpp.h>

template <class T>
bool equal(T const&a, T const&b) {
    return a==b;
}

namespace test_exports = ::exports::test::results::test;

std::expected<float, wit::string> test_exports::StringError(float a) {
    if (a==0.0) {
        return std::unexpected(wit::string::from_view("zero"));
    }
    else {
        return a;
    }
}

std::expected<float, test_exports::E> test_exports::EnumError(float a) {
    if (a==0.0) {
        return std::unexpected(test_exports::E::kA);
    }
    else {
        return a;
    }
}

std::expected<float, test_exports::E2> test_exports::RecordError(float a) {
    if (a==0.0) {
        return std::unexpected(test_exports::E2{420, 0});
    }
    else if (a==1.0) {
        return std::unexpected(test_exports::E2{77, 2});
    }
    else {
        return a;
    }
}

std::expected<float, test_exports::E3> test_exports::VariantError(float a) {
    if (a==0.0) {
        return std::unexpected(test_exports::E3{test_exports::E3::E2{test_exports::E2{420, 0}}});
    }
    else if (a==1.0) {
        return std::unexpected(test_exports::E3{test_exports::E3::E1{test_exports::E::kB}});
    }
    else if (a==2.0) {
        return std::unexpected(test_exports::E3{test_exports::E3::E1{test_exports::E::kC}});
    }
    else {
        return a;
    }
}

std::expected<uint32_t, wit::Void> test_exports::EmptyError(uint32_t a) {
    if (a==0) {
        return std::unexpected(wit::Void{});
    }
    else if (a==1) {
        return 42;
    }
    else {
        return a;
    }
}

std::expected<std::expected<void, wit::string>, wit::string> test_exports::DoubleError(uint32_t a) {
    if (a==0) {
        return std::expected<void, wit::string>();
    }
    else if (a==1) {
        return std::expected<void, wit::string>(std::unexpected(wit::string::from_view("one")));
    }
    else {
        return std::unexpected(wit::string::from_view("two"));
    }
}
