//@ wasmtime-flags = '-Wcomponent-model-async'

package export_wit_world

import (
	"fmt"
	test "wit_component/my_test_i"

	witTypes "go.bytecodealliance.org/pkg/wit/types"
)

type Unit struct{}

func Run() {
	{
		f1 := make(chan *witTypes.FutureReader[string])
		f2 := make(chan Unit)

		tx, rx := test.MakeFutureString()
		go func() {
			f1 <- test.Ping(rx, "world")
		}()

		go func() {
			tx.Write("hello")
			f2 <- Unit{}
		}()

		(<-f2)
		rx2 := (<-f1)
		assertEqual(rx2.Read(), "helloworld")
	}

	{
		f1 := make(chan Unit)
		f2 := make(chan Unit)

		tx, rx := test.MakeFutureString()
		go func() {
			assertEqual(test.Pong(rx), "helloworld")
			f1 <- Unit{}
		}()

		go func() {
			tx.Write("helloworld")
			f2 <- Unit{}
		}()

		(<-f2)
		(<-f1)
	}
}

func assertEqual[T comparable](a, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
