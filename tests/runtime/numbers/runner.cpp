#include <assert.h>
#include <limits.h>
#include <math.h>
#include <runner_cpp.h>

int main()
{
    using namespace ::test::numbers::numbers;

    assert(RoundtripU8(1) == 1);
    assert(RoundtripU8(0) == 0);
    assert(RoundtripU8(UCHAR_MAX) == UCHAR_MAX);

    assert(RoundtripS8(1) == 1);
    assert(RoundtripS8(SCHAR_MIN) == SCHAR_MIN);
    assert(RoundtripS8(SCHAR_MAX) == SCHAR_MAX);

    assert(RoundtripU16(1) == 1);
    assert(RoundtripU16(0) == 0);
    assert(RoundtripU16(USHRT_MAX) == USHRT_MAX);

    assert(RoundtripS16(1) == 1);
    assert(RoundtripS16(SHRT_MIN) == SHRT_MIN);
    assert(RoundtripS16(SHRT_MAX) == SHRT_MAX);

    assert(RoundtripU32(1) == 1);
    assert(RoundtripU32(0) == 0);
    assert(RoundtripU32(UINT_MAX) == UINT_MAX);

    assert(RoundtripS32(1) == 1);
    assert(RoundtripS32(INT_MIN) == INT_MIN);
    assert(RoundtripS32(INT_MAX) == INT_MAX);

    assert(RoundtripU64(1) == 1);
    assert(RoundtripU64(0) == 0);
    assert(RoundtripU64(ULONG_MAX) == ULONG_MAX);

    assert(RoundtripS64(1) == 1);
    assert(RoundtripS64(LONG_MIN) == LONG_MIN);
    assert(RoundtripS64(LONG_MAX) == LONG_MAX);

    assert(RoundtripF32(1.0) == 1.0);
    assert(RoundtripF32(INFINITY) == INFINITY);
    assert(RoundtripF32(-INFINITY) == -INFINITY);
    assert(isnan(RoundtripF32(NAN)));

    assert(RoundtripF64(1.0) == 1.0);
    assert(RoundtripF64(INFINITY) == INFINITY);
    assert(RoundtripF64(-INFINITY) == -INFINITY);
    assert(isnan(RoundtripF64(NAN)));

    assert(RoundtripChar('a') == 'a');
    assert(RoundtripChar(' ') == ' ');
    assert(RoundtripChar(U'🚩') == U'🚩');

    SetScalar(2);
    assert(GetScalar() == 2);
    SetScalar(4);
    assert(GetScalar() == 4);
}
