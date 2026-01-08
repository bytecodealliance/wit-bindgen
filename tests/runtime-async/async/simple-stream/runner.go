package export_wit_world

import (
	"fmt"
	test "wit_component/my_test_i"

	"github.com/bytecodealliance/wit-bindgen/wit_types"
)

func Run() {
	write := make(chan wit_types.Unit)
	read := make(chan wit_types.Unit)

	{
		tx, rx := test.MakeStreamUnit()
		go func() {
			assertEqual(tx.Write([]wit_types.Unit{wit_types.Unit{}}), 1)
			assert(!tx.ReaderDropped())

			assertEqual(tx.Write([]wit_types.Unit{wit_types.Unit{}, wit_types.Unit{}}), 2)

			assertEqual(tx.Write([]wit_types.Unit{wit_types.Unit{}, wit_types.Unit{}}), 0)
			assert(tx.ReaderDropped())

			write <- wit_types.Unit{}
		}()

		go func() {
			test.ReadStream(rx)
			read <- wit_types.Unit{}
		}()

		(<-read)
		(<-write)
	}

	{
		tx, rx := test.MakeStreamUnit()
		syncBarrier := make(chan struct{})
		panicCh := make(chan any, 2)

		for range 2 {
			go func() {
				// Because the channel is empty, it will block until it's closed, at which
				// point all Goroutines will attempt to simultaneously read from the stream.
				<-syncBarrier
				panicCh <- checkPanicValue(func() {
					result := make([]wit_types.Unit, 1)
					rx.Read(result)
				})
			}()
		}
		close(syncBarrier)

		go func() {
			// If this is omitted, the host will see that the "rx.Read" operations aren't paired with 
			// a "tx.WriteAll" and will result in a "wasm trap: deadlock detected" error. Additionally, 
			// this is placed after "close(syncBarrier)" to ensure that the panics are resulting from 
			// concurrent reads, and not from other scenarios that result in a nil handle.
			tx.WriteAll([]wit_types.Unit{wit_types.Unit{}})
		}()

		p1, p2 := <-panicCh, <-panicCh

		// One should succeed (nil), one should panic
		assert((p1 == nil && p2 == "nil handle") || (p1 == "nil handle" && p2 == nil))
	}

	{
		tx, rx := test.MakeStreamUnit()
		syncBarrier := make(chan struct{})
		panicCh := make(chan any, 2)

		for range 2 {
			go func() {
				// Because the channel is empty, it will block until it's closed, at which
				// point all Goroutines will attempt to simultaneously write to the stream.
				<-syncBarrier
				panicCh <- checkPanicValue(func() {
					tx.WriteAll([]wit_types.Unit{wit_types.Unit{}})
				})
			}()
		}
		close(syncBarrier)

		go func() {
			// If this is omitted, the host will see that the "tx.WriteAll" operations aren't paired with 
			// an "rx.Read" and will result in a "wasm trap: deadlock detected" error. Additionally, 
			// this is placed after "close(syncBarrier)" to ensure that the panics are resulting from 
			// concurrent writes, and not from other scenarios that result in a nil handle.
			result := make([]wit_types.Unit, 1)
			rx.Read(result)
		}()

		p1, p2 := <-panicCh, <-panicCh

		// One should succeed (nil), one should panic
		assert((p1 == nil && p2 == "nil handle") || (p1 == "nil handle" && p2 == nil))
	}
}

func assertEqual[T comparable](a, b T) {
	if a != b {
		panic(fmt.Sprintf("%v not equal to %v", a, b))
	}
}

func assert(v bool) {
	if !v {
		panic("assertion failed")
	}
}

func checkPanicValue(f func()) (value any) {
	defer func() {
		value = recover()
	}()
	f()
	return nil
}
