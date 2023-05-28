package main

import (
	. "wit_variants_go/gen"
)

func init() {
	a := VariantsImpl{}
	SetExportsTestVariantsTest(a)
	SetVariants(a)
}

type VariantsImpl struct {
}

func (i VariantsImpl) TestImports() {
	{
		res := TestVariantsTestRoundtripOption(Some[float32](1))
		if res.IsNone() {
			panic("TestVariantsTestRoundtripOption")
		}
		if res.Unwrap() != 1.0 {
			panic("TestVariantsTestRoundtripOption")
		}

		res2 := TestVariantsTestRoundtripOption(None[float32]())
		if res2.IsSome() {
			panic("TestVariantsTestRoundtripOption")
		}
	}
	{
		var param Result[uint32, float32]
		param.Set(5)
		res := TestVariantsTestRoundtripResult(param)
		if res.IsErr() {
			panic("TestVariantsTestRoundtripResult")
		}
		if res.Unwrap() != 5 {
			panic("TestVariantsTestRoundtripResult")
		}

		param.SetErr(10.0)
		res2 := TestVariantsTestRoundtripResult(param)
		if res2.IsOk() {
			panic("TestVariantsTestRoundtripResult")
		}
		if res2.UnwrapErr() != 10.0 {
			panic("TestVariantsTestRoundtripResult")
		}
	}

	{
		a := TestVariantsTestE1A()
		res := TestVariantsTestRoundtripEnum(a)
		if res.Kind() != TestVariantsTestE1KindA {
			panic("TestVariantsTestRoundtripEnum")
		}

		b := TestVariantsTestE1B()
		res2 := TestVariantsTestRoundtripEnum(b)
		if res2.Kind() != TestVariantsTestE1KindB {
			panic("TestVariantsTestRoundtripEnum")
		}
	}
	{
		if TestVariantsTestInvertBool(true) != false {
			panic("TestVariantsTestRoundtripBool")
		}
		if TestVariantsTestInvertBool(false) != true {
			panic("TestVariantsTestRoundtripBool")
		}
	}
	{
		var a TestVariantsTestCasts
		a.F0 = TestVariantsTestC1A(1)
		a.F1 = TestVariantsTestC2A(2)
		a.F2 = TestVariantsTestC3A(3)
		a.F3 = TestVariantsTestC4A(4)
		a.F4 = TestVariantsTestC5A(5)
		a.F5 = TestVariantsTestC6A(6.0)
		res := TestVariantsTestVariantCasts(a)
		if res.F0.GetA() != 1 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F1.GetA() != 2 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F2.GetA() != 3 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F3.GetA() != 4 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F4.GetA() != 5 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F5.GetA() != 6.0 {
			panic("TestVariantsTestVariantCasts")
		}
	}
	{
		var a TestVariantsTestCasts
		a.F0 = TestVariantsTestC1B(1)
		a.F1 = TestVariantsTestC2B(2.0)
		a.F2 = TestVariantsTestC3B(3.0)
		a.F3 = TestVariantsTestC4B(4.0)
		a.F4 = TestVariantsTestC5B(5.0)
		a.F5 = TestVariantsTestC6B(6.0)
		res := TestVariantsTestVariantCasts(a)
		if res.F0.GetB() != 1 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F1.GetB() != 2.0 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F2.GetB() != 3.0 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F3.GetB() != 4.0 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F4.GetB() != 5.0 {
			panic("TestVariantsTestVariantCasts")
		}
		if res.F5.GetB() != 6.0 {
			panic("TestVariantsTestVariantCasts")
		}
	}
	{
		var a TestVariantsTestZeros
		a.F0 = TestVariantsTestZ1A(1)
		a.F1 = TestVariantsTestZ2A(2)
		a.F2 = TestVariantsTestZ3A(3.0)
		a.F3 = TestVariantsTestZ4A(4.0)
		res := TestVariantsTestVariantZeros(a)
		if res.F0.GetA() != 1 {
			panic("TestVariantsTestVariantZeros")
		}
		if res.F1.GetA() != 2 {
			panic("TestVariantsTestVariantZeros")
		}
		if res.F2.GetA() != 3.0 {
			panic("TestVariantsTestVariantZeros")
		}
		if res.F3.GetA() != 4.0 {
			panic("TestVariantsTestVariantZeros")
		}
	}
	{
		var a TestVariantsTestZeros
		a.F0 = TestVariantsTestZ1B()
		a.F1 = TestVariantsTestZ2B()
		a.F2 = TestVariantsTestZ3B()
		a.F3 = TestVariantsTestZ4B()
		res := TestVariantsTestVariantZeros(a)
		if res.F0.Kind() != TestVariantsTestZ1KindB {
			panic("TestVariantsTestVariantZeros")
		}
		if res.F1.Kind() != TestVariantsTestZ2KindB {
			panic("TestVariantsTestVariantZeros")
		}
		if res.F2.Kind() != TestVariantsTestZ3KindB {
			panic("TestVariantsTestVariantZeros")
		}
		if res.F3.Kind() != TestVariantsTestZ4KindB {
			panic("TestVariantsTestVariantZeros")
		}
	}
	{
		var res Result[uint32, struct{}]
		res.SetErr(struct{}{})
		TestVariantsTestVariantTypedefs(None[uint32](), false, res)
	}
	{
		var param Result[struct{}, struct{}]
		param.Set(struct{}{})
		res := TestVariantsTestVariantEnums(true, param, TestVariantsTestMyErrnoSuccess())
		if res.F0 != false {
			panic("TestVariantsTestVariantEnums")
		}
		if res.F1.IsOk() {
			panic("TestVariantsTestVariantEnums")
		}
		if res.F2.Kind() != TestVariantsTestMyErrnoKindA {
			panic("TestVariantsTestVariantEnums")
		}
	}

}

func (i VariantsImpl) RoundtripOption(a Option[float32]) Option[uint8] {
	if a.IsNone() {
		return None[uint8]()
	} else {
		return Some[uint8](uint8(a.Unwrap()))
	}
}

func (i VariantsImpl) RoundtripResult(a Result[uint32, float32]) Result[float64, uint8] {
	var res Result[float64, uint8]
	if a.IsOk() {
		res.Set(float64(a.Unwrap()))
	} else {
		res.SetErr(uint8(a.UnwrapErr()))
	}
	return res
}

func (i VariantsImpl) RoundtripEnum(a TestVariantsTestE1) TestVariantsTestE1 {
	return a
}

func (i VariantsImpl) InvertBool(a bool) bool {
	return !a
}

func (i VariantsImpl) VariantCasts(a TestVariantsTestCasts) TestVariantsTestCasts {
	if a.F0.Kind() == TestVariantsTestC1KindA {
		if a.F0.GetA() != 1 {
			panic("TestVariantsTestVariantCasts")
		}
	}
	return a
}

func (i VariantsImpl) VariantZeros(a TestVariantsTestZeros) TestVariantsTestZeros {
	return a
}

func (i VariantsImpl) VariantTypedefs(a Option[uint32], b bool, c Result[uint32, struct{}]) {

}

func (i VariantsImpl) VariantEnums(a bool, b Result[struct{}, struct{}], c TestVariantsTestMyErrno) TestVariantsTestTuple3BoolResultEmptyEmptyTTestVariantsTestMyErrnoT {
	return TestVariantsTestTuple3BoolResultEmptyEmptyTTestVariantsTestMyErrnoT{a, b, c}
}

func main() {}
