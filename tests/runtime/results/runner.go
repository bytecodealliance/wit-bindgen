package export_wit_world

import (
	"fmt"
	test "wit_component/test_results_test"
)

func Run() {
	{
		val := test.StringError(0.0)
		assertEqual(val.Err(), "zero")

		val = test.StringError(1.0)
		assertEqual(val.Ok(), 1.0)
	}

	{
		val := test.EnumError(0.0)
		assertEqual(val.Err(), test.EA)

		val = test.EnumError(1.0)
		assertEqual(val.Ok(), 1.0)
	}

	{
		val := test.RecordError(0.0)
		assertEqual(val.Err(), test.E2{420, 0})

		val = test.RecordError(1.0)
		assertEqual(val.Err(), test.E2{77, 2})

		val = test.RecordError(2.0)
		assertEqual(val.Ok(), 2.0)
	}

	{
		a := test.VariantError(0.0)
		b := a.Err()
		assertEqual(b.E2(), test.E2{420, 0})

		a = test.VariantError(1.0)
		b = a.Err()
		assertEqual(b.E1(), test.EB)

		a = test.VariantError(2.0)
		b = a.Err()
		assertEqual(b.E1(), test.EC)
	}

	{
		val := test.EmptyError(0)
		val.Err()

		val = test.EmptyError(1)
		assertEqual(val.Ok(), 42)

		val = test.EmptyError(2)
		assertEqual(val.Ok(), 2)
	}

	{
		a := test.DoubleError(0)
		b := a.Ok()
		b.Ok()

		a = test.DoubleError(1)
		b = a.Ok()
		assertEqual(b.Err(), "one")

		a = test.DoubleError(2)
		assertEqual(a.Err(), "two")
	}
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
