package export_test_records_to_test

import (
	. "wit_component/test_records_to_test"

	"go.bytecodealliance.org/wit-bindgen/wit_types"
)

func MultipleResults() (uint8, uint16) {
	return 4, 5
}

func SwapTuple(x wit_types.Tuple2[uint8, uint32]) (uint32, uint8) {
	return x.F1, x.F0
}

func RoundtripFlags1(x F1) F1 {
	return x
}

func RoundtripFlags2(x F2) F2 {
	return x
}

func RoundtripFlags3(x Flag8, y Flag16, z Flag32) (Flag8, Flag16, Flag32) {
	return x, y, z
}

func RoundtripRecord1(x R1) R1 {
	return x
}

func Tuple1(x wit_types.Tuple1[uint8]) uint8 {
	return x.F0
}
