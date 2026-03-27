package export_wit_world

import (
	"fmt"
	test "wit_component/exports"
)

func Run() {
	{
		val := test.TestImports()
		val.Ok()
	}

	x := test.MakeX(5)
	defer x.Drop()
	assertEqual(x.GetA(), 5)
	x.SetA(10)
	assertEqual(x.GetA(), 10)

	z1 := test.MakeZ(10)
	defer z1.Drop()
	assertEqual(z1.GetA(), 10)

	z2 := test.MakeZ(20)
	defer z2.Drop()
	assertEqual(z2.GetA(), 20)

	xadd := test.XAdd(x, 5)
	defer xadd.Drop()
	assertEqual(xadd.GetA(), 15)

	zadd := test.Add(z1, z2)
	defer zadd.Drop()
	assertEqual(zadd.GetA(), 30)

	droppedZsStart := test.ZNumDropped()

	z1.Drop()
	z2.Drop()

	test.Consume(xadd)

	droppedZsEnd := test.ZNumDropped()

	assertEqual(droppedZsEnd, droppedZsStart+2)
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
