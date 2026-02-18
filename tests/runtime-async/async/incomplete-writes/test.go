package export_my_test_test_interface

import (
	"runtime"
	. "wit_component/my_test_test_interface"

	. "go.bytecodealliance.org/pkg/wit/types"
)

type TestThing struct {
	pinner runtime.Pinner
	handle int32
	value  string
}

func (self *TestThing) Get() string {
	return self.value
}

func (self *TestThing) OnDrop() {}

func MakeTestThing(value string) *TestThing {
	return &TestThing{runtime.Pinner{}, 0, value}
}

func ShortReadsTest(stream *StreamReader[*TestThing]) *StreamReader[*TestThing] {
	tx, rx := MakeStreamTestThing()

	go func() {
		defer stream.Drop()
		defer tx.Drop()

		things := []*TestThing{}
		for !stream.WriterDropped() {
			// Read just one item at a time, forcing the writer to
			// re-take ownership of any unwritten items between
			// writes.
			buffer := make([]*TestThing, 1)
			count := stream.Read(buffer)
			if count == 1 {
				things = append(things, buffer[0])
			}
		}

		// Write the things all at once.  The caller will read them only
		// one at a time, forcing us to re-take ownership of any
		// unwritten items between writes.
		tx.WriteAll(things)
	}()

	return rx
}

func ShortReadsLeaf(stream *StreamReader[*LeafThing]) *StreamReader[*LeafThing] {
	tx, rx := MakeStreamMyTestLeafInterfaceLeafThing()

	go func() {
		defer stream.Drop()
		defer tx.Drop()

		things := []*LeafThing{}
		for !stream.WriterDropped() {
			// Read just one item at a time, forcing the writer to
			// re-take ownership of any unwritten items between
			// writes.
			buffer := make([]*LeafThing, 1)
			count := stream.Read(buffer)
			if count == 1 {
				things = append(things, buffer[0])
			}
		}

		// Write the things all at once.  The caller will read them only
		// one at a time, forcing us to re-take ownership of any
		// unwritten items between writes.
		tx.WriteAll(things)
	}()

	return rx
}

func DroppedReaderTest(f1, f2 *FutureReader[*TestThing]) (*FutureReader[*TestThing], *FutureReader[*TestThing]) {
	tx1, rx1 := MakeFutureTestThing()
	tx2, rx2 := MakeFutureTestThing()

	go func() {
		// Drop the first future without reading from it.  This will
		// force the callee to re-take ownership of the thing it tried
		// to write.
		f1.Drop()

		thing := f2.Read()

		// Write the thing to the first future, the read end of which
		// the callee will drop without reading from, forcing us to
		// re-take ownership.
		assert(!tx1.Write(thing))

		// Write it again to the second future.  This time, the caller
		// will read it.
		assert(tx2.Write(thing))
	}()

	return rx1, rx2
}

func DroppedReaderLeaf(f1, f2 *FutureReader[*LeafThing]) (*FutureReader[*LeafThing], *FutureReader[*LeafThing]) {
	tx1, rx1 := MakeFutureMyTestLeafInterfaceLeafThing()
	tx2, rx2 := MakeFutureMyTestLeafInterfaceLeafThing()

	go func() {
		// Drop the first future without reading from it.  This will
		// force the callee to re-take ownership of the thing it tried
		// to write.
		f1.Drop()

		thing := f2.Read()

		// Write the thing to the first future, the read end of which
		// the callee will drop without reading from, forcing us to
		// re-take ownership.
		assert(!tx1.Write(thing))

		// Write it again to the second future.  This time, the caller
		// will read it.
		assert(tx2.Write(thing))
	}()

	return rx1, rx2
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}
