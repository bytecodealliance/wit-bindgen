package export_test_many_arguments_to_test

func ManyArguments(
	a1 uint64,
	a2 uint64,
	a3 uint64,
	a4 uint64,
	a5 uint64,
	a6 uint64,
	a7 uint64,
	a8 uint64,
	a9 uint64,
	a10 uint64,
	a11 uint64,
	a12 uint64,
	a13 uint64,
	a14 uint64,
	a15 uint64,
	a16 uint64,
) {
	assertEqual(a1, 1)
	assertEqual(a2, 2)
	assertEqual(a3, 3)
	assertEqual(a4, 4)
	assertEqual(a5, 5)
	assertEqual(a6, 6)
	assertEqual(a7, 7)
	assertEqual(a8, 8)
	assertEqual(a9, 9)
	assertEqual(a10, 10)
	assertEqual(a11, 11)
	assertEqual(a12, 12)
	assertEqual(a13, 13)
	assertEqual(a14, 14)
	assertEqual(a15, 15)
	assertEqual(a16, 16)
}

func assertEqual(a uint64, b uint64) {
	if a != b {
		panic("trouble")
	}
}
