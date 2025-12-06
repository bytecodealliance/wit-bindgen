package export_test_resource_import_and_export_test

import (
	"runtime"
)

type Thing struct {
	pinner runtime.Pinner
	handle int32
	a      uint32
}

func (self *Thing) Foo() uint32 {
	return self.a + 2
}

func (self *Thing) Bar(a uint32) {
	self.a = a + 3
}

func (self *Thing) OnDrop() {}

func MakeThing(a uint32) *Thing {
	return &Thing{runtime.Pinner{}, 0, a + 1}
}

func ThingBaz(a *Thing, b *Thing) *Thing {
	defer a.Drop()
	defer b.Drop()
	return MakeThing(a.Foo() + b.Foo() + 4)
}
