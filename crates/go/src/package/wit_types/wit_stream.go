package wit_types

import (
	"runtime"
	"unsafe"

	"go.bytecodealliance.org/wit-bindgen/wit_async"
	"go.bytecodealliance.org/wit-bindgen/wit_runtime"
)

type StreamVtable[T any] struct {
	Size         uint32
	Align        uint32
	Read         func(handle int32, items unsafe.Pointer, length uint32) uint32
	Write        func(handle int32, items unsafe.Pointer, length uint32) uint32
	CancelRead   func(handle int32) uint32
	CancelWrite  func(handle int32) uint32
	DropReadable func(handle int32)
	DropWritable func(handle int32)
	Lift         func(src unsafe.Pointer) T
	Lower        func(pinner *runtime.Pinner, value T, dst unsafe.Pointer) func()
}

type StreamReader[T any] struct {
	vtable        *StreamVtable[T]
	handle        *wit_runtime.Handle
	writerDropped bool
}

func (self *StreamReader[T]) WriterDropped() bool {
	return self.writerDropped
}

// Reads data from a stream into a destination slice.
//
// Blocks until the read completes or the destination slice is full.
//
// # Panic
//
// Read will panic if:
//   - dst is empty (length 0)
//   - multiple concurrent reads are attempted on the same stream
func (self *StreamReader[T]) Read(dst []T) uint32 {
	if len(dst) == 0 {
		panic("StreamReader.Read: destination slice cannot be empty")
	}

	handle := self.handle.Take()
	defer self.handle.Set(handle)

	if self.writerDropped {
		return 0
	}

	pinner := runtime.Pinner{}
	defer pinner.Unpin()

	var buffer unsafe.Pointer
	if self.vtable.Lift == nil {
		buffer = unsafe.Pointer(unsafe.SliceData(dst))
	} else {
		buffer = wit_runtime.Allocate(
			&pinner,
			uintptr(self.vtable.Size*uint32(len(dst))),
			uintptr(self.vtable.Align),
		)
	}
	pinner.Pin(buffer)

	code, count := wit_async.FutureOrStreamWait(self.vtable.Read(handle, buffer, uint32(len(dst))), handle)

	if code == wit_async.RETURN_CODE_DROPPED {
		self.writerDropped = true
	}

	if self.vtable.Lift != nil {
		for i := 0; i < int(count); i++ {
			dst[i] = self.vtable.Lift(unsafe.Add(buffer, i*int(self.vtable.Size)))
		}
	}

	return count
}

// Notify the host that the StreamReader is no longer being used.
func (self *StreamReader[T]) Drop() {
	handle := self.handle.TakeOrNil()
	if handle != 0 {
		self.vtable.DropReadable(handle)
	}
}

func (self *StreamReader[T]) TakeHandle() int32 {
	return self.handle.Take()
}

func (self *StreamReader[T]) SetHandle(handle int32) {
	self.handle.Set(handle)
}

func MakeStreamReader[T any](vtable *StreamVtable[T], handleValue int32) *StreamReader[T] {
	handle := wit_runtime.MakeHandle(handleValue)
	value := &StreamReader[T]{vtable, handle, false}
	runtime.AddCleanup(value, func(_ int) {
		handleValue := handle.TakeOrNil()
		if handleValue != 0 {
			vtable.DropReadable(handleValue)
		}
	}, 0)
	return value
}

type StreamWriter[T any] struct {
	vtable        *StreamVtable[T]
	handle        *wit_runtime.Handle
	readerDropped bool
}

func (self *StreamWriter[T]) ReaderDropped() bool {
	return self.readerDropped
}

// Writes items to a stream, returning the count written (may be partial).
//
// # Panic
//
// Write will panic if multiple concurrent writes are attempted on the same stream.
func (self *StreamWriter[T]) Write(items []T) uint32 {
	handle := self.handle.Take()
	defer self.handle.Set(handle)

	if self.readerDropped {
		return 0
	}

	pinner := runtime.Pinner{}
	defer pinner.Unpin()

	writeCount := uint32(len(items))

	var lifters []func()
	var buffer unsafe.Pointer
	if self.vtable.Lower == nil {
		buffer = unsafe.Pointer(unsafe.SliceData(items))
		pinner.Pin(buffer)
	} else {
		lifters = make([]func(), 0, writeCount)
		buffer = wit_runtime.Allocate(
			&pinner,
			uintptr(self.vtable.Size*writeCount),
			uintptr(self.vtable.Align),
		)
		for index, item := range items {
			lifters = append(
				lifters,
				self.vtable.Lower(&pinner, item, unsafe.Add(buffer, index*int(self.vtable.Size))),
			)
		}
	}

	code, count := wit_async.FutureOrStreamWait(self.vtable.Write(handle, buffer, writeCount), handle)

	if lifters != nil && count < writeCount {
		for _, lifter := range lifters[count:] {
			lifter()
		}
	}

	if code == wit_async.RETURN_CODE_DROPPED {
		self.readerDropped = true
	}

	return count
}

// Writes all items to the stream, looping until complete or reader drops.
//
// # Panic
//
// WriteAll will panic if multiple concurrent writes are attempted on the same stream.
func (self *StreamWriter[T]) WriteAll(items []T) uint32 {
	offset := uint32(0)
	count := uint32(len(items))
	for offset < count && !self.readerDropped {
		offset += self.Write(items[offset:])
	}
	return offset
}

// Notify the host that the StreamReader is no longer being used.
func (self *StreamWriter[T]) Drop() {
	handle := self.handle.TakeOrNil()
	if handle != 0 {
		self.vtable.DropWritable(handle)
	}
}

func MakeStreamWriter[T any](vtable *StreamVtable[T], handleValue int32) *StreamWriter[T] {
	handle := wit_runtime.MakeHandle(handleValue)
	value := &StreamWriter[T]{vtable, handle, false}
	runtime.AddCleanup(value, func(_ int) {
		handleValue := handle.TakeOrNil()
		if handleValue != 0 {
			vtable.DropReadable(handleValue)
		}
	}, 0)
	return value
}
