package export_test_maps_to_test

import (
	. "wit_component/test_maps_to_test"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func NamedRoundtrip(a NamesById) IdsByName {
	result := make(IdsByName)
	for id, name := range a {
		result[name] = id
	}
	return result
}

func BytesRoundtrip(a BytesByName) BytesByName {
	return a
}

func EmptyRoundtrip(a NamesById) NamesById {
	return a
}

func OptionRoundtrip(a map[string]Option[uint32]) map[string]Option[uint32] {
	return a
}

func RecordRoundtrip(a LabeledEntry) LabeledEntry {
	return a
}

func InlineRoundtrip(a map[uint32]string) map[string]uint32 {
	result := make(map[string]uint32)
	for k, v := range a {
		result[v] = k
	}
	return result
}

func LargeRoundtrip(a NamesById) NamesById {
	return a
}

func MultiParamRoundtrip(a NamesById, b BytesByName) (IdsByName, BytesByName) {
	ids := make(IdsByName)
	for id, name := range a {
		ids[name] = id
	}
	return ids, b
}

func NestedRoundtrip(a map[string]map[uint32]string) map[string]map[uint32]string {
	return a
}

func VariantRoundtrip(a MapOrString) MapOrString {
	return a
}

func ResultRoundtrip(a Result[NamesById, string]) Result[NamesById, string] {
	return a
}

func TupleRoundtrip(a Tuple2[NamesById, uint64]) (NamesById, uint64) {
	return a.F0, a.F1
}

func SingleEntryRoundtrip(a NamesById) NamesById {
	return a
}
