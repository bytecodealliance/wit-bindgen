package export_wit_world

import (
	"fmt"
	"math"
	test "wit_component/test_numbers_numbers"
)

func Run() {
	assertEqual(test.RoundtripU8(1), 1)
	assertEqual(test.RoundtripU8(0), 0)
	assertEqual(test.RoundtripU8(math.MaxUint8), math.MaxUint8)

	assertEqual(test.RoundtripS8(1), 1)
	assertEqual(test.RoundtripS8(math.MinInt8), math.MinInt8)
	assertEqual(test.RoundtripS8(math.MaxInt8), math.MaxInt8)

	assertEqual(test.RoundtripU16(1), 1)
	assertEqual(test.RoundtripU16(0), 0)
	assertEqual(test.RoundtripU16(math.MaxUint16), math.MaxUint16)

	assertEqual(test.RoundtripS16(1), 1)
	assertEqual(test.RoundtripS16(math.MinInt16), math.MinInt16)
	assertEqual(test.RoundtripS16(math.MaxInt16), math.MaxInt16)

	assertEqual(test.RoundtripU32(1), 1)
	assertEqual(test.RoundtripU32(0), 0)
	assertEqual(test.RoundtripU32(math.MaxUint32), math.MaxUint32)

	assertEqual(test.RoundtripS32(1), 1)
	assertEqual(test.RoundtripS32(math.MinInt32), math.MinInt32)
	assertEqual(test.RoundtripS32(math.MaxInt32), math.MaxInt32)

	assertEqual(test.RoundtripU64(1), 1)
	assertEqual(test.RoundtripU64(0), 0)
	assertEqual(test.RoundtripU64(math.MaxUint64), math.MaxUint64)

	assertEqual(test.RoundtripS64(1), 1)
	assertEqual(test.RoundtripS64(math.MinInt64), math.MinInt64)
	assertEqual(test.RoundtripS64(math.MaxInt64), math.MaxInt64)

	assertEqual(test.RoundtripF32(1.0), 1.0)
	assertEqual(test.RoundtripF32(float32(math.Inf(1))), float32(math.Inf(1)))
	assertEqual(test.RoundtripF32(float32(math.Inf(-1))), float32(math.Inf(-1)))
	assert(math.IsNaN(float64(test.RoundtripF32(float32(math.NaN())))))

	assertEqual(test.RoundtripF64(1.0), 1.0)
	assertEqual(test.RoundtripF64(math.Inf(1)), math.Inf(1))
	assertEqual(test.RoundtripF64(math.Inf(-1)), math.Inf(-1))
	assert(math.IsNaN(test.RoundtripF64(math.NaN())))

	assertEqual(test.RoundtripChar('a'), 'a')
	assertEqual(test.RoundtripChar(' '), ' ')
	assertEqual(test.RoundtripChar('🚩'), '🚩')

	test.SetScalar(2)
	assertEqual(test.GetScalar(), 2)

	test.SetScalar(4)
	assertEqual(test.GetScalar(), 4)
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}
