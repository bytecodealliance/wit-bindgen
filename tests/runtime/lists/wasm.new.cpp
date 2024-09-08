#include <assert.h>
#include <lists_cpp.h>

uint32_t exports::lists::AllocatedBytes() {
    return 0;
}

template<class R>
static bool equal(wit::vector<R> const&a, wit::span<R> const& b);
template<class R>
static bool equal(wit::span<const R> const&a, wit::vector<R> const& b);
template<class R>
static bool equal(wit::span<const R> const&a, std::vector<R> const& b);
template<class R>
static bool equal(wit::vector<R> const&a, std::vector<R> const& b);
static bool equal(wit::string const&a, std::string_view b) {
    return a.get_view() == b;
}
template <class T>
static bool equal(T const&a, T const& b) {
    return a == b;
}
static bool equal(wit::vector<wit::string> const&a, std::vector<std::string_view> const& b);
template<class R,class S, class T, class U>
static bool equal(std::tuple<R,S> const&a, std::tuple<T,U> const& b);

void exports::lists::TestImports() {
    //let _guard = testRust_wasm::guard();

    test::lists::test::EmptyListParam(wit::span<const uint8_t>(std::vector<const uint8_t>()));
    test::lists::test::EmptyStringParam("");
    assert(test::lists::test::EmptyListResult().empty());
    assert(test::lists::test::EmptyStringResult().empty());

    test::lists::test::ListParam(std::vector<uint8_t>{1, 2, 3, 4});
    test::lists::test::ListParam2("foo");
    test::lists::test::ListParam3(std::vector<std::string_view>{"foo", "bar", "baz"});
    test::lists::test::ListParam4(std::vector<wit::span<std::string_view>>{
        std::vector<std::string_view>{"foo", "bar"},
        std::vector<std::string_view>{"baz"},
    });
    assert(equal(test::lists::test::ListResult(), std::vector<uint8_t>{1, 2, 3, 4, 5}));
    assert(equal(test::lists::test::ListResult2(), "hello!"));
    assert(equal(test::lists::test::ListResult3(), std::vector<std::string_view>{"hello,", "world!"}));

    assert(equal(test::lists::test::ListRoundtrip(wit::span<const uint8_t>(std::vector<const uint8_t>())), std::vector<uint8_t>()));
    assert(equal(test::lists::test::ListRoundtrip(wit::span<const uint8_t>(std::vector<const uint8_t>{'x'})), std::vector<uint8_t>{'x'}));
    assert(equal(test::lists::test::ListRoundtrip(wit::span<const uint8_t>(std::vector<const uint8_t>{'h', 'e', 'l', 'l', 'o'})), std::vector<uint8_t>{'h', 'e', 'l', 'l', 'o'}));

    assert(equal(test::lists::test::StringRoundtrip("x"), "x"));
    assert(equal(test::lists::test::StringRoundtrip(""), ""));
    assert(equal(test::lists::test::StringRoundtrip("hello"), "hello"));
    assert(equal(test::lists::test::StringRoundtrip("hello ⚑ world"), "hello ⚑ world"));

    assert(equal(
        test::lists::test::ListMinmax8(std::vector<uint8_t>{0, UINT8_MAX}, std::vector<int8_t>{INT8_MIN, INT8_MAX}),
        std::make_tuple(std::vector<uint8_t>{0, UINT8_MAX}, std::vector<int8_t>{INT8_MIN, INT8_MAX})
    ));
    // assert(equal(
    //     test::lists::test::ListMinmax16(&[u16::MIN, u16::MAX], &[i16::MIN, i16::MAX]),
    //     (vec![u16::MIN, u16::MAX], vec![i16::MIN, i16::MAX]),
    // ));
    // assert(equal(
    //     test::lists::test::ListMinmax32(&[u32::MIN, u32::MAX], &[i32::MIN, i32::MAX]),
    //     (vec![u32::MIN, u32::MAX], vec![i32::MIN, i32::MAX]),
    // ));
    // assert(equal(
    //     test::lists::test::ListMinmax64(&[u64::MIN, u64::MAX], &[i64::MIN, i64::MAX]),
    //     (vec![u64::MIN, u64::MAX], vec![i64::MIN, i64::MAX]),
    // ));
    // assert(equal(
    //     test::lists::test::ListMinmaxFloat(
    //         &[f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
    //         &[f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY]
    //     ),
    //     (
    //         vec![f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
    //         vec![f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY],
    //     ),
    // ));
}


void exports::test::lists::test::EmptyListParam(wit::span<uint8_t const> a) {
    assert(a.empty());
}

void exports::test::lists::test::EmptyStringParam(std::string_view a) {
    assert(a.empty());
}

wit::vector<uint8_t> exports::test::lists::test::EmptyListResult() {
    return wit::vector<uint8_t>();
}

wit::string exports::test::lists::test::EmptyStringResult() {
    return wit::string::from_view(std::string_view());
}

void exports::test::lists::test::ListParam(wit::span<const uint8_t> list) {
    assert(equal(list, std::vector<uint8_t>{1, 2, 3, 4}));
}

void exports::test::lists::test::ListParam2(std::string_view ptr) {
    assert(equal(ptr, std::string_view("foo")));
}

void exports::test::lists::test::ListParam3(wit::span<const std::string_view> ptr) {
    assert(equal(ptr.size(), size_t(3)));
    assert(equal(ptr[0], std::string_view("foo")));
    assert(equal(ptr[1], std::string_view("bar")));
    assert(equal(ptr[2], std::string_view("baz")));
}

void exports::test::lists::test::ListParam4(wit::span<const wit::span<const std::string_view>> ptr) {
    assert(equal(ptr.size(), size_t(2)));
    assert(equal(ptr[0][0], std::string_view("foo")));
    assert(equal(ptr[0][1], std::string_view("bar")));
    assert(equal(ptr[1][0], std::string_view("baz")));
}

wit::vector<uint8_t> exports::test::lists::test::ListResult() {
    return wit::vector<uint8_t>::from_view(wit::span<uint8_t>(std::vector<uint8_t>{1, 2, 3, 4, 5}));
}

wit::string exports::test::lists::test::ListResult2() {
    return wit::string::from_view("hello!");
}

wit::vector<wit::string> exports::test::lists::test::ListResult3() {
    return wit::vector<wit::string>::from_view(wit::span<wit::string>(std::vector<wit::string>{wit::string::from_view("hello,"), wit::string::from_view("world!")}));
}

wit::vector<uint8_t> exports::test::lists::test::ListRoundtrip(wit::span<const uint8_t> x) {
    return wit::vector<uint8_t>::from_view(x);
}

wit::string exports::test::lists::test::StringRoundtrip(std::string_view x) {
    return wit::string::from_view(x);
}

std::tuple<wit::vector<uint8_t>, wit::vector<int8_t>> exports::test::lists::test::ListMinmax8(wit::span<uint8_t const> a, wit::span<int8_t const> b) {
    return std::make_tuple(wit::vector<uint8_t>::from_view(a), wit::vector<int8_t>::from_view(b));
}

std::tuple<wit::vector<uint16_t>, wit::vector<int16_t>> exports::test::lists::test::ListMinmax16(wit::span<uint16_t const> a, wit::span<int16_t const> b) {
    return std::make_tuple(wit::vector<uint16_t>::from_view(a), wit::vector<int16_t>::from_view(b));
}

std::tuple<wit::vector<uint32_t>, wit::vector<int32_t>> exports::test::lists::test::ListMinmax32(wit::span<uint32_t const> a, wit::span<int32_t const> b) {
    return std::make_tuple(wit::vector<uint32_t>::from_view(a), wit::vector<int32_t>::from_view(b));
}

std::tuple<wit::vector<uint64_t>, wit::vector<int64_t>> exports::test::lists::test::ListMinmax64(wit::span<uint64_t const> a, wit::span<int64_t const> b) {
    return std::make_tuple(wit::vector<uint64_t>::from_view(a), wit::vector<int64_t>::from_view(b));
}

std::tuple<wit::vector<float>, wit::vector<double>> exports::test::lists::test::ListMinmaxFloat(wit::span<float const> a, wit::span<double const> b) {
    return std::make_tuple(wit::vector<float>::from_view(a), wit::vector<double>::from_view(b));
}
