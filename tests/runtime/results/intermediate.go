package export_test_results_test

import (
	imports "wit_component/test_results_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func StringError(x float32) Result[float32, string] {
	return imports.StringError(x)
}

func EnumError(x float32) Result[float32, imports.E] {
	return imports.EnumError(x)
}

func RecordError(x float32) Result[float32, imports.E2] {
	return imports.RecordError(x)
}

func VariantError(x float32) Result[float32, imports.E3] {
	return imports.VariantError(x)
}

func EmptyError(x uint32) Result[uint32, Unit] {
	return imports.EmptyError(x)
}

func DoubleError(x uint32) Result[Result[Unit, string], string] {
	return imports.DoubleError(x)
}
