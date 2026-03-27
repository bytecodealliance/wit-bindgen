package export_exports

import (
	"fmt"
	"runtime"
	"wit_component/imports"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func Add(a *Z, b *Z) *Z {
	return MakeZ(a.a + b.a)
}

func Consume(x *X) {
	x.Drop()
}

func TestImports() Result[Unit, string] {
	{
		y1 := imports.MakeY(10)
		defer y1.Drop()
		assertEqual(y1.GetA(), 10)
		y1.SetA(20)
		assertEqual(y1.GetA(), 20)

		y2 := imports.YAdd(y1, 20)
		defer y2.Drop()
		assertEqual(y2.GetA(), 40)
	}

	{
		y1 := imports.MakeY(1)
		defer y1.Drop()
		y2 := imports.MakeY(2)
		defer y2.Drop()
		assertEqual(y1.GetA(), 1)
		assertEqual(y2.GetA(), 2)
		y1.SetA(10)
		y2.SetA(20)
		assertEqual(y1.GetA(), 10)
		assertEqual(y2.GetA(), 20)

		y3 := imports.YAdd(y1, 20)
		defer y3.Drop()
		y4 := imports.YAdd(y2, 30)
		defer y4.Drop()
		assertEqual(y3.GetA(), 30)
		assertEqual(y4.GetA(), 50)
	}

	return Ok[Unit, string](Unit{})
}

type X struct {
	pinner runtime.Pinner
	handle int32
	a      int32
}

func (self *X) GetA() int32 {
	return self.a
}

func (self *X) SetA(a int32) {
	self.a = a
}

func (self *X) OnDrop() {}

func MakeX(a int32) *X {
	return &X{runtime.Pinner{}, 0, a}
}

func XAdd(x *X, a int32) *X {
	defer x.Drop()
	return &X{runtime.Pinner{}, 0, a + x.a}
}

type Z struct {
	pinner runtime.Pinner
	handle int32
	a      int32
}

func (self *Z) GetA() int32 {
	return self.a
}

func (self *Z) OnDrop() {
	numDroppedZs++
}

func MakeZ(a int32) *Z {
	return &Z{runtime.Pinner{}, 0, a}
}

var numDroppedZs uint32 = 0

func ZNumDropped() uint32 {
	return numDroppedZs
}

type KebabCase struct {
	pinner runtime.Pinner
	handle int32
	a      uint32
}

func (self *KebabCase) GetA() uint32 {
	return self.a
}

func (self *KebabCase) OnDrop() {}

func MakeKebabCase(a uint32) *KebabCase {
	return &KebabCase{runtime.Pinner{}, 0, a}
}

func KebabCaseTakeOwned(k *KebabCase) uint32 {
	defer k.Drop()
	return k.a
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}
