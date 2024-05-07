package main

import (
	. "wit_resources_go/gen"
)

func init() {
	n := &ExportsImpl{}
	SetExports(n)
}

type ExportsImpl struct {
}

type MyX struct {
	a int32
}

type MyZ struct {
	a int32
}

type MyKebabCase struct {
	a uint32
}

func (e ExportsImpl) ConstructorX(a int32) ExportsX {
	return &MyX{a: a}
}

func (e ExportsImpl) ConstructorZ(a int32) ExportsZ {
	return &MyZ{a: a}
}

func (e ExportsImpl) ConstructorKebabCase(a uint32) ExportsKebabCase {
	return &MyKebabCase{a: a}
}

func (x *MyX) MethodXGetA() int32 {
	return x.a
}

func (x *MyX) MethodXSetA(a int32) {
	x.a = a
}

func (e ExportsImpl) StaticXAdd(x ExportsX, a int32) ExportsX {
	return &MyX{a: x.MethodXGetA() + a}
}

func (z *MyZ) MethodZGetA() int32 {
	return z.a
}

func (e ExportsImpl) StaticZNumDropped() uint32 {
        return 0
}

func (e ExportsImpl) Add(z ExportsZ, b ExportsZ) ExportsZ {
	return &MyZ{a: z.MethodZGetA() + b.MethodZGetA()}
}

func (e ExportsImpl) Consume(x ExportsX) {
	DropExportsX(x)
}

func (k *MyKebabCase) MethodKebabCaseGetA() uint32 {
	return k.a
}

func (e ExportsImpl) StaticKebabCaseTakeOwned(k ExportsKebabCase) uint32 {
	return k.MethodKebabCaseGetA()
}

func (e ExportsImpl) TestImports() Result[struct{}, string] {
	y := NewY(1)
	if y.GetA() != 1 {
		panic("y.GetA() != 1")
	}
	y.SetA(2)
	if y.GetA() != 2 {
		panic("y.GetA() != 2")
	}

	y2 := StaticYAdd(y, 3)
	if y2.GetA() != 5 {
		panic("y2.GetA() != 5")
	}

	y.SetA(5)

	if y.GetA() != 5 {
		panic("y.GetA() != 5")
	}

	y.Drop()
	return Ok[struct{}, string](struct{}{})
}

func main() {}
