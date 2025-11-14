package export_wit_world

import (
	"fmt"
	test "wit_component/my_test_i"
	"wit_component/wit_types"
)

func Run() {
	write := make(chan wit_types.Unit)
	read := make(chan wit_types.Unit)

	tx, rx := test.MakeStreamUnit()
	go func() {
		assertEqual(tx.Write([]wit_types.Unit{wit_types.Unit{}}), 1)
		assert(!tx.ReaderDropped())

		assertEqual(tx.Write([]wit_types.Unit{wit_types.Unit{}, wit_types.Unit{}}), 2)

		assertEqual(tx.Write([]wit_types.Unit{wit_types.Unit{}, wit_types.Unit{}}), 0)
		assert(tx.ReaderDropped())

		write <- wit_types.Unit{}
	}()

	go func() {
		test.ReadStream(rx)
		read <- wit_types.Unit{}
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
