//@ wasmtime-flags = '-Wcomponent-model-async'

package export_wit_world

import (
	test "wit_component/my_test_i"

	"go.bytecodealliance.org/wit-bindgen/wit_types"
)

func Run() {
	write := make(chan bool)
	read := make(chan wit_types.Unit)

	{
		tx, rx := test.MakeFutureUnit()
		go func() {
			write <- tx.Write(wit_types.Unit{})
		}()
		go func() {
			test.ReadFuture(rx)
			read <- wit_types.Unit{}
		}()
		(<-read)
		assert(<-write)
	}

	{
		tx, rx := test.MakeFutureUnit()
		go func() {
			write <- tx.Write(wit_types.Unit{})
		}()
		go func() {
			test.DropFuture(rx)
			read <- wit_types.Unit{}
		}()
		(<-read)
		assert(!(<-write))
	}

	{
		tx, rx := test.MakeFutureUnit()
		syncBarrier := make(chan struct{})
		panicCh := make(chan any, 2)

		for range 2 {
			go func() {
				// Because the channel is empty, it will block until it's closed, at which
				// point all Goroutines will attempt to simultaneously read from the future.
				<-syncBarrier
				panicCh <- checkPanicValue(func() {
					rx.Read()
				})
			}()
		}
		close(syncBarrier)

		go func() {
			// If this is omitted, the host will see that the "rx.Read" operations aren't paired with
			// a "tx.Write" and will result in a "wasm trap: deadlock detected" error. Additionally,
			// this is placed after "close(syncBarrier)" to ensure that the panics are resulting from
			// concurrent reads, and not from other scenarios that result in a nil handle.
			tx.Write(wit_types.Unit{})
		}()

		p1, p2 := <-panicCh, <-panicCh

		// One should succeed (nil), one should panic
		assert((p1 == nil && p2 == "nil handle") || (p1 == "nil handle" && p2 == nil))
	}

	{
		tx, rx := test.MakeFutureUnit()
		syncBarrier := make(chan struct{})
		panicCh := make(chan any, 2)

		for range 2 {
			go func() {
				// Because the channel is empty, it will block until it's closed, at which
				// point all Goroutines will attempt to simultaneously write to the future.
				<-syncBarrier
				panicCh <- checkPanicValue(func() {
					tx.Write(wit_types.Unit{})
				})
			}()
		}
		close(syncBarrier)

		go func() {
			// If this is omitted, the host will see that the "tx.Write" operations aren't paired with
			// an "rx.Read" and will result in a "wasm trap: deadlock detected" error. Additionally,
			// this is placed after "close(syncBarrier)" to ensure that the panics are resulting from
			// concurrent writes, and not from other scenarios that result in a nil handle.
			rx.Read()
		}()

		p1, p2 := <-panicCh, <-panicCh

		// One should succeed (nil), one should panic
		assert((p1 == nil && p2 == "nil handle") || (p1 == "nil handle" && p2 == nil))
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
