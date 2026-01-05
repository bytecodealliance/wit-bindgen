package export_my_test_i

import . "github.com/bytecodealliance/wit-bindgen/wit_types"

func ReadFuture(x *FutureReader[Unit]) {
	defer x.Drop()
	x.Read()
}

func DropFuture(x *FutureReader[Unit]) {
	x.Drop()
}
