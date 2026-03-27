package export_my_test_i

import (
	"fmt"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func ReadStream(x *StreamReader[Unit]) {
	defer x.Drop()

	{
		buffer := make([]Unit, 1)
		count := x.Read(buffer)
		assertEqual(count, 1)
		assert(!x.WriterDropped())
	}

	{
		buffer := make([]Unit, 2)
		count := x.Read(buffer)
		assertEqual(count, 2)
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
