package export_wit_world

import (
	"fmt"
	"slices"
	test "wit_component/test_flavorful_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Run() {
	test.FListInRecord1(test.ListInRecord1{"list_in_record1"})

	assertEqual(test.FListInRecord2().A, "list_in_record2")

	assertEqual(test.FListInRecord3(test.ListInRecord3{"list_in_record3 input"}).A, "list_in_record3 output")

	assertEqual(test.FListInRecord4(test.ListInAlias{"input4"}).A, "result4")

	test.FListInVariant1(
		Some[string]("foo"),
		Err[Unit, string]("bar"),
	)

	assertEqual(test.FListInVariant2().Some(), "list_in_variant2")

	assertEqual(test.FListInVariant3(Some[string]("input3")).Some(), "output3")

	assertEqual(test.ErrnoResult().Err(), test.MyErrnoB)
	test.ErrnoResult().Ok()

	{
		a, b := test.ListTypedefs("typedef1", []string{"typedef2"})
		assert(slices.Equal(a, []byte("typedef3")))
		assert(slices.Equal(b, []string{"typedef4"}))
	}

	{
		a, b, c := test.ListOfVariants(
			[]bool{true, false},
			[]Result[Unit, Unit]{
				Ok[Unit, Unit](Unit{}),
				Err[Unit, Unit](Unit{}),
			},
			[]test.MyErrno{test.MyErrnoSuccess, test.MyErrnoA},
		)
		assert(slices.Equal(a, []bool{false, true}))
		assert(slices.Equal(b, []Result[Unit, Unit]{
			Err[Unit, Unit](Unit{}),
			Ok[Unit, Unit](Unit{}),
		},
		))
		assert(slices.Equal(c, []test.MyErrno{test.MyErrnoA, test.MyErrnoB}))
	}
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}
