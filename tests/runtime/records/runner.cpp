#include <assert.h>
#include <runner_cpp.h>

template <class T>
bool equal(T const&a, T const&b) {
    return a==b;
}

int main()
{
    using namespace ::test::records::to_test;

    assert(equal(MultipleResults(), std::tuple<uint8_t, uint16_t>(4, 5)));

    assert(equal(SwapTuple(std::tuple<uint8_t, uint32_t>(1, 2)), std::tuple<uint32_t, uint8_t>(2, 1)));
    assert(equal(RoundtripFlags1(::test::records::to_test::F1::kA), ::test::records::to_test::F1::kA));
    assert(equal(RoundtripFlags1(::test::records::to_test::F1::k_None), ::test::records::to_test::F1::k_None));
    assert(equal(RoundtripFlags1(::test::records::to_test::F1::kB), ::test::records::to_test::F1::kB));
    assert(equal(RoundtripFlags1(::test::records::to_test::F1::kA | ::test::records::to_test::F1::kB), ::test::records::to_test::F1::kA | ::test::records::to_test::F1::kB));

    assert(equal(RoundtripFlags2(::test::records::to_test::F2::kC), ::test::records::to_test::F2::kC));
    assert(equal(RoundtripFlags2(::test::records::to_test::F2::k_None), ::test::records::to_test::F2::k_None));
    assert(equal(RoundtripFlags2(::test::records::to_test::F2::kD), ::test::records::to_test::F2::kD));
    assert(equal(RoundtripFlags2(::test::records::to_test::F2::kC | ::test::records::to_test::F2::kE), ::test::records::to_test::F2::kC | ::test::records::to_test::F2::kE));

    assert(equal(
        RoundtripFlags3(::test::records::to_test::Flag8::kB0, ::test::records::to_test::Flag16::kB1, ::test::records::to_test::Flag32::kB2),
        std::tuple<::test::records::to_test::Flag8, ::test::records::to_test::Flag16, ::test::records::to_test::Flag32>(::test::records::to_test::Flag8::kB0, ::test::records::to_test::Flag16::kB1, ::test::records::to_test::Flag32::kB2)
    ));

    {
        auto r = RoundtripRecord1(::test::records::to_test::R1 {
            8,
            ::test::records::to_test::F1::k_None,
        });
        assert(equal(r.a, (uint8_t)8));
        assert(equal(r.b, ::test::records::to_test::F1::k_None));
    }

    auto r = RoundtripRecord1(::test::records::to_test::R1 {
        0,
        ::test::records::to_test::F1::kA | ::test::records::to_test::F1::kB,
    });
    assert(equal(r.a, (uint8_t)0));
    assert(equal(r.b, ::test::records::to_test::F1::kA | ::test::records::to_test::F1::kB));

    assert(equal(Tuple1(std::tuple<uint8_t>(1)), std::tuple<uint8_t>(1)));
}
