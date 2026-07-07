//@ wasmtime-flags = '-Wcomponent-model-async'

package export_wit_world

import (
	"fmt"
	test "wit_component/my_test_i"
)

type Unit struct{}

func Run() {
	write := make(chan Unit)
	read := make(chan Unit)

	tx, rx := test.MakeStreamU8()
	go func() {
		assertEqual(tx.Write([]uint8{0}), 1)
		assert(!tx.ReaderDropped())

		assertEqual(tx.Write([]uint8{1, 2}), 2)
		assert(!tx.ReaderDropped())

		assertEqual(tx.Write([]uint8{3, 4}), 2)

		assertEqual(tx.Write([]uint8{0}), 0)
		assert(tx.ReaderDropped())

		write <- Unit{}
	}()

	go func() {
		test.ReadStream(rx)
		read <- Unit{}
	}()

	(<-read)
	(<-write)
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
