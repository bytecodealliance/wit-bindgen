//@ wasmtime-flags = '-Wcomponent-model-async'

package export_wit_world

import (
	"fmt"
	test "wit_component/a_b_i"
)

func Run() {
	test.OneArgument(1)
	assertEqual(test.OneResult(), 2)
	assertEqual(test.OneArgumentAndResult(3), 4)
	test.TwoArguments(5, 6)
	assertEqual(test.TwoArgumentsAndResult(7, 8), 9)
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
