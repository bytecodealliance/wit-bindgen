#include <assert.h>
#include <limits.h>
#include <float.h>
#include <runner_cpp.h>

static bool equal(wit::string const&a, std::string_view b) {
    return a.get_view() == b;
}
static bool equal(wit::string const&a, const char x[]) {
    return a.get_view() == x;
}
template <class T, class S>
static bool equal(T const&a, S const& b) {
    return a == b;
}
template<class R, class S>
static bool equal(wit::span<R> const&a, wit::span<S> const& b) {
    if (a.size() != b.size()) { return false; }
    for (uint32_t i = 0; i<a.size(); ++i) {
        if (!equal(a[i], b[i])) { return false; }
    }
    return true;
}
template<class R>
static bool equal(wit::vector<R> const&a, wit::span<R> const& b) {
    return equal(a.get_view(), b);
}
template<class R>
static bool equal(wit::span<const R> const&a, wit::vector<R> const& b) {
    return equal(b, a);
}
template<class R>
static bool equal(wit::span<const R> const&a, std::vector<R> const& b) {
    return equal(a, wit::span<R>(b));
}
template<class R>
static bool equal(wit::vector<R> const&a, std::vector<R> const& b) {
    return equal(a.get_view(), wit::span<R>(b));
}
static bool equal(wit::vector<wit::string> const&a, std::vector<std::string_view> const& b) {
    return equal(a.get_view(), wit::span<std::string_view>(b));
}
template<class R,class S, class T, class U>
static bool equal(std::tuple<R,S> const&a, std::tuple<T,U> const& b) {
    return equal(std::get<0>(a), std::get<0>(b)) && equal(std::get<1>(a), std::get<1>(b));
}

template <class T>
static bool equal(T const& a, T const& b) {
    return a==b;
}

int main()
{
    using namespace ::test::lists::to_test;

    EmptyListParam(wit::span<const uint8_t>(std::vector<uint8_t>()));
    EmptyStringParam("");
    assert(EmptyListResult().empty());
    assert(EmptyStringResult().empty());

    ListParam(std::vector<uint8_t>{1, 2, 3, 4});
    ListParam2("foo");
    ListParam3(std::vector<std::string_view>{"foo", "bar", "baz"});
    ListParam4(std::vector<wit::span<const std::string_view>>{
        std::vector<std::string_view>{"foo", "bar"},
        std::vector<std::string_view>{"baz"},
    });
    assert(equal(ListResult(), std::vector<uint8_t>{1, 2, 3, 4, 5}));
    assert(equal(ListResult2(), "hello!"));
    assert(equal(ListResult3(), std::vector<std::string_view>{"hello,", "world!"}));

    assert(equal(ListRoundtrip(wit::span<const uint8_t>(std::vector<uint8_t>())), std::vector<uint8_t>()));
    assert(equal(ListRoundtrip(wit::span<const uint8_t>(std::vector<uint8_t>{'x'})), std::vector<uint8_t>{'x'}));
    assert(equal(ListRoundtrip(wit::span<const uint8_t>(std::vector<uint8_t>{'h', 'e', 'l', 'l', 'o'})), std::vector<uint8_t>{'h', 'e', 'l', 'l', 'o'}));

    assert(equal(StringRoundtrip("x"), "x"));
    assert(equal(StringRoundtrip(""), ""));
    assert(equal(StringRoundtrip("hello"), "hello"));
    assert(equal(StringRoundtrip("hello ⚑ world"), "hello ⚑ world"));

    assert(equal(
        ListMinmax8(std::vector<uint8_t>{0, UINT8_MAX}, std::vector<int8_t>{INT8_MIN, INT8_MAX}),
        std::make_tuple(std::vector<uint8_t>{0, UINT8_MAX}, std::vector<int8_t>{INT8_MIN, INT8_MAX})
    ));
    assert(equal(
        ListMinmax16(std::vector<uint16_t>{0, UINT16_MAX}, std::vector<int16_t>{INT16_MIN, INT16_MAX}),
        std::make_tuple(std::vector<uint16_t>{0, UINT16_MAX}, std::vector<int16_t>{INT16_MIN, INT16_MAX})
    ));
    assert(equal(
        ListMinmax32(std::vector<uint32_t>{0, UINT32_MAX}, std::vector<int32_t>{INT32_MIN, INT32_MAX}),
        std::make_tuple(std::vector<uint32_t>{0, UINT32_MAX}, std::vector<int32_t>{INT32_MIN, INT32_MAX})
    ));
    assert(equal(
        ListMinmax64(std::vector<uint64_t>{0, UINT64_MAX}, std::vector<int64_t>{INT64_MIN, INT64_MAX}),
        std::make_tuple(std::vector<uint64_t>{0, UINT64_MAX}, std::vector<int64_t>{INT64_MIN, INT64_MAX})
    ));
    assert(equal(
        ListMinmaxFloat(
            std::vector<float>{FLT_MIN, FLT_MAX, -HUGE_VALF, HUGE_VALF},
            std::vector<double>{DBL_MIN, DBL_MAX, -HUGE_VAL, HUGE_VAL}
        ),
        std::make_tuple(
            std::vector<float>{FLT_MIN, FLT_MAX, -HUGE_VALF, HUGE_VALF},
            std::vector<double>{DBL_MIN, DBL_MAX, -HUGE_VAL, HUGE_VAL}
        )
    ));
}
