package export_wit_world

import (
	"fmt"
	test "wit_component/test_resource_borrow_to_test"
)

func Run() {
	thing := test.MakeThing(42)
	defer thing.Drop()

	result := test.Foo(thing)
	assertEqual(result, uint32(42+1+2))
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
