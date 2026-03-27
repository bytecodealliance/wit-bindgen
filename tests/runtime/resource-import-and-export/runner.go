package export_wit_world

import (
	"fmt"
	. "wit_component/test_resource_import_and_export_test"
)

func Run() {
	thing1 := MakeThing(42)
	defer thing1.Drop()
	// 42 + 1 (constructor) + 1 (constructor) + 2 (foo) + 2 (foo)
	assertEqual(thing1.Foo(), 48)

	// 33 + 3 (bar) + 3 (bar) + 2 (foo) + 2 (foo)
	thing1.Bar(33)
	assertEqual(thing1.Foo(), 43)

	thing2 := MakeThing(81)
	defer thing2.Drop()
	thing3 := ThingBaz(thing1, thing2)
	defer thing3.Drop()
	assertEqual(thing3.Foo(), 33+3+3+81+1+1+2+2+4+1+2+4+1+1+2+2)
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
