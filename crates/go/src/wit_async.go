package wit_async

import (
	"fmt"
	"runtime"
	"unsafe"
)

const EVENT_NONE uint32 = 0
const EVENT_SUBTASK uint32 = 1
const EVENT_STREAM_READ uint32 = 2
const EVENT_STREAM_WRITE uint32 = 3
const EVENT_FUTURE_READ uint32 = 4
const EVENT_FUTURE_WRITE uint32 = 5

const STATUS_STARTING uint32 = 0
const STATUS_STARTED uint32 = 1
const STATUS_RETURNED uint32 = 2

const CALLBACK_CODE_EXIT uint32 = 0
const CALLBACK_CODE_YIELD uint32 = 1
const CALLBACK_CODE_WAIT uint32 = 2
const CALLBACK_CODE_POLL uint32 = 3

const RETURN_CODE_BLOCKED uint32 = 0xFFFFFFFF
const RETURN_CODE_COMPLETED uint32 = 0
const RETURN_CODE_DROPPED uint32 = 1

type unit struct{}

type taskState struct {
	channel     chan unit
	waitableSet uint32
	pending     map[uint32]chan uint32
	yielding    chan unit
	pinner      runtime.Pinner
}

var state *taskState = nil

func Run(closure func()) uint32 {
	state = &taskState{
		make(chan unit),
		0,
		make(map[uint32]chan uint32),
		nil,
		runtime.Pinner{},
	}
	state.pinner.Pin(state)

	defer func() {
		state = nil
	}()

	go closure()

	return callback(EVENT_NONE, 0, 0)
}

func Callback(event0, event1, event2 uint32) uint32 {
	state = (*taskState)(contextGet())
	contextSet(nil)

	return callback(event0, event1, event2)
}

func callback(event0, event1, event2 uint32) uint32 {
	yielding := state.yielding
	if state.yielding != nil {
		state.yielding = nil
		yielding <- unit{}
	}

	switch event0 {
	case EVENT_NONE:

	case EVENT_SUBTASK:
		switch event2 {
		case STATUS_STARTING:
			panic(fmt.Sprintf("unexpected subtask status: %v", event2))

		case STATUS_STARTED:

		case STATUS_RETURNED:
			waitableJoin(event1, 0)
			subtaskDrop(event1)
			channel := state.pending[event1]
			delete(state.pending, event1)
			channel <- event2

		default:
			panic("todo")
		}

	case EVENT_STREAM_READ, EVENT_STREAM_WRITE, EVENT_FUTURE_READ, EVENT_FUTURE_WRITE:
		waitableJoin(event1, 0)
		channel := state.pending[event1]
		delete(state.pending, event1)
		channel <- event2

	default:
		panic("todo")
	}

	// Tell the Go scheduler to write to `state.channel` only after all
	// goroutines have either blocked or exited.  This allows us to reliably
	// delay returning control to the host until there's truly nothing more
	// we can do in the guest.
	//
	// Note that this function is _not_ currently part of upstream Go; it
	// requires [this
	// patch](https://github.com/dicej/go/commit/a1c83220fc9576cdb810e9624366cb998e69b22b)
	runtime.WasiOnIdle(func() bool {
		state.channel <- unit{}
		return true
	})
	defer runtime.WasiOnIdle(func() bool {
		return false
	})

	// Block this goroutine until the scheduler wakes us up.
	(<-state.channel)

	if state.yielding != nil {
		contextSet(unsafe.Pointer(state))
		if len(state.pending) == 0 {
			return CALLBACK_CODE_YIELD
		} else {
			if state.waitableSet == 0 {
				panic("unreachable")
			}
			return CALLBACK_CODE_POLL | (state.waitableSet << 4)
		}
	} else if len(state.pending) == 0 {
		state.pinner.Unpin()
		if state.waitableSet != 0 {
			waitableSetDrop(state.waitableSet)
		}
		return CALLBACK_CODE_EXIT
	} else {
		if state.waitableSet == 0 {
			panic("unreachable")
		}
		contextSet(unsafe.Pointer(state))
		return CALLBACK_CODE_WAIT | (state.waitableSet << 4)
	}
}

func SubtaskWait(status uint32) {
	subtask := status >> 4
	status = status & 0xF

	switch status {
	case STATUS_STARTING, STATUS_STARTED:
		if state.waitableSet == 0 {
			state.waitableSet = waitableSetNew()
		}
		waitableJoin(subtask, state.waitableSet)
		channel := make(chan uint32)
		state.pending[subtask] = channel
		(<-channel)

	case STATUS_RETURNED:

	default:
		panic(fmt.Sprintf("unexpected subtask status: %v", status))
	}
}

func FutureOrStreamWait(code uint32, handle int32) (uint32, uint32) {
	if code == RETURN_CODE_BLOCKED {
		if state.waitableSet == 0 {
			state.waitableSet = waitableSetNew()
		}
		waitableJoin(uint32(handle), state.waitableSet)
		channel := make(chan uint32)
		state.pending[uint32(handle)] = channel
		code = (<-channel)
	}

	count := code >> 4
	code = code & 0xF

	return code, count
}

func Yield() {
	channel := make(chan unit)
	state.yielding = channel
	(<-channel)
}

//go:wasmimport $root [waitable-set-new]
func waitableSetNew() uint32

//go:wasmimport $root [waitable-set-drop]
func waitableSetDrop(waitableSet uint32)

//go:wasmimport $root [waitable-join]
func waitableJoin(waitable, waitableSet uint32)

//go:wasmimport $root [context-get-0]
func contextGet() unsafe.Pointer

//go:wasmimport $root [context-set-0]
func contextSet(value unsafe.Pointer)

//go:wasmimport $root [subtask-drop]
func subtaskDrop(subtask uint32)
