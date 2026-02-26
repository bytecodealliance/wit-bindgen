package export_test_resource_borrow_to_test

import (
	"runtime"
)

type Thing struct {
	pinner runtime.Pinner
	handle int32
	val    uint32
}

func (self *Thing) OnDrop() {}

func MakeThing(v uint32) *Thing {
	return &Thing{runtime.Pinner{}, 0, v + 1}
}

func Foo(v *Thing) uint32 {
	return v.val + 2
}
