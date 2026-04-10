//@ wasmtime-flags = '-Wcomponent-model-map'

package export_wit_world

import (
	"fmt"
	test "wit_component/test_maps_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Run() {
	testNamedRoundtrip()
	testBytesRoundtrip()
	testEmptyRoundtrip()
	testOptionRoundtrip()
	testRecordRoundtrip()
	testInlineRoundtrip()
	testLargeRoundtrip()
	testMultiParamRoundtrip()
	testNestedRoundtrip()
	testVariantRoundtrip()
	testResultRoundtrip()
	testTupleRoundtrip()
	testSingleEntryRoundtrip()
}

func testNamedRoundtrip() {
	input := test.NamesById{
		1: "uno",
		2: "two",
	}
	result := test.NamedRoundtrip(input)
	assertEqual(result["uno"], uint32(1))
	assertEqual(result["two"], uint32(2))
}

func testBytesRoundtrip() {
	input := test.BytesByName{
		"hello": []uint8("world"),
		"bin":   {0, 1, 2},
	}
	result := test.BytesRoundtrip(input)
	assertSliceEqual(result["hello"], []uint8("world"))
	assertSliceEqual(result["bin"], []uint8{0, 1, 2})
}

func testEmptyRoundtrip() {
	input := test.NamesById{}
	result := test.EmptyRoundtrip(input)
	assertEqual(len(result), 0)
}

func testOptionRoundtrip() {
	input := map[string]Option[uint32]{
		"some": Some[uint32](42),
		"none": None[uint32](),
	}
	result := test.OptionRoundtrip(input)
	assertEqual(len(result), 2)
	assertEqual(result["some"].Some(), uint32(42))
	assertEqual(result["none"].Tag(), OptionNone)
}

func testRecordRoundtrip() {
	entry := test.LabeledEntry{
		Label: "test-label",
		Values: test.NamesById{
			10: "ten",
			20: "twenty",
		},
	}
	result := test.RecordRoundtrip(entry)
	assertEqual(result.Label, "test-label")
	assertEqual(len(result.Values), 2)
	assertEqual(result.Values[10], "ten")
	assertEqual(result.Values[20], "twenty")
}

func testInlineRoundtrip() {
	input := map[uint32]string{
		1: "one",
		2: "two",
	}
	result := test.InlineRoundtrip(input)
	assertEqual(len(result), 2)
	assertEqual(result["one"], uint32(1))
	assertEqual(result["two"], uint32(2))
}

func testLargeRoundtrip() {
	input := make(test.NamesById)
	for i := uint32(0); i < 100; i++ {
		input[i] = fmt.Sprintf("value-%d", i)
	}
	result := test.LargeRoundtrip(input)
	assertEqual(len(result), 100)
	for i := uint32(0); i < 100; i++ {
		assertEqual(result[i], fmt.Sprintf("value-%d", i))
	}
}

func testMultiParamRoundtrip() {
	names := test.NamesById{
		1: "one",
		2: "two",
	}
	bytes := test.BytesByName{
		"key": {42},
	}
	ids, bytesOut := test.MultiParamRoundtrip(names, bytes)
	assertEqual(len(ids), 2)
	assertEqual(ids["one"], uint32(1))
	assertEqual(ids["two"], uint32(2))
	assertEqual(len(bytesOut), 1)
	assertSliceEqual(bytesOut["key"], []uint8{42})
}

func testNestedRoundtrip() {
	input := map[string]map[uint32]string{
		"group-a": {
			1: "one",
			2: "two",
		},
		"group-b": {
			10: "ten",
		},
	}
	result := test.NestedRoundtrip(input)
	assertEqual(len(result), 2)
	assertEqual(result["group-a"][1], "one")
	assertEqual(result["group-a"][2], "two")
	assertEqual(result["group-b"][10], "ten")
}

func testVariantRoundtrip() {
	m := test.NamesById{1: "one"}
	asMap := test.VariantRoundtrip(test.MakeMapOrStringAsMap(m))
	assertEqual(asMap.Tag(), test.MapOrStringAsMap)
	assertEqual(asMap.AsMap()[1], "one")

	asStr := test.VariantRoundtrip(test.MakeMapOrStringAsString("hello"))
	assertEqual(asStr.Tag(), test.MapOrStringAsString)
	assertEqual(asStr.AsString(), "hello")
}

func testResultRoundtrip() {
	m := test.NamesById{5: "five"}
	okResult := test.ResultRoundtrip(Ok[test.NamesById, string](m))
	assertEqual(okResult.Tag(), ResultOk)
	assertEqual(okResult.Ok()[5], "five")

	errResult := test.ResultRoundtrip(Err[test.NamesById, string]("bad input"))
	assertEqual(errResult.Tag(), ResultErr)
	assertEqual(errResult.Err(), "bad input")
}

func testTupleRoundtrip() {
	m := test.NamesById{7: "seven"}
	resultMap, resultNum := test.TupleRoundtrip(Tuple2[test.NamesById, uint64]{m, 42})
	assertEqual(len(resultMap), 1)
	assertEqual(resultMap[7], "seven")
	assertEqual(resultNum, uint64(42))
}

func testSingleEntryRoundtrip() {
	input := test.NamesById{99: "ninety-nine"}
	result := test.SingleEntryRoundtrip(input)
	assertEqual(len(result), 1)
	assertEqual(result[99], "ninety-nine")
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}

func assertSliceEqual[T comparable](a []T, b []T) {
	if len(a) != len(b) {
		panic(fmt.Sprintf("slices have different lengths: %d vs %d", len(a), len(b)))
	}
	for i := range a {
		if a[i] != b[i] {
			panic(fmt.Sprintf("slices differ at index %d: %v vs %v", i, a[i], b[i]))
		}
	}
}
