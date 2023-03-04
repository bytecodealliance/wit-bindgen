package main

import (
	. "wit_variants_go/gen"
)

func init() {
	a := VariantsImpl{}
	SetExports(a)
	SetVariants(a)
}

type VariantsImpl struct {
}

func (i VariantsImpl) TestImports() {
	{
		res := ImportsRoundtripOption(Some[float32](1))
		if res.IsNone() {
			panic("ImportsRoundtripOption")
		}
		if res.Unwrap() != 1.0 {
			panic("ImportsRoundtripOption")
		}

		res2 := ImportsRoundtripOption(None[float32]())
		if res2.IsSome() {
			panic("ImportsRoundtripOption")
		}
	}
	{
		var param Result[uint32, float32]
		param.Set(5)
		res := ImportsRoundtripResult(param)
		if res.IsErr() {
			panic("ImportsRoundtripResult")
		}
		if res.Unwrap() != 5 {
			panic("ImportsRoundtripResult")
		}

		param.SetErr(10.0)
		res2 := ImportsRoundtripResult(param)
		if res2.IsOk() {
			panic("ImportsRoundtripResult")
		}
		if res2.UnwrapErr() != 10.0 {
			panic("ImportsRoundtripResult")
		}
	}

	{
		a := ImportsE1A()
		res := ImportsRoundtripEnum(a)
		if res.Kind() != ImportsE1KindA {
			panic("ImportsRoundtripEnum")
		}

		b := ImportsE1B()
		res2 := ImportsRoundtripEnum(b)
		if res2.Kind() != ImportsE1KindB {
			panic("ImportsRoundtripEnum")
		}
	}
	{
		if ImportsInvertBool(true) != false {
			panic("ImportsRoundtripBool")
		}
		if ImportsInvertBool(false) != true {
			panic("ImportsRoundtripBool")
		}
	}
	{
		var a ImportsCasts
		a.F0 = ImportsC1A(1)
		a.F1 = ImportsC2A(2)
		a.F2 = ImportsC3A(3)
		a.F3 = ImportsC4A(4)
		a.F4 = ImportsC5A(5)
		a.F5 = ImportsC6A(6.0)
		res := ImportsVariantCasts(a)
		if res.F0.GetA() != 1 {
			panic("ImportsVariantCasts")
		}
		if res.F1.GetA() != 2 {
			panic("ImportsVariantCasts")
		}
		if res.F2.GetA() != 3 {
			panic("ImportsVariantCasts")
		}
		if res.F3.GetA() != 4 {
			panic("ImportsVariantCasts")
		}
		if res.F4.GetA() != 5 {
			panic("ImportsVariantCasts")
		}
		if res.F5.GetA() != 6.0 {
			panic("ImportsVariantCasts")
		}
	}
	{
		var a ImportsCasts
		a.F0 = ImportsC1B(1)
		a.F1 = ImportsC2B(2.0)
		a.F2 = ImportsC3B(3.0)
		a.F3 = ImportsC4B(4.0)
		a.F4 = ImportsC5B(5.0)
		a.F5 = ImportsC6B(6.0)
		res := ImportsVariantCasts(a)
		if res.F0.GetB() != 1 {
			panic("ImportsVariantCasts")
		}
		if res.F1.GetB() != 2.0 {
			panic("ImportsVariantCasts")
		}
		if res.F2.GetB() != 3.0 {
			panic("ImportsVariantCasts")
		}
		if res.F3.GetB() != 4.0 {
			panic("ImportsVariantCasts")
		}
		if res.F4.GetB() != 5.0 {
			panic("ImportsVariantCasts")
		}
		if res.F5.GetB() != 6.0 {
			panic("ImportsVariantCasts")
		}
	}
	{
		var a ImportsZeros
		a.F0 = ImportsZ1A(1)
		a.F1 = ImportsZ2A(2)
		a.F2 = ImportsZ3A(3.0)
		a.F3 = ImportsZ4A(4.0)
		res := ImportsVariantZeros(a)
		if res.F0.GetA() != 1 {
			panic("ImportsVariantZeros")
		}
		if res.F1.GetA() != 2 {
			panic("ImportsVariantZeros")
		}
		if res.F2.GetA() != 3.0 {
			panic("ImportsVariantZeros")
		}
		if res.F3.GetA() != 4.0 {
			panic("ImportsVariantZeros")
		}
	}
	{
		var a ImportsZeros
		a.F0 = ImportsZ1B()
		a.F1 = ImportsZ2B()
		a.F2 = ImportsZ3B()
		a.F3 = ImportsZ4B()
		res := ImportsVariantZeros(a)
		if res.F0.Kind() != ImportsZ1KindB {
			panic("ImportsVariantZeros")
		}
		if res.F1.Kind() != ImportsZ2KindB {
			panic("ImportsVariantZeros")
		}
		if res.F2.Kind() != ImportsZ3KindB {
			panic("ImportsVariantZeros")
		}
		if res.F3.Kind() != ImportsZ4KindB {
			panic("ImportsVariantZeros")
		}
	}
	{
		var res Result[uint32, struct{}]
		res.SetErr(struct{}{})
		ImportsVariantTypedefs(None[uint32](), false, res)
	}
	{
		var param Result[struct{}, struct{}]
		param.Set(struct{}{})
		res := ImportsVariantEnums(true, param, ImportsMyErrnoSuccess())
		if res.F0 != false {
			panic("ImportsVariantEnums")
		}
		if res.F1.IsOk() {
			panic("ImportsVariantEnums")
		}
		if res.F2.Kind() != ImportsMyErrnoKindA {
			panic("ImportsVariantEnums")
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

func (i VariantsImpl) RoundtripEnum(a ExportsE1) ExportsE1 {
	return a
}

func (i VariantsImpl) InvertBool(a bool) bool {
	return !a
}

func (i VariantsImpl) VariantCasts(a ExportsCasts) ExportsCasts {
	if a.F0.Kind() == ExportsC1KindA {
		println(a.F0.GetA())
	}
	return a
}

func (i VariantsImpl) VariantZeros(a ExportsZeros) ExportsZeros {
	return a
}

func (i VariantsImpl) VariantTypedefs(a Option[uint32], b bool, c Result[uint32, struct{}]) {

}

func (i VariantsImpl) VariantEnums(a bool, b Result[struct{}, struct{}], c ExportsMyErrno) ExportsTuple3BoolResultEmptyEmptyTExportsMyErrnoT {
	return ExportsTuple3BoolResultEmptyEmptyTExportsMyErrnoT{a, b, c}
}

func main() {}
