package export_wit_world

import (
	"fmt"
	test "wit_component/test_resource_own_roundtrip_to_test"
)

func Run() {
	assertEqual(test.ConsumeBoth(test.MakeThing(1), test.MakeThing(2)), uint32(3))
	assertEqual(test.NumDropped(), uint32(2))

	a := test.MakeThing(3)
	b := test.MakeThing(4)
	assertEqual(test.ConsumeBoth(b, a), uint32(7))
	assertEqual(test.NumDropped(), uint32(4))
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
