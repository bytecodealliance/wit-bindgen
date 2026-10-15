package export_test_resource_own_roundtrip_to_test

import (
	"runtime"
)

type Thing struct {
	pinner runtime.Pinner
	handle int32
	val    uint32
}

var numDropped uint32 = 0

func (self *Thing) OnDrop() {
	numDropped++
}

func MakeThing(v uint32) *Thing {
	return &Thing{runtime.Pinner{}, 0, v}
}

func ConsumeBoth(a *Thing, b *Thing) uint32 {
	defer a.Drop()
	defer b.Drop()
	return a.val + b.val
}

func NumDropped() uint32 {
	return numDropped
}
