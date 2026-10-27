package export_my_test_i

import (
	"runtime"
	"wit_component/my_test_i"

	. "go.bytecodealliance.org/pkg/wit/types"
)

func ManyArguments(a, b, c, d, e, f, g, h, i string) {
	if a != "a" {
		panic("a")
	}
	if b != "b" {
		panic("b")
	}
	if c != "c" {
		panic("c")
	}
	if d != "d" {
		panic("d")
	}
	if e != "e" {
		panic("e")
	}
	if f != "f" {
		panic("f")
	}
	if g != "g" {
		panic("g")
	}
	if h != "h" {
		panic("h")
	}
	if i != "i" {
		panic("i")
	}
}

func ManyArgumentsPending(x *FutureReader[Unit], a, b, c, d, e, f, g, h string) {
	if a != "a" {
		panic("a")
	}
	if b != "b" {
		panic("b")
	}
	if c != "c" {
		panic("c")
	}
	if d != "d" {
		panic("d")
	}
	if e != "e" {
		panic("e")
	}
	if f != "f" {
		panic("f")
	}
	if g != "g" {
		panic("g")
	}
	if h != "h" {
		panic("h")
	}
	defer x.Drop()
	x.Read()
}

func ManyArgumentsCases(a Option[uint32], b Result[uint32, string], c, d, e, f, g, h string) {
	// Either (Some(42), Err("error")) or (None, Ok(7)).
	switch a.Tag() {
	case OptionSome:
		if a.Some() != 42 {
			panic("a")
		}
		if b.Tag() != ResultErr || b.Err() != "error" {
			panic("b")
		}
	case OptionNone:
		if b.Tag() != ResultOk || b.Ok() != 7 {
			panic("b")
		}
	default:
		panic("a")
	}
	if c != "c" {
		panic("c")
	}
	if d != "d" {
		panic("d")
	}
	if e != "e" {
		panic("e")
	}
	if f != "f" {
		panic("f")
	}
	if g != "g" {
		panic("g")
	}
	if h != "h" {
		panic("h")
	}
}

func OneBigRecord(r my_test_i.BigRecord) {
	if r.F0 != 0 {
		panic("f0")
	}
	if r.F8 != 8 {
		panic("f8")
	}
	if r.F16 != 16 {
		panic("f16")
	}
}

func DirectValues(a, b, c, d, e, f, g string, h uint32) {
	if a != "a" {
		panic("a")
	}
	if b != "b" {
		panic("b")
	}
	if c != "c" {
		panic("c")
	}
	if d != "d" {
		panic("d")
	}
	if e != "e" {
		panic("e")
	}
	if f != "f" {
		panic("f")
	}
	if g != "g" {
		panic("g")
	}
	if h != 8 {
		panic("h")
	}
}

// The first GC starts a mark worker, which takes the goroutine finished calls
// reuse, and every GC drops the runtime's cached Pinner, which its finalizer
// keeps on the heap one GC longer. Do both here, before any call, so the first
// reading holds what every later one does.
func init() {
	var p runtime.Pinner
	p.Pin(new(byte))
	p.Unpin()
	runtime.GC()
}

func HeapMark() uint32 {
	runtime.GC()
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	return uint32(m.HeapAlloc)
}

// The Go async runtime has no case for a cancelled export yet.
func SupportsCancel() bool {
	return false
}
