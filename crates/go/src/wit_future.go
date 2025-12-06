package wit_types

import (
	"runtime"
	"unsafe"
	"wit_component/wit_async"
	"wit_component/wit_runtime"
)

type FutureVtable[T any] struct {
	Size         uint32
	Align        uint32
	Read         func(handle int32, item unsafe.Pointer) uint32
	Write        func(handle int32, item unsafe.Pointer) uint32
	CancelRead   func(handle int32) uint32
	CancelWrite  func(handle int32) uint32
	DropReadable func(handle int32)
	DropWritable func(handle int32)
	Lift         func(src unsafe.Pointer) T
	Lower        func(pinner *runtime.Pinner, value T, dst unsafe.Pointer)
}

type FutureReader[T any] struct {
	vtable *FutureVtable[T]
	handle *wit_runtime.Handle
}

func (self *FutureReader[T]) Read() T {
	handle := self.handle.Take()
	defer self.vtable.DropReadable(handle)

	pinner := runtime.Pinner{}
	defer pinner.Unpin()

	buffer := wit_runtime.Allocate(&pinner, uintptr(self.vtable.Size), uintptr(self.vtable.Align))

	code, _ := wit_async.FutureOrStreamWait(self.vtable.Read(handle, buffer), handle)

	switch code {
	case wit_async.RETURN_CODE_COMPLETED:
		if self.vtable.Lift == nil {
			return unsafe.Slice((*T)(buffer), 1)[0]
		} else {
			return self.vtable.Lift(buffer)
		}

	case wit_async.RETURN_CODE_DROPPED:
		panic("unreachable")

	default:
		panic("todo: handle cancellation")
	}
}

func (self *FutureReader[T]) Drop() {
	handle := self.handle.TakeOrNil()
	if handle != 0 {
		self.vtable.DropReadable(handle)
	}
}

func (self *FutureReader[T]) TakeHandle() int32 {
	return self.handle.Take()
}

func MakeFutureReader[T any](vtable *FutureVtable[T], handleValue int32) *FutureReader[T] {
	handle := wit_runtime.MakeHandle(handleValue)
	value := &FutureReader[T]{vtable, handle}
	runtime.AddCleanup(value, func(_ int) {
		handleValue := handle.TakeOrNil()
		if handleValue != 0 {
			vtable.DropReadable(handleValue)
		}
	}, 0)
	return value
}

type FutureWriter[T any] struct {
	vtable *FutureVtable[T]
	handle *wit_runtime.Handle
}

func (self *FutureWriter[T]) Write(item T) bool {
	handle := self.handle.Take()
	defer self.vtable.DropWritable(handle)

	pinner := runtime.Pinner{}
	defer pinner.Unpin()

	var buffer unsafe.Pointer
	if self.vtable.Lower == nil {
		buffer = unsafe.Pointer(unsafe.SliceData([]T{item}))
		pinner.Pin(buffer)
	} else {
		buffer = wit_runtime.Allocate(&pinner, uintptr(self.vtable.Size), uintptr(self.vtable.Align))
		self.vtable.Lower(&pinner, item, buffer)
	}

	code, _ := wit_async.FutureOrStreamWait(self.vtable.Write(handle, buffer), handle)

	// TODO: restore handles to any unwritten resources, streams, or futures

	switch code {
	case wit_async.RETURN_CODE_COMPLETED:
		return true

	case wit_async.RETURN_CODE_DROPPED:
		return false

	default:
		panic("todo: handle cancellation")
	}
}

func (self *FutureWriter[T]) Drop() {
	handle := self.handle.TakeOrNil()
	if handle != 0 {
		self.vtable.DropWritable(handle)
	}
}

func MakeFutureWriter[T any](vtable *FutureVtable[T], handleValue int32) *FutureWriter[T] {
	handle := wit_runtime.MakeHandle(handleValue)
	value := &FutureWriter[T]{vtable, handle}
	runtime.AddCleanup(value, func(_ int) {
		handleValue := handle.TakeOrNil()
		if handleValue != 0 {
			vtable.DropReadable(handleValue)
		}
	}, 0)
	return value
}
