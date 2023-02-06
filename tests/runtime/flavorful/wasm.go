package main

import (
	. "wit_flavorful_go/gen"
)

func init() {
	n := &FlavorfulImpl{}
	SetFlavorful(n)
	SetExports(n)
}

type FlavorfulImpl struct{}

func (f FlavorfulImpl) TestImports() {

	ImportsFListInRecord1(ImportsListInRecord1{"list_in_record1"})
	if ImportsFListInRecord2().A != "list_in_record2" {
		panic("ImportsFListInRecord2")
	}
	if ImportsFListInRecord3(ImportsListInRecord3{"list_in_record3 input"}).A != "list_in_record3 output" {
		panic("ImportsFListInRecord3")
	}
	if ImportsFListInRecord4(ImportsListInAlias{"input4"}).A != "result4" {
		panic("ImportsFListInRecord4")
	}

	var b Result[struct{}, string]
	b.SetErr("bar")
	ImportsFListInVariant1(Some[string]("foo"), b, ImportsListInVariant1V3F0("baz"))
	if ImportsFListInVariant2().Unwrap() != "list_in_variant2" {
		panic("ImportsFListInVariant2")
	}
	if ImportsFListInVariant3(Some[string]("input3")).Unwrap() != "output3" {
		panic("ImportsFListInVariant3")
	}
	if !ImportsErrnoResult().IsErr() {
		panic("ImportsErrnoResult")
	}
	ImportsMyErrnoA()
	// TODO: be able to print my_error_a

	if !ImportsErrnoResult().IsOk() {
		panic("ImportsErrnoResult")
	}

	t1, t2 := ImportsListTypedefs("typedef1", []string{"typedef2"})
	if len(t1) != 8 {
		panic("ImportsListTypedefs")
	}
	if len(t2) != 1 {
		panic("ImportsListTypedefs")
	}
	if t2[0] != "typedef4" {
		panic("ImportsListTypedefs")
	}

	var v2_ok Result[struct{}, struct{}]
	v2_ok.Set(struct{}{})
	var v2_err Result[struct{}, struct{}]
	v2_err.SetErr(struct{}{})

	v1, v2, v3 := ImportsListOfVariants(
		[]bool{true, false},
		[]Result[struct{}, struct{}]{v2_ok, v2_err},
		[]ImportsMyErrno{ImportsMyErrnoSuccess(), ImportsMyErrnoA()},
	)
	if v1[0] != false {
		panic("ImportsListOfVariants")
	}
	if v1[1] != true {
		panic("ImportsListOfVariants")
	}
	if v2[0].IsOk() {
		panic("ImportsListOfVariants")
	}
	if v2[1].IsErr() {
		panic("ImportsListOfVariants")
	}
	if v3[0].Kind() != ImportsMyErrnoKindA {
		panic("ImportsListOfVariants")
	}
	if v3[1].Kind() != ImportsMyErrnoKindB {
		panic("ImportsListOfVariants")
	}

}

func (f FlavorfulImpl) FListInRecord1(a ExportsListInRecord1) {
	if a.A != "list_in_record1" {
		panic("FListInRecord1")
	}
}

func (f FlavorfulImpl) FListInRecord2() ExportsListInRecord2 {
	return ExportsListInRecord2{"list_in_record2"}
}

func (f FlavorfulImpl) FListInRecord3(a ExportsListInRecord3) ExportsListInRecord3 {
	if a.A != "list_in_record3 input" {
		panic("FListInRecord3")
	}
	return ExportsListInRecord3{"list_in_record3 output"}
}

func (f FlavorfulImpl) FListInRecord4(a ExportsListInRecord4) ExportsListInRecord4 {
	if a.A != "input4" {
		panic("FListInRecord4")
	}
	return ExportsListInRecord4{"result4"}
}

func (f FlavorfulImpl) FListInVariant1(a Option[string], b Result[struct{}, string], c ExportsListInVariant1V3) {
	if a.Unwrap() != "foo" {
		panic("FListInVariant1")
	}
	if b.UnwrapErr() != "bar" {
		panic("FListInVariant1")
	}
	switch c.Kind() {
	case ExportsListInVariant1V3KindF0:
		if c.GetF0() != "baz" {
			panic("FListInVariant1")
		}
	case ExportsListInVariant1V3KindF1:
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

func (f FlavorfulImpl) ErrnoResult() Result[struct{}, ExportsMyErrno] {
	var res Result[struct{}, ExportsMyErrno]
	res.SetErr(ExportsMyErrnoB())
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

func (f FlavorfulImpl) ListOfVariants(a []bool, b []Result[struct{}, struct{}], c []ExportsMyErrno) ([]bool, []Result[struct{}, struct{}], []ExportsMyErrno) {
	return a, b, c
}

func main() {}
