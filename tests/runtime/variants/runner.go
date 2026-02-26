package export_wit_world

import (
	"fmt"
	test "wit_component/test_variants_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Run() {
	assertEqual(test.RoundtripOption(Some[float32](1.0)).Some(), 1)
	assertEqual(test.RoundtripOption(None[float32]()).Tag(), OptionNone)
	assertEqual(test.RoundtripOption(Some[float32](2.0)).Some(), 2)

	assertEqual(test.RoundtripResult(Ok[uint32, float32](2)).Ok(), 2.0)
	assertEqual(test.RoundtripResult(Ok[uint32, float32](4)).Ok(), 4.0)
	assertEqual(test.RoundtripResult(Err[uint32, float32](5.3)).Err(), 5)

	assertEqual(test.InvertBool(true), false)
	assertEqual(test.InvertBool(false), true)

	{
		a, b, c, d, e, f := test.VariantCasts(test.Casts{
			test.MakeC1A(1),
			test.MakeC2A(2),
			test.MakeC3A(3),
			test.MakeC4A(4),
			test.MakeC5A(5),
			test.MakeC6A(6.0),
		})
		assertEqual(a.A(), 1)
		assertEqual(b.A(), 2)
		assertEqual(c.A(), 3)
		assertEqual(d.A(), 4)
		assertEqual(e.A(), 5)
		assertEqual(f.A(), 6.0)
	}

	{
		a, b, c, d, e, f := test.VariantCasts(test.Casts{
			test.MakeC1B(1),
			test.MakeC2B(2.0),
			test.MakeC3B(3.0),
			test.MakeC4B(4.0),
			test.MakeC5B(5.0),
			test.MakeC6B(6.0),
		})
		assertEqual(a.B(), 1)
		assertEqual(b.B(), 2.0)
		assertEqual(c.B(), 3.0)
		assertEqual(d.B(), 4.0)
		assertEqual(e.B(), 5.0)
		assertEqual(f.B(), 6.0)
	}

	{
		a, b, c, d := test.VariantZeros(test.Zeros{
			test.MakeZ1A(1),
			test.MakeZ2A(2),
			test.MakeZ3A(3.0),
			test.MakeZ4A(4.0),
		})
		assertEqual(a.A(), 1)
		assertEqual(b.A(), 2)
		assertEqual(c.A(), 3.0)
		assertEqual(d.A(), 4.0)
	}

	{
		a, b, c, d := test.VariantZeros(test.Zeros{
			test.MakeZ1B(),
			test.MakeZ2B(),
			test.MakeZ3B(),
			test.MakeZ4B(),
		})
		assertEqual(a.Tag(), test.Z1B)
		assertEqual(b.Tag(), test.Z2B)
		assertEqual(c.Tag(), test.Z3B)
		assertEqual(d.Tag(), test.Z4B)
	}

	test.VariantTypedefs(None[uint32](), false, Err[uint32, Unit](Unit{}))

	{
		a, b, c := test.VariantEnums(true, Ok[Unit, Unit](Unit{}), test.MyErrnoSuccess)
		assertEqual(a, true)
		b.Ok()
		assertEqual(c, test.MyErrnoSuccess)
	}
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
