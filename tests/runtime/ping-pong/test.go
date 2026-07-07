package export_my_test_i

import (
	"wit_component/my_test_i"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Ping(x *FutureReader[string], y string) *FutureReader[string] {
	message := x.Read() + y
	tx, rx := my_test_i.MakeFutureString()
	go func() {
		tx.Write(message)
	}()
	return rx
}

func Pong(x *FutureReader[string]) string {
	return x.Read()
}
