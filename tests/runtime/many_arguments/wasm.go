package main

import (
	. "wit_many_arguments_go/gen"
)

func init() {
	n := &ManyArgumentsImpl{}
	SetManyArguments(n)
}

type ManyArgumentsImpl struct{}

func (m *ManyArgumentsImpl) ManyArguments(a1 uint64, a2 uint64, a3 uint64, a4 uint64, a5 uint64, a6 uint64, a7 uint64, a8 uint64, a9 uint64, a10 uint64, a11 uint64, a12 uint64, a13 uint64, a14 uint64, a15 uint64, a16 uint64) {
	assert_eq(a1, 1)
	assert_eq(a2, 2)
	assert_eq(a3, 3)
	assert_eq(a4, 4)
	assert_eq(a5, 5)
	assert_eq(a6, 6)
	assert_eq(a7, 7)
	assert_eq(a8, 8)
	assert_eq(a9, 9)
	assert_eq(a10, 10)
	assert_eq(a11, 11)
	assert_eq(a12, 12)
	assert_eq(a13, 13)
	assert_eq(a14, 14)
	assert_eq(a15, 15)
	assert_eq(a16, 16)
	ImportsManyArguments(a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16)
}

func assert_eq(a uint64, b uint64) {
	if a != b {
		panic("assertion failed")
	}
}
func main() {}
