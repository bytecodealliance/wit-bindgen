package main

import (
	"math"
	. "wit_lists_go/gen"
)

func init() {
	a := ListImpl{}
	SetLists(a)
	SetExports(a)
}

type ListImpl struct {
}

func (i ListImpl) TestImports() {
	ImportsEmptyListParam([]uint8{})
	ImportsEmptyStringParam("")
	res := ImportsEmptyListResult()
	if len(res) != 0 {
		panic("ImportsEmptyListResult")
	}
	res2 := ImportsEmptyStringResult()
	if res2 != "" {
		panic("ImportsEmptyStringResult")
	}
	ImportsListParam([]uint8{1, 2, 3, 4})
	ImportsListParam2("foo")
	ImportsListParam3([]string{"foo", "bar", "baz"})
	ImportsListParam4([][]string{{"foo", "bar"}, {"baz"}})
	res3 := ImportsListResult()
	if len(res3) != 5 {
		panic("ImportsListResult")
	}
	for i := range res3 {
		if res3[i] != uint8(i+1) {
			panic("ImportsListResult")
		}
	}
	res4 := ImportsListResult2()
	if res4 != "hello!" {
		panic("ImportsListResult2")
	}
	res5 := ImportsListResult3()
	if len(res5) != 2 {
		panic("ImportsListResult3")
	}
	if res5[0] != "hello," {
		panic("ImportsListResult3")
	}
	if res5[1] != "world!" {
		panic("ImportsListResult3")
	}

	res6 := ImportsListRoundtrip([]uint8{})
	if len(res6) != 0 {
		panic("ImportsListRoundtrip")
	}
	res7 := ImportsListRoundtrip([]uint8{1, 2, 3, 4, 5})
	if len(res7) != 5 {
		panic("ImportsListRoundtrip")
	}

	res8 := ImportsStringRoundtrip("")
	if res8 != "" {
		panic("ImportsStringRoundtrip")
	}
	res9 := ImportsStringRoundtrip("hello ⚑ world")
	if res9 != "hello ⚑ world" {
		panic("ImportsStringRoundtrip")
	}

	u8, i8 := ImportsListMinmax8([]uint8{0, math.MaxUint8}, []int8{math.MinInt8, math.MaxInt8})
	if u8[0] != uint8(0) {
		panic("ImportsListMinmax8")
	}
	if u8[1] != math.MaxUint8 {
		panic("ImportsListMinmax8")
	}
	if i8[0] != math.MinInt8 {
		panic("ImportsListMinmax8")
	}
	if i8[1] != math.MaxInt8 {
		panic("ImportsListMinmax8")
	}

	u16, i16 := ImportsListMinmax16([]uint16{0, math.MaxUint16}, []int16{math.MinInt16, math.MaxInt16})
	if u16[0] != uint16(0) {
		panic("ImportsListMinmax16")
	}
	if u16[1] != math.MaxUint16 {
		panic("ImportsListMinmax16")
	}
	if i16[0] != math.MinInt16 {
		panic("ImportsListMinmax16")
	}
	if i16[1] != math.MaxInt16 {
		panic("ImportsListMinmax16")
	}

	u32, i32 := ImportsListMinmax32([]uint32{0, math.MaxUint32}, []int32{math.MinInt32, math.MaxInt32})
	if u32[0] != uint32(0) {
		panic("ImportsListMinmax32")
	}
	if u32[1] != math.MaxUint32 {
		panic("ImportsListMinmax32")
	}
	if i32[0] != math.MinInt32 {
		panic("ImportsListMinmax32")
	}
	if i32[1] != math.MaxInt32 {
		panic("ImportsListMinmax32")
	}

	u64, i64 := ImportsListMinmax64([]uint64{0, math.MaxUint64}, []int64{math.MinInt64, math.MaxInt64})
	if u64[0] != uint64(0) {
		panic("ImportsListMinmax64")
	}
	if u64[1] != math.MaxUint64 {
		panic("ImportsListMinmax64")
	}
	if i64[0] != math.MinInt64 {
		panic("ImportsListMinmax64")
	}
	if i64[1] != math.MaxInt64 {
		panic("ImportsListMinmax64")
	}

}

func (i ListImpl) AllocatedBytes() uint32 {
	return 0
}

func (i ListImpl) EmptyListParam(a []uint8) {
	if len(a) != 0 {
		panic("EmptyListParam")
	}
}

func (i ListImpl) EmptyStringParam(a string) {
	if a != "" {
		panic("EmptyStringParam")
	}
}

func (i ListImpl) EmptyListResult() []uint8 {
	return []uint8{}
}

func (i ListImpl) EmptyStringResult() string {
	return ""
}

func (i ListImpl) ListParam(a []uint8) {
	if len(a) != 4 {
		panic("ListParam")
	}
	for i := range a {
		if a[i] != uint8(i+1) {
			panic("ListParam")
		}
	}
}

func (i ListImpl) ListParam2(a string) {
	if a != "foo" {
		panic("ListParam2")
	}
}

func (i ListImpl) ListParam3(a []string) {
	if len(a) != 3 {
		panic("ListParam3")
	}
	if a[0] != "foo" {
		panic("ListParam3")
	}
	if a[1] != "bar" {
		panic("ListParam3")
	}
	if a[2] != "baz" {
		panic("ListParam3")
	}
}

func (i ListImpl) ListParam4(a [][]string) {
	if len(a) != 2 {
		panic("ListParam4")
	}
	if a[0][0] != "foo" {
		panic("ListParam4")
	}
	if a[0][1] != "bar" {
		panic("ListParam4")
	}
	if a[1][0] != "baz" {
		panic("ListParam4")
	}
}

func (i ListImpl) ListResult() []uint8 {
	return []uint8{1, 2, 3, 4, 5}
}

func (i ListImpl) ListResult2() string {
	return "hello!"
}

func (i ListImpl) ListResult3() []string {
	return []string{"hello,", "world!"}
}

func (i ListImpl) ListMinmax8(a []uint8, b []int8) ([]uint8, []int8) {
	return a, b
}

func (i ListImpl) ListMinmax16(a []uint16, b []int16) ([]uint16, []int16) {
	return a, b
}

func (i ListImpl) ListMinmax32(a []uint32, b []int32) ([]uint32, []int32) {
	return a, b
}

func (i ListImpl) ListMinmax64(a []uint64, b []int64) ([]uint64, []int64) {
	return a, b
}

func (i ListImpl) ListRoundtrip(a []uint8) []uint8 {
	return a
}

func (i ListImpl) StringRoundtrip(a string) string {
	return a
}

func main() {}
