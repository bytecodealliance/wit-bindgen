package export_test_variants_to_test

import (
	. "wit_component/test_variants_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func RoundtripOption(x Option[float32]) Option[uint8] {
	switch x.Tag() {
	case OptionSome:
		return Some[uint8](uint8(x.Some()))
	case OptionNone:
		return None[uint8]()
	default:
		panic("unreachable")
	}
}

func RoundtripResult(x Result[uint32, float32]) Result[float64, uint8] {
	switch x.Tag() {
	case ResultOk:
		return Ok[float64, uint8](float64(x.Ok()))
	case ResultErr:
		return Err[float64, uint8](uint8(x.Err()))
	default:
		panic("unreachable")
	}
}

func RoundtripEnum(x E1) E1 {
	return x
}

func InvertBool(x bool) bool {
	return !x
}

func VariantCasts(x Casts) (C1, C2, C3, C4, C5, C6) {
	return x.F0, x.F1, x.F2, x.F3, x.F4, x.F5
}

func VariantZeros(x Zeros) (Z1, Z2, Z3, Z4) {
	return x.F0, x.F1, x.F2, x.F3
}

func VariantTypedefs(x Option[uint32], y bool, z Result[uint32, Unit]) {
}

func VariantEnums(x bool, y Result[Unit, Unit], z MyErrno) (bool, Result[Unit, Unit], MyErrno) {
	return x, y, z
}
