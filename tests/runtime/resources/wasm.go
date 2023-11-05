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

func (e ExportsImpl) NewX(a int32) ExportsX {
	return &MyX{a: a}
}

func (e ExportsImpl) NewZ(a int32) ExportsZ {
	return &MyZ{a: a}
}

func (x *MyX) GetA() int32 {
	return x.a
}

func (x *MyX) SetA(a int32) {
	x.a = a
}

func (e ExportsImpl) XAdd(x ExportsX, a int32) ExportsX {
	return &MyX{a: x.GetA() + a}
}

func (z *MyZ) GetA() int32 {
	return z.a
}

func (e ExportsImpl) ZAdd(z ExportsZ, b ExportsZ) ExportsZ {
	return &MyZ{a: z.GetA() + b.GetA()}
}

func (e ExportsImpl) X() ExportsX {
	return &MyX{a: 0}
}

func (e ExportsImpl) Z() ExportsZ {
	return &MyZ{a: 0}
}

func (e ExportsImpl) TestImports() Result[struct{}, string]  {
	y := NewY(1)
	if y.GetA() != 1 {
		panic("y.GetA() != 1")
	}
	y.SetA(2)
	if y.GetA() != 2 {
		panic("y.GetA() != 2")
	}

	y2 := YAdd(y, 3)
	if y2.GetA() != 5 {
		panic("y2.GetA() != 5")
	}

	y.SetA(5)

	if y.GetA() != 5 {
		panic("y.GetA() != 5")
	}

	y.Drop()
	y2.Drop()

	return Result[struct{}, string]{
		Kind: Ok,
		Val: struct{}{},
	}
}

func main() {}