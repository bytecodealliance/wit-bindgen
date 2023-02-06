package main

import (
	"math"
	. "wit_unions_go/gen"
)

func init() {
	n := UnionsImpl{}
	SetUnions(n)
	SetExports(n)
}

type UnionsImpl struct{}

func (s UnionsImpl) TestImports() {
	res1 := ImportsAddOneInteger(ImportsAllIntegersF0(false))
	if res1.GetF0() != true {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res2 := ImportsAddOneInteger(ImportsAllIntegersF0(true))
	if res2.GetF0() != false {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res3 := ImportsAddOneInteger(ImportsAllIntegersF1(0))
	if res3.GetF1() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res4 := ImportsAddOneInteger(ImportsAllIntegersF1(math.MaxUint8))
	if res4.GetF1() != 0 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res5 := ImportsAddOneInteger(ImportsAllIntegersF2(0))
	if res5.GetF2() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res6 := ImportsAddOneInteger(ImportsAllIntegersF2(math.MaxUint16))
	if res6.GetF2() != 0 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res7 := ImportsAddOneInteger(ImportsAllIntegersF3(0))
	if res7.GetF3() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res8 := ImportsAddOneInteger(ImportsAllIntegersF3(math.MaxUint32))
	if res8.GetF3() != 0 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res9 := ImportsAddOneInteger(ImportsAllIntegersF4(0))
	if res9.GetF4() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res10 := ImportsAddOneInteger(ImportsAllIntegersF4(math.MaxUint64))
	if res10.GetF4() != 0 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res11 := ImportsAddOneInteger(ImportsAllIntegersF5(0))
	if res11.GetF5() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res12 := ImportsAddOneInteger(ImportsAllIntegersF5(math.MaxInt8))
	if res12.GetF5() != math.MinInt8 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res13 := ImportsAddOneInteger(ImportsAllIntegersF6(0))
	if res13.GetF6() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res14 := ImportsAddOneInteger(ImportsAllIntegersF6(math.MaxInt16))
	if res14.GetF6() != math.MinInt16 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res15 := ImportsAddOneInteger(ImportsAllIntegersF7(0))
	if res15.GetF7() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res16 := ImportsAddOneInteger(ImportsAllIntegersF7(math.MaxInt32))
	if res16.GetF7() != math.MinInt32 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res17 := ImportsAddOneInteger(ImportsAllIntegersF8(0))
	if res17.GetF8() != 1 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	res18 := ImportsAddOneInteger(ImportsAllIntegersF8(math.MaxInt64))
	if res18.GetF8() != math.MinInt64 {
		panic("unexpect result in ImportsAddOneInteger")
	}

	// All Floats
	res19 := ImportsAddOneFloat(ImportsAllFloatsF0(0.0))
	if res19.GetF0() != 1.0 {
		panic("unexpect result in ImportsAddOneFloat")
	}

	res20 := ImportsAddOneFloat(ImportsAllFloatsF1(0.0))
	if res20.GetF1() != 1.0 {
		panic("unexpect result in ImportsAddOneFloat")
	}

	// All Text
	if ImportsReplaceFirstChar(ImportsAllTextF0('a'), 'z').GetF0() != 'z' {
		panic("unexpect result in ImportsReplaceFirstChar")
	}
	if ImportsReplaceFirstChar(ImportsAllTextF1("abc"), 'z').GetF1() != "zbc" {
		panic("unexpect result in ImportsReplaceFirstChar")
	}

	// All Integers
	if ImportsIdentifyInteger(ImportsAllIntegersF0(true)) != 0 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF1(0)) != 1 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF2(0)) != 2 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF3(0)) != 3 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF4(0)) != 4 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF5(0)) != 5 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF6(0)) != 6 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF7(0)) != 7 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyInteger(ImportsAllIntegersF8(0)) != 8 {
		panic("unexpect result in ImportsIdentifyInteger")
	}

	if ImportsIdentifyFloat(ImportsAllFloatsF0(0.0)) != 0 {
		panic("unexpect result in ImportsIdentifyFloat")
	}

	if ImportsIdentifyFloat(ImportsAllFloatsF1(0.0)) != 1 {
		panic("unexpect result in ImportsIdentifyFloat")
	}

	// All Text
	if ImportsIdentifyText(ImportsAllTextF0('a')) != 0 {
		panic("unexpected result in ImportsIdentifyText")
	}
	if ImportsIdentifyText(ImportsAllTextF1("abc")) != 1 {
		panic("unexpected result in ImportsIdentifyText")
	}

	// All Duplicated
	res21 := ImportsAddOneDuplicated(ImportsDuplicatedS32F0(0))
	if res21.GetF0() != 1 {
		panic("unexpected result in ImportsAddOneDuplicated")
	}

	res22 := ImportsAddOneDuplicated(ImportsDuplicatedS32F1(1))
	if res22.GetF1() != 2 {
		panic("unexpected result in ImportsAddOneDuplicated")
	}

	res23 := ImportsAddOneDuplicated(ImportsDuplicatedS32F2(2))
	if res23.GetF2() != 3 {
		panic("unexpected result in ImportsAddOneDuplicated")
	}

	// Distinguishable

	if ImportsAddOneDistinguishableNum(ImportsDistinguishableNumF1(0)).GetF1() != 1 {
		panic("unexpect result in ImportsAddOneDistinguishableNum")
	}

	if ImportsIdentifyDistinguishableNum(ImportsDistinguishableNumF0(0.0)) != 0 {
		panic("unexpect result in ImportsIdentifyDistinguishableNum")
	}

	if ImportsIdentifyDistinguishableNum(ImportsDistinguishableNumF1(1)) != 1 {
		panic("unexpect result in ImportsIdentifyDistinguishableNum")
	}

}

func (u UnionsImpl) AddOneInteger(num ExportsAllIntegers) ExportsAllIntegers {
	switch num.Kind() {
	case ExportsAllIntegersKindF0:
		return ExportsAllIntegersF0(!num.GetF0())
	case ExportsAllIntegersKindF1:
		return ExportsAllIntegersF1(num.GetF1() + 1)
	case ExportsAllIntegersKindF2:
		return ExportsAllIntegersF2(num.GetF2() + 1)
	case ExportsAllIntegersKindF3:
		return ExportsAllIntegersF3(num.GetF3() + 1)
	case ExportsAllIntegersKindF4:
		return ExportsAllIntegersF4(num.GetF4() + 1)
	case ExportsAllIntegersKindF5:
		return ExportsAllIntegersF5(num.GetF5() + 1)
	case ExportsAllIntegersKindF6:
		return ExportsAllIntegersF6(num.GetF6() + 1)
	case ExportsAllIntegersKindF7:
		return ExportsAllIntegersF7(num.GetF7() + 1)
	case ExportsAllIntegersKindF8:
		return ExportsAllIntegersF8(num.GetF8() + 1)
	default:
		panic("unexpected type in ExportsAllIntegers")
	}
}

func (u UnionsImpl) AddOneFloat(num ExportsAllFloats) ExportsAllFloats {
	switch num.Kind() {
	case ExportsAllFloatsKindF0:
		return ExportsAllFloatsF0(num.GetF0() + 1.0)
	case ExportsAllFloatsKindF1:
		return ExportsAllFloatsF1(num.GetF1() + 1.0)
	default:
		panic("unexpected type in ExportsAllFloats")
	}
}

func (u UnionsImpl) ReplaceFirstChar(text ExportsAllText, letter rune) ExportsAllText {
	switch text.Kind() {
	case ExportsAllTextKindF0:
		return ExportsAllTextF0(letter)
	case ExportsAllTextKindF1:
		return ExportsAllTextF1(string(letter) + text.GetF1()[1:])
	default:
		// handle error or panic
		return ExportsAllText{}
	}
}

func (u UnionsImpl) IdentifyInteger(num ExportsAllIntegers) uint8 {
	switch num.Kind() {
	case ExportsAllIntegersKindF0:
		return 0
	case ExportsAllIntegersKindF1:
		return 1
	case ExportsAllIntegersKindF2:
		return 2
	case ExportsAllIntegersKindF3:
		return 3
	case ExportsAllIntegersKindF4:
		return 4
	case ExportsAllIntegersKindF5:
		return 5
	case ExportsAllIntegersKindF6:
		return 6
	case ExportsAllIntegersKindF7:
		return 7
	case ExportsAllIntegersKindF8:
		return 8
	default:
		panic("unexpected type in ExportsAllIntegers")
	}
}

func (u UnionsImpl) IdentifyFloat(num ExportsAllFloats) uint8 {
	switch num.Kind() {
	case ExportsAllFloatsKindF0:
		return 0
	case ExportsAllFloatsKindF1:
		return 1
	default:
		panic("unexpected type in ExportsAllFloats")
	}
}

func (u UnionsImpl) IdentifyText(text ExportsAllText) uint8 {
	switch text.Kind() {
	case ExportsAllTextKindF0:
		return 0
	case ExportsAllTextKindF1:
		return 1
	default:
		panic("unexpected type in ExportsAllText")
	}
}

func (u UnionsImpl) AddOneDuplicated(num ExportsDuplicatedS32) ExportsDuplicatedS32 {
	switch num.Kind() {
	case ExportsDuplicatedS32KindF0:
		return ExportsDuplicatedS32F0(num.GetF0() + 1)
	case ExportsDuplicatedS32KindF1:
		return ExportsDuplicatedS32F1(num.GetF1() + 1)
	case ExportsDuplicatedS32KindF2:
		return ExportsDuplicatedS32F2(num.GetF2() + 1)
	default:
		panic("unexpected type in ExportsDuplicatedS32")
	}
}

func (u UnionsImpl) IdentifyDuplicated(num ExportsDuplicatedS32) uint8 {
	switch num.Kind() {
	case ExportsDuplicatedS32KindF0:
		return 0
	case ExportsDuplicatedS32KindF1:
		return 1
	case ExportsDuplicatedS32KindF2:
		return 2
	default:
		panic("unexpected type in IdentifyDuplicated")
	}
}

func (u UnionsImpl) AddOneDistinguishableNum(num ExportsDistinguishableNum) ExportsDistinguishableNum {
	switch num.Kind() {
	case ExportsDistinguishableNumKindF0:
		return ExportsDistinguishableNumF0(num.GetF0() + 1.0)
	case ExportsDistinguishableNumKindF1:
		return ExportsDistinguishableNumF1(num.GetF1() + 1)
	default:
		panic("unexpected type in ExportsDistinguishableNum")
	}
}

func (u UnionsImpl) IdentifyDistinguishableNum(num ExportsDistinguishableNum) uint8 {
	switch num.Kind() {
	case ExportsDistinguishableNumKindF0:
		return 0
	case ExportsDistinguishableNumKindF1:
		return 1
	default:
		panic("unexpected type in ExportsDistinguishableNum")
	}
}

func main() {}
