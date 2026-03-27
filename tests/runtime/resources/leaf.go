package export_imports

import (
	"runtime"
)

type Y struct {
	pinner runtime.Pinner
	handle int32
	a      int32
}

func (self *Y) GetA() int32 {
	return self.a
}

func (self *Y) SetA(a int32) {
	self.a = a
}

func (self *Y) OnDrop() {}

func MakeY(a int32) *Y {
	return &Y{runtime.Pinner{}, 0, a}
}

func YAdd(y *Y, a int32) *Y {
	defer y.Drop()
	return &Y{runtime.Pinner{}, 0, a + y.a}
}
