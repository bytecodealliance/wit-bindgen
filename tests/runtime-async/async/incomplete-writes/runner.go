package export_wit_world

import (
	"fmt"
	leaf "wit_component/my_test_leaf_interface"
	test "wit_component/my_test_test_interface"
)

func Run() {
	{
		tx, rx := test.MakeStreamTestThing()
		defer tx.Drop()
		defer rx.Drop()

		stream := test.ShortReadsTest(rx)
		defer stream.Drop()

		// Write the things all at once.  The callee will read them only
		// one at a time, forcing us to re-take ownership of any
		// unwritten items between writes.
		tx.WriteAll([]*test.TestThing{
			test.MakeTestThing("a"),
			test.MakeTestThing("b"),
			test.MakeTestThing("c"),
		})
		tx.Drop()

		things := []*test.TestThing{}
		for !stream.WriterDropped() {
			// Read just one item at a time, forcing the writer to
			// re-take ownership of any unwritten items between
			// writes.
			buffer := make([]*test.TestThing, 1)
			count := stream.Read(buffer)
			if count == 1 {
				things = append(things, buffer[0])
			}
		}

		assertEqual(things[0].Get(), "a")
		assertEqual(things[1].Get(), "b")
		assertEqual(things[2].Get(), "c")
	}

	{
		tx, rx := test.MakeStreamMyTestLeafInterfaceLeafThing()
		defer tx.Drop()
		defer rx.Drop()

		stream := test.ShortReadsLeaf(rx)
		defer stream.Drop()

		// Write the things all at once.  The callee will read them only
		// one at a time, forcing us to re-take ownership of any
		// unwritten items between writes.
		tx.WriteAll([]*leaf.LeafThing{
			leaf.MakeLeafThing("a"),
			leaf.MakeLeafThing("b"),
			leaf.MakeLeafThing("c"),
		})
		tx.Drop()

		things := []*leaf.LeafThing{}
		for !stream.WriterDropped() {
			// Read just one item at a time, forcing the writer to
			// re-take ownership of any unwritten items between
			// writes.
			buffer := make([]*leaf.LeafThing, 1)
			count := stream.Read(buffer)
			if count == 1 {
				things = append(things, buffer[0])
			}
		}

		assertEqual(things[0].Get(), "a")
		assertEqual(things[1].Get(), "b")
		assertEqual(things[2].Get(), "c")
	}

	{
		tx1, rx1 := test.MakeFutureTestThing()
		tx2, rx2 := test.MakeFutureTestThing()
		f1, f2 := test.DroppedReaderTest(rx1, rx2)

		{
			// Write a thing to the first future, the read end of
			// which the callee will drop without reading from,
			// forcing us to re-take ownership.
			thing := test.MakeTestThing("a")
			assert(!tx1.Write(thing))

			// Write it again to the second future.  This time, the
			// callee will read it.
			assert(tx2.Write(thing))
		}

		{
			// Drop the first future without reading from it.  This
			// will force the callee to re-take ownership of the
			// thing it tried to write.
			f1.Drop()

			// Read from the second future and assert it matches
			// what we wrote above.
			thing := f2.Read()
			assertEqual(thing.Get(), "a")
		}
	}

	{
		tx1, rx1 := test.MakeFutureMyTestLeafInterfaceLeafThing()
		tx2, rx2 := test.MakeFutureMyTestLeafInterfaceLeafThing()
		f1, f2 := test.DroppedReaderLeaf(rx1, rx2)

		{
			// Write a thing to the first future, the read end of
			// which the callee will drop without reading from,
			// forcing us to re-take ownership.
			thing := leaf.MakeLeafThing("a")
			assert(!tx1.Write(thing))

			// Write it again to the second future.  This time, the
			// callee will read it.
			assert(tx2.Write(thing))
		}

		{
			// Drop the first future without reading from it.  This
			// will force the callee to re-take ownership of the
			// thing it tried to write.
			f1.Drop()

			// Read from the second future and assert it matches
			// what we wrote above.
			thing := f2.Read()
			assertEqual(thing.Get(), "a")
		}
	}
}

func assertEqual[T comparable](a T, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}
