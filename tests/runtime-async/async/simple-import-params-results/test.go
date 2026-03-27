package export_a_b_i

import "fmt"

func OneArgument(x uint32) {
	assertEqual(x, 1)
}

func OneResult() uint32 {
	return 2
}

func OneArgumentAndResult(x uint32) uint32 {
	assertEqual(x, 3)
	return 4
}

func TwoArguments(x, y uint32) {
	assertEqual(x, 5)
	assertEqual(y, 6)
}

func TwoArgumentsAndResult(x, y uint32) uint32 {
	assertEqual(x, 7)
	assertEqual(y, 8)
	return 9
}

func assertEqual[T comparable](a, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
