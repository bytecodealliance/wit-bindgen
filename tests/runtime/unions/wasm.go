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

func (u UnionsImpl) AddOneInteger(num TestUnionsTestAllIntegers) TestUnionsTestAllIntegers {
	switch num.Kind() {
	case TestUnionsTestAllIntegersKindF0:
		return TestUnionsTestAllIntegersF0(!num.GetF0())
	case TestUnionsTestAllIntegersKindF1:
		return TestUnionsTestAllIntegersF1(num.GetF1() + 1)
	case TestUnionsTestAllIntegersKindF2:
		return TestUnionsTestAllIntegersF2(num.GetF2() + 1)
	case TestUnionsTestAllIntegersKindF3:
		return TestUnionsTestAllIntegersF3(num.GetF3() + 1)
	case TestUnionsTestAllIntegersKindF4:
		return TestUnionsTestAllIntegersF4(num.GetF4() + 1)
	case TestUnionsTestAllIntegersKindF5:
		return TestUnionsTestAllIntegersF5(num.GetF5() + 1)
	case TestUnionsTestAllIntegersKindF6:
		return TestUnionsTestAllIntegersF6(num.GetF6() + 1)
	case TestUnionsTestAllIntegersKindF7:
		return TestUnionsTestAllIntegersF7(num.GetF7() + 1)
	case TestUnionsTestAllIntegersKindF8:
		return TestUnionsTestAllIntegersF8(num.GetF8() + 1)
	default:
		panic("unexpected type in TestUnionsTestAllIntegers")
	}
}

func (u UnionsImpl) AddOneFloat(num TestUnionsTestAllFloats) TestUnionsTestAllFloats {
	switch num.Kind() {
	case TestUnionsTestAllFloatsKindF0:
		return TestUnionsTestAllFloatsF0(num.GetF0() + 1.0)
	case TestUnionsTestAllFloatsKindF1:
		return TestUnionsTestAllFloatsF1(num.GetF1() + 1.0)
	default:
		panic("unexpected type in TestUnionsTestAllFloats")
	}
}

func (u UnionsImpl) ReplaceFirstChar(text TestUnionsTestAllText, letter rune) TestUnionsTestAllText {
	switch text.Kind() {
	case TestUnionsTestAllTextKindF0:
		return TestUnionsTestAllTextF0(letter)
	case TestUnionsTestAllTextKindF1:
		return TestUnionsTestAllTextF1(string(letter) + text.GetF1()[1:])
	default:
		// handle error or panic
		return TestUnionsTestAllText{}
	}
}

func (u UnionsImpl) IdentifyInteger(num TestUnionsTestAllIntegers) uint8 {
	switch num.Kind() {
	case TestUnionsTestAllIntegersKindF0:
		return 0
	case TestUnionsTestAllIntegersKindF1:
		return 1
	case TestUnionsTestAllIntegersKindF2:
		return 2
	case TestUnionsTestAllIntegersKindF3:
		return 3
	case TestUnionsTestAllIntegersKindF4:
		return 4
	case TestUnionsTestAllIntegersKindF5:
		return 5
	case TestUnionsTestAllIntegersKindF6:
		return 6
	case TestUnionsTestAllIntegersKindF7:
		return 7
	case TestUnionsTestAllIntegersKindF8:
		return 8
	default:
		panic("unexpected type in TestUnionsTestAllIntegers")
	}
}

func (u UnionsImpl) IdentifyFloat(num TestUnionsTestAllFloats) uint8 {
	switch num.Kind() {
	case TestUnionsTestAllFloatsKindF0:
		return 0
	case TestUnionsTestAllFloatsKindF1:
		return 1
	default:
		panic("unexpected type in TestUnionsTestAllFloats")
	}
}

func (u UnionsImpl) IdentifyText(text TestUnionsTestAllText) uint8 {
	switch text.Kind() {
	case TestUnionsTestAllTextKindF0:
		return 0
	case TestUnionsTestAllTextKindF1:
		return 1
	default:
		panic("unexpected type in TestUnionsTestAllText")
	}
}

func (u UnionsImpl) AddOneDuplicated(num TestUnionsTestDuplicatedS32) TestUnionsTestDuplicatedS32 {
	switch num.Kind() {
	case TestUnionsTestDuplicatedS32KindF0:
		return TestUnionsTestDuplicatedS32F0(num.GetF0() + 1)
	case TestUnionsTestDuplicatedS32KindF1:
		return TestUnionsTestDuplicatedS32F1(num.GetF1() + 1)
	case TestUnionsTestDuplicatedS32KindF2:
		return TestUnionsTestDuplicatedS32F2(num.GetF2() + 1)
	default:
		panic("unexpected type in TestUnionsTestDuplicatedS32")
	}
}

func (u UnionsImpl) IdentifyDuplicated(num TestUnionsTestDuplicatedS32) uint8 {
	switch num.Kind() {
	case TestUnionsTestDuplicatedS32KindF0:
		return 0
	case TestUnionsTestDuplicatedS32KindF1:
		return 1
	case TestUnionsTestDuplicatedS32KindF2:
		return 2
	default:
		panic("unexpected type in IdentifyDuplicated")
	}
}

func (u UnionsImpl) AddOneDistinguishableNum(num TestUnionsTestDistinguishableNum) TestUnionsTestDistinguishableNum {
	switch num.Kind() {
	case TestUnionsTestDistinguishableNumKindF0:
		return TestUnionsTestDistinguishableNumF0(num.GetF0() + 1.0)
	case TestUnionsTestDistinguishableNumKindF1:
		return TestUnionsTestDistinguishableNumF1(num.GetF1() + 1)
	default:
		panic("unexpected type in TestUnionsTestDistinguishableNum")
	}
}

func (u UnionsImpl) IdentifyDistinguishableNum(num TestUnionsTestDistinguishableNum) uint8 {
	switch num.Kind() {
	case TestUnionsTestDistinguishableNumKindF0:
		return 0
	case TestUnionsTestDistinguishableNumKindF1:
		return 1
	default:
		panic("unexpected type in TestUnionsTestDistinguishableNum")
	}
}

func main() {}
