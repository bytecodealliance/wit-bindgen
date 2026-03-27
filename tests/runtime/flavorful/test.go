package export_test_flavorful_to_test

import (
	"slices"
	. "wit_component/test_flavorful_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func FListInRecord1(x ListInRecord1) {
	if x.A != "list_in_record1" {
		panic("trouble")
	}
}

func FListInRecord2() ListInRecord2 {
	return ListInRecord2{"list_in_record2"}
}

func FListInRecord3(x ListInRecord3) ListInRecord3 {
	if x.A != "list_in_record3 input" {
		panic("trouble")
	}
	return ListInRecord3{"list_in_record3 output"}
}

func FListInRecord4(x ListInAlias) ListInAlias {
	if x.A != "input4" {
		panic("trouble")
	}
	return ListInRecord4{"result4"}
}

func FListInVariant1(x ListInVariant1V1, y ListInVariant1V2) {
	if x.Some() != "foo" {
		panic("trouble")
	}
	if y.Err() != "bar" {
		panic("trouble")
	}
}

func FListInVariant2() Option[string] {
	return Some[string]("list_in_variant2")
}

func FListInVariant3(x ListInVariant3) Option[string] {
	if x.Some() != "input3" {
		panic("trouble")
	}
	return Some[string]("output3")
}

var first bool = true

func ErrnoResult() Result[Unit, MyErrno] {
	if first {
		first = false
		return Err[Unit, MyErrno](MyErrnoB)
	} else {
		return Ok[Unit, MyErrno](Unit{})
	}
}

func ListTypedefs(x ListTypedef, y ListTypedef3) (ListTypedef2, ListTypedef3) {
	if x != "typedef1" {
		panic("trouble")
	}
	if !slices.Equal(y, []string{"typedef2"}) {
		panic("trouble")
	}
	return []uint8("typedef3"), []string{"typedef4"}
}

func ListOfVariants(bools []bool, results []Result[Unit, Unit], enums []MyErrno) (
	[]bool,
	[]Result[Unit, Unit],
	[]MyErrno,
) {
	if !slices.Equal(bools, []bool{true, false}) {
		panic("trouble")
	}
	if len(results) != 2 {
		panic("trouble")
	}
	if results[0].Tag() != ResultOk {
		panic("trouble")
	}
	if results[1].Tag() != ResultErr {
		panic("trouble")
	}
	if len(enums) != 2 {
		panic("trouble")
	}
	if enums[0] != MyErrnoSuccess {
		panic("trouble")
	}
	if enums[1] != MyErrnoA {
		panic("trouble")
	}
	return []bool{false, true},
		[]Result[Unit, Unit]{Err[Unit, Unit](Unit{}), Ok[Unit, Unit](Unit{})},
		[]MyErrno{MyErrnoA, MyErrnoB}
}
