#include <assert.h>
#include <test_cpp.h>
#include <float.h>
#include <math.h>

uint32_t exports::test::lists::to_test::AllocatedBytes() {
    return 0;
}

template <class T, class S>
static bool equal(T const&a, S const& b) {
    return a == b;
}
template<class R, class S>
static bool equal(std::span<R> const&a, std::span<S> const& b) {
    if (a.size() != b.size()) { return false; }
    for (uint32_t i = 0; i<a.size(); ++i) {
        if (!equal(a[i], b[i])) { return false; }
    }
    return true;
}
template<class R>
static bool equal(wit::vector<R> const&a, std::span<R> const& b) {
    return equal(a.get_view(), b);
}
template<class R>
static bool equal(std::span<const R> const&a, wit::vector<R> const& b) {
    return equal(b, a);
}
template<class R>
static bool equal(std::span<const R> const&a, std::vector<R> const& b) {
    return equal(a, std::span<const R>(b));
}
template<class R>
static bool equal(wit::vector<R> const&a, std::vector<R> const& b) {
    return equal(a.get_view(), std::span<R>(b));
}
template<class R,class S, class T, class U>
static bool equal(std::tuple<R,S> const&a, std::tuple<T,U> const& b) {
    return equal(std::get<0>(a), std::get<0>(b)) && equal(std::get<1>(a), std::get<1>(b));
}

static bool equal(wit::string const& a, std::string_view b) {
    return a.get_view() == b;
}

void exports::test::lists::to_test::EmptyListParam(wit::vector<uint8_t> a) {
    assert(a.empty());
}

void exports::test::lists::to_test::EmptyStringParam(wit::string a) {
    assert(a.empty());
}

wit::vector<uint8_t> exports::test::lists::to_test::EmptyListResult() {
    return wit::vector<uint8_t>();
}

wit::string exports::test::lists::to_test::EmptyStringResult() {
    return wit::string::from_view(std::string_view());
}

void exports::test::lists::to_test::ListParam(wit::vector<uint8_t> list) {
    assert(equal(list, std::vector<uint8_t>{1, 2, 3, 4}));
}

void exports::test::lists::to_test::ListParam2(wit::string ptr) {
    assert(equal(ptr, std::string_view("foo")));
}

void exports::test::lists::to_test::ListParam3(wit::vector<wit::string> ptr) {
    assert(equal(ptr.size(), size_t(3)));
    assert(equal(ptr[0], std::string_view("foo")));
    assert(equal(ptr[1], std::string_view("bar")));
    assert(equal(ptr[2], std::string_view("baz")));
}

void exports::test::lists::to_test::ListParam4(wit::vector<wit::vector<wit::string>> ptr) {
    assert(equal(ptr.size(), size_t(2)));
    assert(equal(ptr[0][0], std::string_view("foo")));
    assert(equal(ptr[0][1], std::string_view("bar")));
    assert(equal(ptr[1][0], std::string_view("baz")));
}


void exports::test::lists::to_test::ListParam5(wit::vector<std::tuple<uint8_t, uint32_t, uint8_t>> a) {

}

void exports::test::lists::to_test::ListParamLarge(wit::vector<wit::string> a) {

}

wit::vector<uint8_t> exports::test::lists::to_test::ListResult() {
    return wit::vector<uint8_t>::from_view(std::span<uint8_t const>(std::vector<uint8_t>{1, 2, 3, 4, 5}));
}

wit::string exports::test::lists::to_test::ListResult2() {
    return wit::string::from_view("hello!");
}

wit::vector<wit::string> exports::test::lists::to_test::ListResult3() {
    return wit::vector<wit::string>::from_view(std::span<wit::string const>(std::vector<wit::string>{wit::string::from_view("hello,"), wit::string::from_view("world!")}));
}

wit::vector<uint8_t> exports::test::lists::to_test::ListRoundtrip(wit::vector<uint8_t> x) {
    return x;
}

wit::string exports::test::lists::to_test::StringRoundtrip(wit::string x) {
    return x;
}

std::tuple<wit::vector<uint8_t>, wit::vector<int8_t>> exports::test::lists::to_test::ListMinmax8(wit::vector<uint8_t> a, wit::vector<int8_t> b) {
    return std::make_tuple(std::move(a), std::move(b));
}

std::tuple<wit::vector<uint16_t>, wit::vector<int16_t>> exports::test::lists::to_test::ListMinmax16(wit::vector<uint16_t> a, wit::vector<int16_t> b) {
    return std::make_tuple(std::move(a), std::move(b));
}

std::tuple<wit::vector<uint32_t>, wit::vector<int32_t>> exports::test::lists::to_test::ListMinmax32(wit::vector<uint32_t> a, wit::vector<int32_t> b) {
    return std::make_tuple(std::move(a), std::move(b));
}

std::tuple<wit::vector<uint64_t>, wit::vector<int64_t>> exports::test::lists::to_test::ListMinmax64(wit::vector<uint64_t> a, wit::vector<int64_t> b) {
    return std::make_tuple(std::move(a), std::move(b));
}

std::tuple<wit::vector<float>, wit::vector<double>> exports::test::lists::to_test::ListMinmaxFloat(wit::vector<float> a, wit::vector<double> b) {
    return std::make_tuple(std::move(a), std::move(b));
}
