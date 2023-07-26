package main

import (
	. "wit_records_go/gen"
)

func init() {
	n := &RecordImpl{}
	SetRecords(n)
	SetExportsTestRecordsTest(n)
}

type RecordImpl struct{}

func (r *RecordImpl) TestImports() {
	a, b := TestRecordsTestMultipleResults()
	if a != 4 && b != 5 {
		panic("TestRecordsTestMultipleResults")
	}
	t := TestRecordsTestSwapTuple(TestRecordsTestTuple2U8U32T{1, 2})
	if t.F0 != 2 && t.F1 != 1 {
		panic("TestRecordsTestSwapTuple")
	}

	// TODO: how to handle empty flags?
	if TestRecordsTestRoundtripFlags1(TestRecordsTestF1_A) != TestRecordsTestF1_A {
		panic("TestRecordsTestRoundtripFlags1")
	}
	if TestRecordsTestRoundtripFlags1(TestRecordsTestF1_B) != TestRecordsTestF1_B {
		panic("TestRecordsTestRoundtripFlags1")
	}
	if TestRecordsTestRoundtripFlags1(TestRecordsTestF1_A|TestRecordsTestF1_B) != TestRecordsTestF1_A|TestRecordsTestF1_B {
		panic("TestRecordsTestRoundtripFlags1")
	}

	if TestRecordsTestRoundtripFlags2(TestRecordsTestF2_C) != TestRecordsTestF2_C {
		panic("TestRecordsTestRoundtripFlags2")
	}
	if TestRecordsTestRoundtripFlags2(TestRecordsTestF2_D) != TestRecordsTestF2_D {
		panic("TestRecordsTestRoundtripFlags2")
	}
	if TestRecordsTestRoundtripFlags2(TestRecordsTestF2_C|TestRecordsTestF2_E) != TestRecordsTestF2_C|TestRecordsTestF2_E {
		panic("TestRecordsTestRoundtripFlags2")
	}

	if a, b, c, d := TestRecordsTestRoundtripFlags3(TestRecordsTestFlag8_B0, TestRecordsTestFlag16_B1, TestRecordsTestFlag32_B2, TestRecordsTestFlag64_B3); a != TestRecordsTestFlag8_B0 && b != TestRecordsTestFlag16_B1 && c != TestRecordsTestFlag32_B2 && d != TestRecordsTestFlag64_B3 {
		panic("TestRecordsTestRoundtripFlags3")
	}

	r1 := TestRecordsTestRoundtripRecord1(TestRecordsTestR1{8, TestRecordsTestF1_A})
	if r1.A != 8 && r1.B != TestRecordsTestF1_A {
		panic("TestRecordsTestRoundtripRecord1")
	}

	r2 := TestRecordsTestRoundtripRecord1(TestRecordsTestR1{0, TestRecordsTestF1_A | TestRecordsTestF1_B})
	if r2.A != 0 && r2.B != TestRecordsTestF1_A|TestRecordsTestF1_B {
		panic("TestRecordsTestRoundtripRecord1")
	}

	if TestRecordsTestTuple1(TestRecordsTestTuple1U8T{1}).F0 != 1 {
		panic("TestRecordsTestTuple1")
	}
}

func (r *RecordImpl) MultipleResults() (uint8, uint16) {
	return 100, 200
}

func (r *RecordImpl) SwapTuple(a TestRecordsTestTuple2U8U32T) TestRecordsTestTuple2U32U8T {
	return TestRecordsTestTuple2U32U8T{a.F1, a.F0}
}

func (r *RecordImpl) RoundtripFlags1(a TestRecordsTestF1) TestRecordsTestF1 {
	return a
}

func (r *RecordImpl) RoundtripFlags2(a TestRecordsTestF2) TestRecordsTestF2 {
	return a
}

func (r *RecordImpl) RoundtripFlags3(a TestRecordsTestFlag8, b TestRecordsTestFlag16, c TestRecordsTestFlag32, d TestRecordsTestFlag64) (TestRecordsTestFlag8, TestRecordsTestFlag16, TestRecordsTestFlag32, TestRecordsTestFlag64) {
	return a, b, c, d
}

func (r *RecordImpl) RoundtripRecord1(a TestRecordsTestR1) TestRecordsTestR1 {
	return a
}

func (r *RecordImpl) Tuple1(a TestRecordsTestTuple1U8T) TestRecordsTestTuple1U8T {
	return a
}

func main() {}
