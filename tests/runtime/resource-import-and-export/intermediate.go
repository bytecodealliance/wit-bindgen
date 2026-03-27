package export_test_resource_import_and_export_test

import (
	"runtime"
	test "wit_component/test_resource_import_and_export_test"
)

type Thing struct {
	pinner runtime.Pinner
	handle int32
	thing  *test.Thing
}

func (self *Thing) Foo() uint32 {
	return self.thing.Foo() + 2
}

func (self *Thing) Bar(a uint32) {
	self.thing.Bar(a + 3)
}

func (self *Thing) OnDrop() {
	self.thing.Drop()
}

func MakeThing(a uint32) *Thing {
	return &Thing{runtime.Pinner{}, 0, test.MakeThing(a + 1)}
}

func ThingBaz(a *Thing, b *Thing) *Thing {
	defer a.Drop()
	defer b.Drop()
	return MakeThing(test.ThingBaz(a.thing, b.thing).Foo() + 4)
}
