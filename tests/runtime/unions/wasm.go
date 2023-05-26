package main

import (
	"math"
	. "wit_unions_go/gen"
)

func init() {
	n := UnionsImpl{}
	SetUnions(n)
	SetExportsTestUnionsTest(n)
}

type UnionsImpl struct{}

func (s UnionsImpl) TestImports() {
	res1 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF0(false))
	if res1.GetF0() != true {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res2 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF0(true))
	if res2.GetF0() != false {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res3 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF1(0))
	if res3.GetF1() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res4 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF1(math.MaxUint8))
	if res4.GetF1() != 0 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res5 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF2(0))
	if res5.GetF2() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res6 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF2(math.MaxUint16))
	if res6.GetF2() != 0 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res7 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF3(0))
	if res7.GetF3() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res8 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF3(math.MaxUint32))
	if res8.GetF3() != 0 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res9 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF4(0))
	if res9.GetF4() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res10 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF4(math.MaxUint64))
	if res10.GetF4() != 0 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res11 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF5(0))
	if res11.GetF5() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res12 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF5(math.MaxInt8))
	if res12.GetF5() != math.MinInt8 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res13 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF6(0))
	if res13.GetF6() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res14 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF6(math.MaxInt16))
	if res14.GetF6() != math.MinInt16 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res15 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF7(0))
	if res15.GetF7() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res16 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF7(math.MaxInt32))
	if res16.GetF7() != math.MinInt32 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res17 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF8(0))
	if res17.GetF8() != 1 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	res18 := TestUnionsTestAddOneInteger(TestUnionsTestAllIntegersF8(math.MaxInt64))
	if res18.GetF8() != math.MinInt64 {
		panic("unexpect result in TestUnionsTestAddOneInteger")
	}

	// All Floats
	res19 := TestUnionsTestAddOneFloat(TestUnionsTestAllFloatsF0(0.0))
	if res19.GetF0() != 1.0 {
		panic("unexpect result in TestUnionsTestAddOneFloat")
	}

	res20 := TestUnionsTestAddOneFloat(TestUnionsTestAllFloatsF1(0.0))
	if res20.GetF1() != 1.0 {
		panic("unexpect result in TestUnionsTestAddOneFloat")
	}

	// All Text
	if TestUnionsTestReplaceFirstChar(TestUnionsTestAllTextF0('a'), 'z').GetF0() != 'z' {
		panic("unexpect result in TestUnionsTestReplaceFirstChar")
	}
	if TestUnionsTestReplaceFirstChar(TestUnionsTestAllTextF1("abc"), 'z').GetF1() != "zbc" {
		panic("unexpect result in TestUnionsTestReplaceFirstChar")
	}

	// All Integers
	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF0(true)) != 0 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF1(0)) != 1 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF2(0)) != 2 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF3(0)) != 3 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF4(0)) != 4 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF5(0)) != 5 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF6(0)) != 6 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF7(0)) != 7 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyInteger(TestUnionsTestAllIntegersF8(0)) != 8 {
		panic("unexpect result in TestUnionsTestIdentifyInteger")
	}

	if TestUnionsTestIdentifyFloat(TestUnionsTestAllFloatsF0(0.0)) != 0 {
		panic("unexpect result in TestUnionsTestIdentifyFloat")
	}

	if TestUnionsTestIdentifyFloat(TestUnionsTestAllFloatsF1(0.0)) != 1 {
		panic("unexpect result in TestUnionsTestIdentifyFloat")
	}

	// All Text
	if TestUnionsTestIdentifyText(TestUnionsTestAllTextF0('a')) != 0 {
		panic("unexpected result in TestUnionsTestIdentifyText")
	}
	if TestUnionsTestIdentifyText(TestUnionsTestAllTextF1("abc")) != 1 {
		panic("unexpected result in TestUnionsTestIdentifyText")
	}

	// All Duplicated
	res21 := TestUnionsTestAddOneDuplicated(TestUnionsTestDuplicatedS32F0(0))
	if res21.GetF0() != 1 {
		panic("unexpected result in TestUnionsTestAddOneDuplicated")
	}

	res22 := TestUnionsTestAddOneDuplicated(TestUnionsTestDuplicatedS32F1(1))
	if res22.GetF1() != 2 {
		panic("unexpected result in TestUnionsTestAddOneDuplicated")
	}

	res23 := TestUnionsTestAddOneDuplicated(TestUnionsTestDuplicatedS32F2(2))
	if res23.GetF2() != 3 {
		panic("unexpected result in TestUnionsTestAddOneDuplicated")
	}

	// Distinguishable

	if TestUnionsTestAddOneDistinguishableNum(TestUnionsTestDistinguishableNumF1(0)).GetF1() != 1 {
		panic("unexpect result in TestUnionsTestAddOneDistinguishableNum")
	}

	if TestUnionsTestIdentifyDistinguishableNum(TestUnionsTestDistinguishableNumF0(0.0)) != 0 {
		panic("unexpect result in TestUnionsTestIdentifyDistinguishableNum")
	}

	if TestUnionsTestIdentifyDistinguishableNum(TestUnionsTestDistinguishableNumF1(1)) != 1 {
		panic("unexpect result in TestUnionsTestIdentifyDistinguishableNum")
	}

}

func (u UnionsImpl) AddOneInteger(num ExportsTestUnionsTestAllIntegers) ExportsTestUnionsTestAllIntegers {
	switch num.Kind() {
	case ExportsTestUnionsTestAllIntegersKindF0:
		return ExportsTestUnionsTestAllIntegersF0(!num.GetF0())
	case ExportsTestUnionsTestAllIntegersKindF1:
		return ExportsTestUnionsTestAllIntegersF1(num.GetF1() + 1)
	case ExportsTestUnionsTestAllIntegersKindF2:
		return ExportsTestUnionsTestAllIntegersF2(num.GetF2() + 1)
	case ExportsTestUnionsTestAllIntegersKindF3:
		return ExportsTestUnionsTestAllIntegersF3(num.GetF3() + 1)
	case ExportsTestUnionsTestAllIntegersKindF4:
		return ExportsTestUnionsTestAllIntegersF4(num.GetF4() + 1)
	case ExportsTestUnionsTestAllIntegersKindF5:
		return ExportsTestUnionsTestAllIntegersF5(num.GetF5() + 1)
	case ExportsTestUnionsTestAllIntegersKindF6:
		return ExportsTestUnionsTestAllIntegersF6(num.GetF6() + 1)
	case ExportsTestUnionsTestAllIntegersKindF7:
		return ExportsTestUnionsTestAllIntegersF7(num.GetF7() + 1)
	case ExportsTestUnionsTestAllIntegersKindF8:
		return ExportsTestUnionsTestAllIntegersF8(num.GetF8() + 1)
	default:
		panic("unexpected type in ExportsTestUnionsTestAllIntegers")
	}
}

func (u UnionsImpl) AddOneFloat(num ExportsTestUnionsTestAllFloats) ExportsTestUnionsTestAllFloats {
	switch num.Kind() {
	case ExportsTestUnionsTestAllFloatsKindF0:
		return ExportsTestUnionsTestAllFloatsF0(num.GetF0() + 1.0)
	case ExportsTestUnionsTestAllFloatsKindF1:
		return ExportsTestUnionsTestAllFloatsF1(num.GetF1() + 1.0)
	default:
		panic("unexpected type in ExportsTestUnionsTestAllFloats")
	}
}

func (u UnionsImpl) ReplaceFirstChar(text ExportsTestUnionsTestAllText, letter rune) ExportsTestUnionsTestAllText {
	switch text.Kind() {
	case ExportsTestUnionsTestAllTextKindF0:
		return ExportsTestUnionsTestAllTextF0(letter)
	case ExportsTestUnionsTestAllTextKindF1:
		return ExportsTestUnionsTestAllTextF1(string(letter) + text.GetF1()[1:])
	default:
		// handle error or panic
		return ExportsTestUnionsTestAllText{}
	}
}

func (u UnionsImpl) IdentifyInteger(num ExportsTestUnionsTestAllIntegers) uint8 {
	switch num.Kind() {
	case ExportsTestUnionsTestAllIntegersKindF0:
		return 0
	case ExportsTestUnionsTestAllIntegersKindF1:
		return 1
	case ExportsTestUnionsTestAllIntegersKindF2:
		return 2
	case ExportsTestUnionsTestAllIntegersKindF3:
		return 3
	case ExportsTestUnionsTestAllIntegersKindF4:
		return 4
	case ExportsTestUnionsTestAllIntegersKindF5:
		return 5
	case ExportsTestUnionsTestAllIntegersKindF6:
		return 6
	case ExportsTestUnionsTestAllIntegersKindF7:
		return 7
	case ExportsTestUnionsTestAllIntegersKindF8:
		return 8
	default:
		panic("unexpected type in ExportsTestUnionsTestAllIntegers")
	}
}

func (u UnionsImpl) IdentifyFloat(num ExportsTestUnionsTestAllFloats) uint8 {
	switch num.Kind() {
	case ExportsTestUnionsTestAllFloatsKindF0:
		return 0
	case ExportsTestUnionsTestAllFloatsKindF1:
		return 1
	default:
		panic("unexpected type in ExportsTestUnionsTestAllFloats")
	}
}

func (u UnionsImpl) IdentifyText(text ExportsTestUnionsTestAllText) uint8 {
	switch text.Kind() {
	case ExportsTestUnionsTestAllTextKindF0:
		return 0
	case ExportsTestUnionsTestAllTextKindF1:
		return 1
	default:
		panic("unexpected type in ExportsTestUnionsTestAllText")
	}
}

func (u UnionsImpl) AddOneDuplicated(num ExportsTestUnionsTestDuplicatedS32) ExportsTestUnionsTestDuplicatedS32 {
	switch num.Kind() {
	case ExportsTestUnionsTestDuplicatedS32KindF0:
		return ExportsTestUnionsTestDuplicatedS32F0(num.GetF0() + 1)
	case ExportsTestUnionsTestDuplicatedS32KindF1:
		return ExportsTestUnionsTestDuplicatedS32F1(num.GetF1() + 1)
	case ExportsTestUnionsTestDuplicatedS32KindF2:
		return ExportsTestUnionsTestDuplicatedS32F2(num.GetF2() + 1)
	default:
		panic("unexpected type in ExportsTestUnionsTestDuplicatedS32")
	}
}

func (u UnionsImpl) IdentifyDuplicated(num ExportsTestUnionsTestDuplicatedS32) uint8 {
	switch num.Kind() {
	case ExportsTestUnionsTestDuplicatedS32KindF0:
		return 0
	case ExportsTestUnionsTestDuplicatedS32KindF1:
		return 1
	case ExportsTestUnionsTestDuplicatedS32KindF2:
		return 2
	default:
		panic("unexpected type in IdentifyDuplicated")
	}
}

func (u UnionsImpl) AddOneDistinguishableNum(num ExportsTestUnionsTestDistinguishableNum) ExportsTestUnionsTestDistinguishableNum {
	switch num.Kind() {
	case ExportsTestUnionsTestDistinguishableNumKindF0:
		return ExportsTestUnionsTestDistinguishableNumF0(num.GetF0() + 1.0)
	case ExportsTestUnionsTestDistinguishableNumKindF1:
		return ExportsTestUnionsTestDistinguishableNumF1(num.GetF1() + 1)
	default:
		panic("unexpected type in ExportsTestUnionsTestDistinguishableNum")
	}
}

func (u UnionsImpl) IdentifyDistinguishableNum(num ExportsTestUnionsTestDistinguishableNum) uint8 {
	switch num.Kind() {
	case ExportsTestUnionsTestDistinguishableNumKindF0:
		return 0
	case ExportsTestUnionsTestDistinguishableNumKindF1:
		return 1
	default:
		panic("unexpected type in ExportsTestUnionsTestDistinguishableNum")
	}
}

func main() {}
