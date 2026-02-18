package export_test_results_test

import (
	. "wit_component/test_results_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func StringError(x float32) Result[float32, string] {
	if x == 0.0 {
		return Err[float32, string]("zero")
	} else {
		return Ok[float32, string](x)
	}
}

func EnumError(x float32) Result[float32, E] {
	if x == 0.0 {
		return Err[float32, E](EA)
	} else {
		return Ok[float32, E](x)
	}
}

func RecordError(x float32) Result[float32, E2] {
	if x == 0.0 {
		return Err[float32, E2](E2{420, 0})
	} else if x == 1.0 {
		return Err[float32, E2](E2{77, 2})
	} else {
		return Ok[float32, E2](x)
	}
}

func VariantError(x float32) Result[float32, E3] {
	if x == 0.0 {
		return Err[float32, E3](MakeE3E2(E2{420, 0}))
	} else if x == 1.0 {
		return Err[float32, E3](MakeE3E1(EB))
	} else if x == 2.0 {
		return Err[float32, E3](MakeE3E1(EC))
	} else {
		return Ok[float32, E3](x)
	}
}

func EmptyError(x uint32) Result[uint32, Unit] {
	if x == 0 {
		return Err[uint32, Unit](Unit{})
	} else if x == 1 {
		return Ok[uint32, Unit](42)
	} else {
		return Ok[uint32, Unit](x)
	}
}

func DoubleError(x uint32) Result[Result[Unit, string], string] {
	if x == 0 {
		return Ok[Result[Unit, string], string](
			Ok[Unit, string](Unit{}),
		)
	} else if x == 1 {
		return Ok[Result[Unit, string], string](
			Err[Unit, string]("one"),
		)
	} else {
		return Err[Result[Unit, string], string](
			"two",
		)
	}
}
