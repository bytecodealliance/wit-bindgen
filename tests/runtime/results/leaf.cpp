#include <assert.h>
#include <leaf_cpp.h>

template <class T>
bool equal(T const&a, T const&b) {
    return a==b;
}

std::expected<float, wit::string> exports::test::results::test::StringError(float a) {
    if (a==0.0) {
        return std::unexpected(wit::string::from_view("zero"));
    }
    else {
        return a;
    }
}

std::expected<float, ::test::results::test::E> exports::test::results::test::EnumError(float a) {
    if (a==0.0) {
        return std::unexpected(::test::results::test::E::kA);
    }
    else {
        return a;
    }
}

std::expected<float, ::test::results::test::E2> exports::test::results::test::RecordError(float a) {
    if (a==0.0) {
        return std::unexpected(::test::results::test::E2{420, 0});
    }
    else if (a==1.0) {
        return std::unexpected(::test::results::test::E2{77, 2});
    }
    else {
        return a;
    }
}

std::expected<float, ::test::results::test::E3> exports::test::results::test::VariantError(float a) {
    if (a==0.0) {
        return std::unexpected(::test::results::test::E3{::test::results::test::E3::E2{::test::results::test::E2{420, 0}}});
    }
    else if (a==1.0) {
        return std::unexpected(::test::results::test::E3{::test::results::test::E3::E1{::test::results::test::E::kB}});
    }
    else if (a==2.0) {
        return std::unexpected(::test::results::test::E3{::test::results::test::E3::E1{::test::results::test::E::kC}});
    }
    else {
        return a;
    }
}

std::expected<uint32_t, wit::Void> exports::test::results::test::EmptyError(uint32_t a) {
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

std::expected<std::expected<void, wit::string>, wit::string> exports::test::results::test::DoubleError(uint32_t a) {
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
