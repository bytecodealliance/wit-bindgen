package export_wit_world

import (
	"fmt"
	"slices"
	test "wit_component/test_lists_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Run() {
	test.EmptyListParam([]uint8{})
	test.EmptyStringParam("")
	assertEqual(0, len(test.EmptyListResult()))
	assertEqual(0, len(test.EmptyStringResult()))
	test.ListParam([]uint8{1, 2, 3, 4})
	test.ListParam2("foo")
	test.ListParam3([]string{"foo", "bar", "baz"})
	test.ListParam4([][]string{[]string{"foo", "bar"}, []string{"baz"}})
	test.ListParam5([]Tuple3[uint8, uint32, uint8]{
		Tuple3[uint8, uint32, uint8]{1, 2, 3},
		Tuple3[uint8, uint32, uint8]{4, 5, 6},
	})

	large := make([]string, 0, 1000)
	for i := 0; i < 1000; i++ {
		large = append(large, "string")
	}
	test.ListParamLarge(large)

	assert(slices.Equal(test.ListResult(), []uint8{1, 2, 3, 4, 5}))
	assertEqual(test.ListResult2(), "hello!")
	assert(slices.Equal(test.ListResult3(), []string{"hello,", "world!"}))
	assert(slices.Equal(test.ListRoundtrip([]uint8{}), []uint8{}))

	{
		headers := []Tuple2[string, []uint8]{
			{"Content-Type", []uint8("text/plain")},
			{"Content-Length", []uint8("9")},
		}
		result := test.WasiHttpHeadersRoundtrip(headers)
		assertEqual(len(result), 2)
		assertEqual(result[0].F0, "Content-Type")
		assert(slices.Equal(result[0].F1, []uint8("text/plain")))
		assertEqual(result[1].F0, "Content-Length")
		assert(slices.Equal(result[1].F1, []uint8("9")))
	}
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
