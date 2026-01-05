package export_wit_world

import (
	test "wit_component/my_test_i"

	"github.com/bytecodealliance/wit-bindgen/wit_types"
)

func Run() {
	write := make(chan bool)
	read := make(chan wit_types.Unit)

	{
		tx, rx := test.MakeFutureUnit()
		go func() {
			write <- tx.Write(wit_types.Unit{})
		}()
		go func() {
			test.ReadFuture(rx)
			read <- wit_types.Unit{}
		}()
		(<-read)
		assert(<-write)
	}

	{
		tx, rx := test.MakeFutureUnit()
		go func() {
			write <- tx.Write(wit_types.Unit{})
		}()
		go func() {
			test.DropFuture(rx)
			read <- wit_types.Unit{}
		}()
		(<-read)
		assert(!(<-write))
	}
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}
