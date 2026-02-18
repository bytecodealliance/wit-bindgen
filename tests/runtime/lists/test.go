package export_test_lists_to_test

import (
	"slices"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func AllocatedBytes() uint32 {
	return 0
}

func EmptyListParam(x []uint8) {
	if len(x) != 0 {
		panic("trouble")
	}
}

func EmptyStringParam(x string) {
	if len(x) != 0 {
		panic("trouble")
	}
}

func EmptyListResult() []uint8 {
	return []uint8{}
}

func EmptyStringResult() string {
	return ""
}

func ListParam(x []uint8) {
	if !slices.Equal(x, []uint8{1, 2, 3, 4}) {
		panic("trouble")
	}
}

func ListParam2(x string) {
	if x != "foo" {
		panic("trouble")
	}
}

func ListParam3(x []string) {
	if !slices.Equal(x, []string{"foo", "bar", "baz"}) {
		panic("trouble")
	}
}

func ListParam4(x [][]string) {
	if !slices.Equal(x[0], []string{"foo", "bar"}) {
		panic("trouble")
	}
	if !slices.Equal(x[1], []string{"baz"}) {
		panic("trouble")
	}
}

func ListParam5(x []Tuple3[uint8, uint32, uint8]) {
	if !slices.Equal(x, []Tuple3[uint8, uint32, uint8]{
		Tuple3[uint8, uint32, uint8]{1, 2, 3},
		Tuple3[uint8, uint32, uint8]{4, 5, 6},
	}) {
		panic("trouble")
	}
}

func ListParamLarge(x []string) {
	if len(x) != 1000 {
		panic("trouble")
	}
}

func ListResult() []uint8 {
	return []uint8{1, 2, 3, 4, 5}
}

func ListResult2() string {
	return "hello!"
}

func ListResult3() []string {
	return []string{"hello,", "world!"}
}

func ListRoundtrip(x []uint8) []uint8 {
	return x
}

func StringRoundtrip(x string) string {
	return x
}

func ListMinmax8(x []uint8, y []int8) ([]uint8, []int8) {
	return x, y
}

func ListMinmax16(x []uint16, y []int16) ([]uint16, []int16) {
	return x, y
}

func ListMinmax32(x []uint32, y []int32) ([]uint32, []int32) {
	return x, y
}

func ListMinmax64(x []uint64, y []int64) ([]uint64, []int64) {
	return x, y
}

func ListMinmaxFloat(x []float32, y []float64) ([]float32, []float64) {
	return x, y
}
