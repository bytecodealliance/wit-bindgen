package main

import (
	"math"

	. "wit_numbers_go/gen"
)

func init() {
	n := &NumbersImpl{}
	SetNumbers(n)
	SetExportsTestNumbersTest(n)
}

type NumbersImpl struct {
	scalar uint32
}

func (i NumbersImpl) TestImports() {
	if TestNumbersTestRoundtripU8(1) != 1 {
		panic("roundtrip-u8")
	}
	if TestNumbersTestRoundtripU8(0) != 0 {
		panic("roundtrip-u8")
	}
	if TestNumbersTestRoundtripU8(math.MaxUint8) != math.MaxUint8 {
		panic("roundtrip-u8")
	}

	if TestNumbersTestRoundtripS8(1) != 1 {
		panic("roundtrip-s8")
	}
	if TestNumbersTestRoundtripS8(math.MaxInt8) != math.MaxInt8 {
		panic("roundtrip-s8")
	}
	if TestNumbersTestRoundtripS8(math.MinInt8) != math.MinInt8 {
		panic("roundtrip-s8")
	}

	if TestNumbersTestRoundtripU16(1) != 1 {
		panic("roundtrip-u16")
	}
	if TestNumbersTestRoundtripU16(0) != 0 {
		panic("roundtrip-u16")
	}
	if TestNumbersTestRoundtripU16(math.MaxUint16) != math.MaxUint16 {
		panic("roundtrip-u16")
	}

	if TestNumbersTestRoundtripS16(1) != 1 {
		panic("roundtrip-s16")
	}
	if TestNumbersTestRoundtripS16(math.MaxInt16) != math.MaxInt16 {
		panic("roundtrip-s16")
	}
	if TestNumbersTestRoundtripS16(math.MinInt16) != math.MinInt16 {
		panic("roundtrip-s16")
	}

	if TestNumbersTestRoundtripU32(1) != 1 {
		panic("roundtrip-u32")
	}
	if TestNumbersTestRoundtripU32(0) != 0 {
		panic("roundtrip-u32")
	}
	if TestNumbersTestRoundtripU32(math.MaxUint32) != math.MaxUint32 {
		panic("roundtrip-u32")
	}

	if TestNumbersTestRoundtripS32(1) != 1 {
		panic("roundtrip-s32")
	}
	if TestNumbersTestRoundtripS32(math.MaxInt32) != math.MaxInt32 {
		panic("roundtrip-s32")
	}
	if TestNumbersTestRoundtripS32(math.MinInt32) != math.MinInt32 {
		panic("roundtrip-s32")
	}

	if TestNumbersTestRoundtripU64(1) != 1 {
		panic("roundtrip-u64")
	}
	if TestNumbersTestRoundtripU64(0) != 0 {
		panic("roundtrip-u64")
	}
	if TestNumbersTestRoundtripU64(math.MaxUint64) != math.MaxUint64 {
		panic("roundtrip-u64")
	}

	if TestNumbersTestRoundtripS64(1) != 1 {
		panic("roundtrip-s64")
	}
	if TestNumbersTestRoundtripS64(math.MaxInt64) != math.MaxInt64 {
		panic("roundtrip-s64")
	}
	if TestNumbersTestRoundtripS64(math.MinInt64) != math.MinInt64 {
		panic("roundtrip-s64")
	}

	if TestNumbersTestRoundtripFloat32(1.0) != 1.0 {
		panic("roundtrip-float32")
	}
	if TestNumbersTestRoundtripFloat32(math.MaxFloat32) != math.MaxFloat32 {
		panic("roundtrip-float32")
	}
	if TestNumbersTestRoundtripFloat32(math.SmallestNonzeroFloat32) != math.SmallestNonzeroFloat32 {
		panic("roundtrip-float32")
	}

	if TestNumbersTestRoundtripFloat64(1.0) != 1.0 {
		panic("roundtrip-float64")
	}
	if TestNumbersTestRoundtripFloat64(math.MaxFloat64) != math.MaxFloat64 {
		panic("roundtrip-float64")
	}
	if TestNumbersTestRoundtripFloat64(math.SmallestNonzeroFloat64) != math.SmallestNonzeroFloat64 {
		panic("roundtrip-float64")
	}
	if !math.IsNaN(TestNumbersTestRoundtripFloat64(math.NaN())) {
		panic("roundtrip-float64")
	}

	if TestNumbersTestRoundtripChar('a') != 'a' {
		panic("roundtrip-char")
	}
	if TestNumbersTestRoundtripChar(' ') != ' ' {
		panic("roundtrip-char")
	}
	if TestNumbersTestRoundtripChar('🚩') != '🚩' {
		panic("roundtrip-char")
	}

	TestNumbersTestSetScalar(2)
	if TestNumbersTestGetScalar() != 2 {
		panic("get-scalar")
	}

	TestNumbersTestSetScalar(4)
	if TestNumbersTestGetScalar() != 4 {
		panic("get-scalar")
	}
}

func (o *NumbersImpl) RoundtripU8(a uint8) uint8 {
	return a
}

func (o *NumbersImpl) RoundtripS8(a int8) int8 {
	return a
}

func (o *NumbersImpl) RoundtripU16(a uint16) uint16 {
	return a
}

func (o *NumbersImpl) RoundtripS16(a int16) int16 {
	return a
}

func (o *NumbersImpl) RoundtripU32(a uint32) uint32 {
	return a
}

func (o *NumbersImpl) RoundtripS32(a int32) int32 {
	return a
}

func (o *NumbersImpl) RoundtripU64(a uint64) uint64 {
	return a
}

func (o *NumbersImpl) RoundtripS64(a int64) int64 {
	return a
}

func (o *NumbersImpl) RoundtripFloat32(a float32) float32 {
	return a
}

func (o *NumbersImpl) RoundtripFloat64(a float64) float64 {
	return a
}

func (o *NumbersImpl) RoundtripChar(a rune) rune {
	return a
}

func (o *NumbersImpl) SetScalar(a uint32) {
	o.scalar = a
}

func (o *NumbersImpl) GetScalar() uint32 {
	return o.scalar
}

func assert_eq(a, b interface{}) {
	if a != b {
		panic("assertion failed")
	}
}

func main() {}
