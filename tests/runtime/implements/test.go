package export_test_implements_store

import (
	. "go.bytecodealliance.org/pkg/wit/types"
)

var store = map[string]string{}

func Get(key string) Option[string] {
	if value, ok := store[key]; ok {
		return Some[string](value)
	}
	return None[string]()
}

func Set(key string, value string) {
	store[key] = value
}
