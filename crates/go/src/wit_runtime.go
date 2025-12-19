package wit_runtime

import (
	"fmt"
	"runtime"
	"unsafe"
)

type Handle struct {
	value int32
}

func (self *Handle) Use() int32 {
	if self.value == 0 {
		panic("nil handle")
	}
	return self.value
}

func (self *Handle) Take() int32 {
	if self.value == 0 {
		panic("nil handle")
	}
	value := self.value
	self.value = 0
	return value
}

func (self *Handle) Set(value int32) {
	if value == 0 {
		panic("nil handle")
	}
	if self.value != 0 {
		panic("handle already set")
	}
	self.value = value
}

func (self *Handle) TakeOrNil() int32 {
	value := self.value
	self.value = 0
	return value
}

func MakeHandle(value int32) *Handle {
	if value == 0 {
		panic("nil handle")
	}
	return &Handle{value}
}

func Allocate(pinner *runtime.Pinner, size, align uintptr) unsafe.Pointer {
	pointer := allocateRaw(size, align)
	pinner.Pin(pointer)
	return pointer
}

func allocateRaw(size, align uintptr) unsafe.Pointer {
	if size == 0 {
		return unsafe.Pointer(uintptr(0))
	}

	if size%align != 0 {
		panic(fmt.Sprintf("size %v is not compatible with alignment %v", size, align))
	}

	switch align {
	case 1:
		return unsafe.Pointer(unsafe.SliceData(make([]uint8, size)))
	case 2:
		return unsafe.Pointer(unsafe.SliceData(make([]uint16, size/align)))
	case 4:
		return unsafe.Pointer(unsafe.SliceData(make([]uint32, size/align)))
	case 8:
		return unsafe.Pointer(unsafe.SliceData(make([]uint64, size/align)))
	default:
		panic(fmt.Sprintf("unsupported alignment: %v", align))
	}
}

// NB: `cabi_realloc` may be called before the Go runtime has been initialized,
// in which case we need to use `runtime.sbrk` to do allocations.  The following
// is an abbreviation of [Till's
// efforts](https://github.com/bytecodealliance/go-modules/pull/367).

//go:linkname sbrk runtime.sbrk
func sbrk(n uintptr) unsafe.Pointer

var useGCAllocations = false

func init() {
	useGCAllocations = true
}

func offset(ptr, align uintptr) uintptr {
	newptr := (ptr + align - 1) &^ (align - 1)
	return newptr - ptr
}

var pinner = runtime.Pinner{}

func Unpin() {
	pinner.Unpin()
}

//go:wasmexport cabi_realloc
func cabiRealloc(oldPointer unsafe.Pointer, oldSize, align, newSize uintptr) unsafe.Pointer {
	if oldPointer != nil || oldSize != 0 {
		panic("todo")
	}

	if useGCAllocations {
		return Allocate(&pinner, newSize, align)
	} else {
		alignedSize := newSize + offset(newSize, align)
		unaligned := sbrk(alignedSize)
		off := offset(uintptr(unaligned), align)
		return unsafe.Add(unaligned, off)
	}
}
