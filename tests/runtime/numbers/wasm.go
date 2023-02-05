package main

import (
	"math"

	. "wit_numbers_go/gen"
)

func init() {
	n := &NumbersImpl{}
	SetNumbers(n)
	SetExports(n)
}

type NumbersImpl struct {
	scalar uint32
}

func (i NumbersImpl) TestImports() {
	if ImportsRoundtripU8(1) != 1 {
		panic("roundtrip-u8")
	}
	if ImportsRoundtripU8(0) != 0 {
		panic("roundtrip-u8")
	}
	if ImportsRoundtripU8(math.MaxUint8) != math.MaxUint8 {
		panic("roundtrip-u8")
	}

	if ImportsRoundtripS8(1) != 1 {
		panic("roundtrip-s8")
	}
	if ImportsRoundtripS8(math.MaxInt8) != math.MaxInt8 {
		panic("roundtrip-s8")
	}
	if ImportsRoundtripS8(math.MinInt8) != math.MinInt8 {
		panic("roundtrip-s8")
	}

	if ImportsRoundtripU16(1) != 1 {
		panic("roundtrip-u16")
	}
	if ImportsRoundtripU16(0) != 0 {
		panic("roundtrip-u16")
	}
	if ImportsRoundtripU16(math.MaxUint16) != math.MaxUint16 {
		panic("roundtrip-u16")
	}

	if ImportsRoundtripS16(1) != 1 {
		panic("roundtrip-s16")
	}
	if ImportsRoundtripS16(math.MaxInt16) != math.MaxInt16 {
		panic("roundtrip-s16")
	}
	if ImportsRoundtripS16(math.MinInt16) != math.MinInt16 {
		panic("roundtrip-s16")
	}

	if ImportsRoundtripU32(1) != 1 {
		panic("roundtrip-u32")
	}
	if ImportsRoundtripU32(0) != 0 {
		panic("roundtrip-u32")
	}
	if ImportsRoundtripU32(math.MaxUint32) != math.MaxUint32 {
		panic("roundtrip-u32")
	}

	if ImportsRoundtripS32(1) != 1 {
		panic("roundtrip-s32")
	}
	if ImportsRoundtripS32(math.MaxInt32) != math.MaxInt32 {
		panic("roundtrip-s32")
	}
	if ImportsRoundtripS32(math.MinInt32) != math.MinInt32 {
		panic("roundtrip-s32")
	}

	if ImportsRoundtripU64(1) != 1 {
		panic("roundtrip-u64")
	}
	if ImportsRoundtripU64(0) != 0 {
		panic("roundtrip-u64")
	}
	if ImportsRoundtripU64(math.MaxUint64) != math.MaxUint64 {
		panic("roundtrip-u64")
	}

	if ImportsRoundtripS64(1) != 1 {
		panic("roundtrip-s64")
	}
	if ImportsRoundtripS64(math.MaxInt64) != math.MaxInt64 {
		panic("roundtrip-s64")
	}
	if ImportsRoundtripS64(math.MinInt64) != math.MinInt64 {
		panic("roundtrip-s64")
	}

	if ImportsRoundtripFloat32(1.0) != 1.0 {
		panic("roundtrip-float32")
	}
	if ImportsRoundtripFloat32(math.MaxFloat32) != math.MaxFloat32 {
		panic("roundtrip-float32")
	}
	if ImportsRoundtripFloat32(math.SmallestNonzeroFloat32) != math.SmallestNonzeroFloat32 {
		panic("roundtrip-float32")
	}

	if ImportsRoundtripFloat64(1.0) != 1.0 {
		panic("roundtrip-float64")
	}
	if ImportsRoundtripFloat64(math.MaxFloat64) != math.MaxFloat64 {
		panic("roundtrip-float64")
	}
	if ImportsRoundtripFloat64(math.SmallestNonzeroFloat64) != math.SmallestNonzeroFloat64 {
		panic("roundtrip-float64")
	}
	if !math.IsNaN(ImportsRoundtripFloat64(math.NaN())) {
		panic("roundtrip-float64")
	}

	if ImportsRoundtripChar('a') != 'a' {
		panic("roundtrip-char")
	}
	if ImportsRoundtripChar(' ') != ' ' {
		panic("roundtrip-char")
	}
	if ImportsRoundtripChar('🚩') != '🚩' {
		panic("roundtrip-char")
	}

	ImportsSetScalar(2)
	if ImportsGetScalar() != 2 {
		panic("get-scalar")
	}

	ImportsSetScalar(4)
	if ImportsGetScalar() != 4 {
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
