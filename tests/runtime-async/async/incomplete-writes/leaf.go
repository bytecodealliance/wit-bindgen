package export_my_test_leaf_interface

import "runtime"

type LeafThing struct {
	pinner runtime.Pinner
	handle int32
	value  string
}

func (self *LeafThing) Get() string {
	return self.value
}

func (self *LeafThing) OnDrop() {}

func MakeLeafThing(value string) *LeafThing {
	return &LeafThing{runtime.Pinner{}, 0, value}
}
