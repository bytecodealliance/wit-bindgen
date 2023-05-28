package main

import (
	. "wit_flavorful_go/gen"
)

func init() {
	n := &FlavorfulImpl{}
	SetFlavorful(n)
	SetExportsTestFlavorfulTest(n)
}

type FlavorfulImpl struct{}

func (f FlavorfulImpl) TestImports() {

	TestFlavorfulTestFListInRecord1(TestFlavorfulTestListInRecord1{"list_in_record1"})
	if TestFlavorfulTestFListInRecord2().A != "list_in_record2" {
		panic("TestFlavorfulTestFListInRecord2")
	}
	if TestFlavorfulTestFListInRecord3(TestFlavorfulTestListInRecord3{"list_in_record3 input"}).A != "list_in_record3 output" {
		panic("TestFlavorfulTestFListInRecord3")
	}
	if TestFlavorfulTestFListInRecord4(TestFlavorfulTestListInAlias{"input4"}).A != "result4" {
		panic("TestFlavorfulTestFListInRecord4")
	}

	var b Result[struct{}, string]
	b.SetErr("bar")
	TestFlavorfulTestFListInVariant1(Some[string]("foo"), b, TestFlavorfulTestListInVariant1V3F0("baz"))
	if TestFlavorfulTestFListInVariant2().Unwrap() != "list_in_variant2" {
		panic("TestFlavorfulTestFListInVariant2")
	}
	if TestFlavorfulTestFListInVariant3(Some[string]("input3")).Unwrap() != "output3" {
		panic("TestFlavorfulTestFListInVariant3")
	}
	if !TestFlavorfulTestErrnoResult().IsErr() {
		panic("TestFlavorfulTestErrnoResult")
	}
	TestFlavorfulTestMyErrnoA()
	// TODO: be able to print my_error_a

	if !TestFlavorfulTestErrnoResult().IsOk() {
		panic("TestFlavorfulTestErrnoResult")
	}

	t1, t2 := TestFlavorfulTestListTypedefs("typedef1", []string{"typedef2"})
	if len(t1) != 8 {
		panic("TestFlavorfulTestListTypedefs")
	}
	if len(t2) != 1 {
		panic("TestFlavorfulTestListTypedefs")
	}
	if t2[0] != "typedef4" {
		panic("TestFlavorfulTestListTypedefs")
	}

	var v2_ok Result[struct{}, struct{}]
	v2_ok.Set(struct{}{})
	var v2_err Result[struct{}, struct{}]
	v2_err.SetErr(struct{}{})

	v1, v2, v3 := TestFlavorfulTestListOfVariants(
		[]bool{true, false},
		[]Result[struct{}, struct{}]{v2_ok, v2_err},
		[]TestFlavorfulTestMyErrno{TestFlavorfulTestMyErrnoSuccess(), TestFlavorfulTestMyErrnoA()},
	)
	if v1[0] != false {
		panic("TestFlavorfulTestListOfVariants")
	}
	if v1[1] != true {
		panic("TestFlavorfulTestListOfVariants")
	}
	if v2[0].IsOk() {
		panic("TestFlavorfulTestListOfVariants")
	}
	if v2[1].IsErr() {
		panic("TestFlavorfulTestListOfVariants")
	}
	if v3[0].Kind() != TestFlavorfulTestMyErrnoKindA {
		panic("TestFlavorfulTestListOfVariants")
	}
	if v3[1].Kind() != TestFlavorfulTestMyErrnoKindB {
		panic("TestFlavorfulTestListOfVariants")
	}

}

func (f FlavorfulImpl) FListInRecord1(a TestFlavorfulTestListInRecord1) {
	if a.A != "list_in_record1" {
		panic("FListInRecord1")
	}
}

func (f FlavorfulImpl) FListInRecord2() TestFlavorfulTestListInRecord2 {
	return TestFlavorfulTestListInRecord2{"list_in_record2"}
}

func (f FlavorfulImpl) FListInRecord3(a TestFlavorfulTestListInRecord3) TestFlavorfulTestListInRecord3 {
	if a.A != "list_in_record3 input" {
		panic("FListInRecord3")
	}
	return TestFlavorfulTestListInRecord3{"list_in_record3 output"}
}

func (f FlavorfulImpl) FListInRecord4(a TestFlavorfulTestListInRecord4) TestFlavorfulTestListInRecord4 {
	if a.A != "input4" {
		panic("FListInRecord4")
	}
	return TestFlavorfulTestListInRecord4{"result4"}
}

func (f FlavorfulImpl) FListInVariant1(a Option[string], b Result[struct{}, string], c TestFlavorfulTestListInVariant1V3) {
	if a.Unwrap() != "foo" {
		panic("FListInVariant1")
	}
	if b.UnwrapErr() != "bar" {
		panic("FListInVariant1")
	}
	switch c.Kind() {
	case TestFlavorfulTestListInVariant1V3KindF0:
		if c.GetF0() != "baz" {
			panic("FListInVariant1")
		}
	case TestFlavorfulTestListInVariant1V3KindF1:
		panic("FListInVariant1")
	}
}

func (f FlavorfulImpl) FListInVariant2() Option[string] {
	return Some[string]("list_in_variant2")
}

func (f FlavorfulImpl) FListInVariant3(a Option[string]) Option[string] {
	if a.Unwrap() != "input3" {
		panic("FListInVariant3")
	}
	return Some[string]("output3")
}

func (f FlavorfulImpl) ErrnoResult() Result[struct{}, TestFlavorfulTestMyErrno] {
	var res Result[struct{}, TestFlavorfulTestMyErrno]
	res.SetErr(TestFlavorfulTestMyErrnoB())
	return res
}

func (f FlavorfulImpl) ListTypedefs(a string, c []string) ([]uint8, []string) {
	if a != "typedef1" {
		panic("ListTypedefs")
	}
	if len(c) != 1 {
		panic("ListTypedefs")
	}
	if c[0] != "typedef2" {
		panic("ListTypedefs")
	}
	return []uint8("typedef3"), []string{"typedef4"}
}

func (f FlavorfulImpl) ListOfVariants(a []bool, b []Result[struct{}, struct{}], c []TestFlavorfulTestMyErrno) ([]bool, []Result[struct{}, struct{}], []TestFlavorfulTestMyErrno) {
	return a, b, c
}

func main() {}
