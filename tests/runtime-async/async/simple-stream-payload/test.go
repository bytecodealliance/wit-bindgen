package export_my_test_i

import (
	"fmt"
	"slices"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func ReadStream(x *StreamReader[uint8]) {
	defer x.Drop()

	{
		buffer := make([]uint8, 1)
		count := x.Read(buffer)
		assertEqual(count, 1)
		assert(slices.Equal(buffer, []uint8{0}))
		assert(!x.WriterDropped())
	}

	{
		buffer := make([]uint8, 2)
		count := x.Read(buffer)
		assertEqual(count, 2)
		assert(slices.Equal(buffer, []uint8{1, 2}))
		assert(!x.WriterDropped())
	}

	{
		buffer := make([]uint8, 1)
		count := x.Read(buffer)
		assertEqual(count, 1)
		assert(slices.Equal(buffer, []uint8{3}))
		assert(!x.WriterDropped())
	}

	{
		buffer := make([]uint8, 1)
		count := x.Read(buffer)
		assertEqual(count, 1)
		assert(slices.Equal(buffer, []uint8{4}))
	}
}

func assertEqual[T comparable](a, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}
