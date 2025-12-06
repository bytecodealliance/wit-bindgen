package export_my_test_i

import . "wit_component/wit_types"

func ReadFuture(x *FutureReader[Unit]) {
	defer x.Drop()
	x.Read()
}

func DropFuture(x *FutureReader[Unit]) {
	x.Drop()
}
