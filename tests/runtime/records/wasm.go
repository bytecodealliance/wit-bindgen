package main

import (
	. "wit_records_go/gen"
)

func init() {
	n := &RecordImpl{}
	SetRecords(n)
	SetExports(n)
}

type RecordImpl struct{}

func (r *RecordImpl) TestImports() {
	a, b := ImportsMultipleResults()
	if a != 4 && b != 5 {
		panic("ImportsMultipleResults")
	}
	t := ImportsSwapTuple(ImportsTuple2U8U32T{1, 2})
	if t.F0 != 2 && t.F1 != 1 {
		panic("ImportsSwapTuple")
	}

	// TODO: how to handle empty flags?
	if ImportsRoundtripFlags1(ImportsF1_A) != ImportsF1_A {
		panic("ImportsRoundtripFlags1")
	}
	if ImportsRoundtripFlags1(ImportsF1_B) != ImportsF1_B {
		panic("ImportsRoundtripFlags1")
	}
	if ImportsRoundtripFlags1(ImportsF1_A|ImportsF1_B) != ImportsF1_A|ImportsF1_B {
		panic("ImportsRoundtripFlags1")
	}

	if ImportsRoundtripFlags2(ImportsF2_C) != ImportsF2_C {
		panic("ImportsRoundtripFlags2")
	}
	if ImportsRoundtripFlags2(ImportsF2_D) != ImportsF2_D {
		panic("ImportsRoundtripFlags2")
	}
	if ImportsRoundtripFlags2(ImportsF2_C|ImportsF2_E) != ImportsF2_C|ImportsF2_E {
		panic("ImportsRoundtripFlags2")
	}

	if a, b, c, d := ImportsRoundtripFlags3(ImportsFlag8_B0, ImportsFlag16_B1, ImportsFlag32_B2, ImportsFlag64_B3); a != ImportsFlag8_B0 && b != ImportsFlag16_B1 && c != ImportsFlag32_B2 && d != ImportsFlag64_B3 {
		panic("ImportsRoundtripFlags3")
	}

	r1 := ImportsRoundtripRecord1(ImportsR1{8, ImportsF1_A})
	if r1.A != 8 && r1.B != ImportsF1_A {
		panic("ImportsRoundtripRecord1")
	}

	r2 := ImportsRoundtripRecord1(ImportsR1{0, ImportsF1_A | ImportsF1_B})
	if r2.A != 0 && r2.B != ImportsF1_A|ImportsF1_B {
		panic("ImportsRoundtripRecord1")
	}

	ImportsTuple0(ImportsTuple0T{})
	if ImportsTuple1(ImportsTuple1U8T{1}).F0 != 1 {
		panic("ImportsTuple0")
	}
}

func (r *RecordImpl) MultipleResults() (uint8, uint16) {
	return 100, 200
}

func (r *RecordImpl) SwapTuple(a ExportsTuple2U8U32T) ExportsTuple2U32U8T {
	return ExportsTuple2U32U8T{a.F1, a.F0}
}

func (r *RecordImpl) RoundtripFlags1(a ExportsF1) ExportsF1 {
	return a
}

func (r *RecordImpl) RoundtripFlags2(a ExportsF2) ExportsF2 {
	return a
}

func (r *RecordImpl) RoundtripFlags3(a ExportsFlag8, b ExportsFlag16, c ExportsFlag32, d ExportsFlag64) (ExportsFlag8, ExportsFlag16, ExportsFlag32, ExportsFlag64) {
	return a, b, c, d
}

func (r *RecordImpl) RoundtripRecord1(a ExportsR1) ExportsR1 {
	return a
}

func (r *RecordImpl) Tuple0(a ExportsTuple0T) ExportsTuple0T {
	return a
}

func (r *RecordImpl) Tuple1(a ExportsTuple1U8T) ExportsTuple1U8T {
	return a
}

func main() {}
