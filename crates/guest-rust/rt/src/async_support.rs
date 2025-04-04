#![deny(missing_docs)]
// TODO: Switch to interior mutability (e.g. use Mutexes or thread-local
// RefCells) and remove this, since even in single-threaded mode `static mut`
// references can be a hazard due to recursive access.
#![allow(static_mut_refs)]

extern crate std;
use core::sync::atomic::{AtomicBool, Ordering};
use std::boxed::Box;
use std::collections::HashMap;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::ptr;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::vec::Vec;

use futures::channel::oneshot;
use futures::future::FutureExt;
use futures::stream::{FuturesUnordered, StreamExt};

macro_rules! rtdebug {
    ($($f:tt)*) => {
        // Change this flag to enable debugging, right now we're not using a
        // crate like `log` or such to reduce runtime deps. Intended to be used
        // during development for now.
        if false {
            std::eprintln!($($f)*);
        }
    }

}

mod abi_buffer;
mod cabi;
mod error_context;
mod future_support;
mod stream_support;
mod subtask;
mod waitable;
mod waitable_set;

use self::waitable_set::WaitableSet;
pub use abi_buffer::*;
pub use error_context::*;
pub use future_support::*;
pub use stream_support::*;
#[doc(hidden)]
pub use subtask::Subtask;

pub use futures;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

/// Represents a task created by either a call to an async-lifted export or a
/// future run using `block_on` or `first_poll`.
struct FutureState {
    /// Remaining work to do (if any) before this task can be considered "done".
    ///
    /// Note that we won't tell the host the task is done until this is drained
    /// and `waitables` is empty.
    tasks: FuturesUnordered<BoxFuture>,

    /// The waitable set containing waitables created by this task, if any.
    waitable_set: Option<WaitableSet>,

    /// State of all waitables in `waitable_set`, and the ptr/callback they're
    /// associated with.
    waitables: HashMap<u32, (*mut c_void, unsafe extern "C" fn(*mut c_void, u32))>,

    /// Raw structure used to pass to `cabi::wasip3_task_set`
    wasip3_task: cabi::wasip3_task,

    /// Rust-level state for the waker, notably a bool as to whether this has
    /// been woken.
    waker: Arc<FutureWaker>,

    /// Clone of `waker` field, but represented as `std::task::Waker`.
    waker_clone: Waker,
}

impl FutureState {
    fn new(future: BoxFuture) -> FutureState {
        let waker = Arc::new(FutureWaker::default());
        FutureState {
            waker_clone: waker.clone().into(),
            waker,
            tasks: [future].into_iter().collect(),
            waitable_set: None,
            waitables: HashMap::new(),
            wasip3_task: cabi::wasip3_task {
                // This pointer is filled in before calling `wasip3_task_set`.
                ptr: ptr::null_mut(),
                version: cabi::WASIP3_TASK_V1,
                waitable_register,
                waitable_unregister,
            },
        }
    }

    fn get_or_create_waitable_set(&mut self) -> &WaitableSet {
        self.waitable_set.get_or_insert_with(WaitableSet::new)
    }

    fn add_waitable(&mut self, waitable: u32) {
        self.get_or_create_waitable_set().join(waitable)
    }

    fn remove_waitable(&mut self, waitable: u32) {
        WaitableSet::remove_waitable_from_all_sets(waitable)
    }

    fn remaining_work(&self) -> bool {
        !self.waitables.is_empty()
    }

    fn callback(&mut self, event0: u32, event1: u32, event2: u32) -> u32 {
        match event0 {
            EVENT_NONE => {
                rtdebug!("EVENT_NONE");
            }
            EVENT_CALL_STARTED => {
                rtdebug!("EVENT_CALL_STARTED({event1:#x})");
                self.deliver_waitable_event(event1, STATUS_STARTED)
            }
            EVENT_CALL_RETURNED => {
                rtdebug!("EVENT_CALL_RETURNED({event1:#x})");
                self.deliver_waitable_event(event1, STATUS_RETURNED)
            }

            EVENT_STREAM_READ | EVENT_STREAM_WRITE | EVENT_FUTURE_READ | EVENT_FUTURE_WRITE => {
                rtdebug!(
                    "EVENT_{{STREAM,FUTURE}}_{{READ,WRITE}}({event0:#x}, {event1:#x}, {event2:#x})"
                );
                self.deliver_waitable_event(event1, event2)
            }

            _ => unreachable!(),
        }

        loop {
            match self.poll() {
                // TODO: don't re-loop here once a host supports
                // `CALLBACK_CODE_YIELD`.
                CALLBACK_CODE_YIELD => {}
                other => break other,
            }
        }
    }

    /// Deliver the `code` event to the `waitable` store within our map. This
    /// waitable should be present because it's part of the waitable set which
    /// is kept in-sync with our map.
    fn deliver_waitable_event(&mut self, waitable: u32, code: u32) {
        self.remove_waitable(waitable);
        let (ptr, callback) = self.waitables.remove(&waitable).unwrap();
        unsafe {
            callback(ptr, code);
        }
    }

    /// Poll this task until it either completes or can't make immediate
    /// progress.
    fn poll(&mut self) -> u32 {
        // Finish our `wasip3_task` by initializing its self-referential pointer,
        // and then register it for the duration of this function with
        // `wasip3_task_set`. The previous value of `wasip3_task_set` will get
        // restored when this function returns.
        struct ResetTask(*mut cabi::wasip3_task);
        impl Drop for ResetTask {
            fn drop(&mut self) {
                unsafe {
                    cabi::wasip3_task_set(self.0);
                }
            }
        }
        let self_raw = self as *mut FutureState;
        self.wasip3_task.ptr = self_raw.cast();
        let prev = unsafe { cabi::wasip3_task_set(&mut self.wasip3_task) };
        let _reset = ResetTask(prev);

        let mut context = Context::from_waker(&self.waker_clone);

        loop {
            // Reset the waker before polling to clear out any pending
            // notification, if any.
            self.waker.0.store(false, Ordering::Relaxed);

            // Poll our future, handling `SPAWNED` around this.
            let poll;
            unsafe {
                poll = self.tasks.poll_next_unpin(&mut context);
                if !SPAWNED.is_empty() {
                    self.tasks.extend(SPAWNED.drain(..));
                }
            }

            match poll {
                // A future completed, yay! Keep going to see if more have
                // completed.
                Poll::Ready(Some(())) => (),

                // The `FuturesUnordered` list is empty meaning that there's no
                // more work left to do, so we're done.
                Poll::Ready(None) => {
                    assert!(!self.remaining_work());
                    assert!(self.tasks.is_empty());
                    break CALLBACK_CODE_EXIT;
                }

                // Some future within `FuturesUnordered` is not ready yet. If
                // our `waker` was signaled then that means this is a yield
                // operation, otherwise it means we're blocking on something.
                Poll::Pending => {
                    assert!(!self.tasks.is_empty());
                    if self.waker.0.load(Ordering::Relaxed) {
                        break CALLBACK_CODE_YIELD;
                    }

                    assert!(self.remaining_work());
                    let waitable = self.waitable_set.as_ref().unwrap().as_raw();
                    break CALLBACK_CODE_WAIT | (waitable << 4);
                }
            }
        }
    }
}

unsafe extern "C" fn waitable_register(
    ptr: *mut c_void,
    waitable: u32,
    callback: unsafe extern "C" fn(*mut c_void, u32),
    callback_ptr: *mut c_void,
) -> *mut c_void {
    let ptr = ptr.cast::<FutureState>();
    assert!(!ptr.is_null());
    (*ptr).add_waitable(waitable);
    match (*ptr).waitables.insert(waitable, (callback_ptr, callback)) {
        Some((prev, _)) => prev,
        None => ptr::null_mut(),
    }
}

unsafe extern "C" fn waitable_unregister(ptr: *mut c_void, waitable: u32) -> *mut c_void {
    let ptr = ptr.cast::<FutureState>();
    assert!(!ptr.is_null());
    (*ptr).remove_waitable(waitable);
    match (*ptr).waitables.remove(&waitable) {
        Some((prev, _)) => prev,
        None => ptr::null_mut(),
    }
}

#[derive(Default)]
struct FutureWaker(AtomicBool);

impl Wake for FutureWaker {
    fn wake(self: Arc<Self>) {
        Self::wake_by_ref(&self)
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Relaxed)
    }
}

/// Any newly-deferred work queued by calls to the `spawn` function while
/// polling the current task.
static mut SPAWNED: Vec<BoxFuture> = Vec::new();

const EVENT_NONE: u32 = 0;
const _EVENT_CALL_STARTING: u32 = 1;
const EVENT_CALL_STARTED: u32 = 2;
const EVENT_CALL_RETURNED: u32 = 3;
const EVENT_STREAM_READ: u32 = 5;
const EVENT_STREAM_WRITE: u32 = 6;
const EVENT_FUTURE_READ: u32 = 7;
const EVENT_FUTURE_WRITE: u32 = 8;

const CALLBACK_CODE_EXIT: u32 = 0;
const CALLBACK_CODE_YIELD: u32 = 1;
const CALLBACK_CODE_WAIT: u32 = 2;
const _CALLBACK_CODE_POLL: u32 = 3;

const STATUS_STARTING: u32 = 1;
const STATUS_STARTED: u32 = 2;
const STATUS_RETURNED: u32 = 3;

/// Poll the future generated by a call to an async-lifted export once, calling
/// the specified closure (presumably backed by a call to `task.return`) when it
/// generates a value.
///
/// This will return an appropriate status code to be returned from the
/// exported function.
#[doc(hidden)]
pub fn first_poll<T: 'static>(
    future: impl Future<Output = T> + 'static,
    fun: impl FnOnce(&T) + 'static,
) -> u32 {
    // Allocate a new `FutureState` which will track all state necessary for
    // our exported task.
    let state = Box::into_raw(Box::new(FutureState::new(Box::pin(
        future.map(|v| fun(&v)),
    ))));

    // Store our `FutureState` into our context-local-storage slot and then
    // pretend we got EVENT_NONE to kick off everything.
    //
    // SAFETY: we should own `context.set` as we're the root level exported
    // task, and then `callback` is only invoked when context-local storage is
    // valid.
    unsafe {
        assert!(context_get().is_null());
        context_set(state.cast());
        callback(EVENT_NONE, 0, 0)
    }
}

/// stream/future read/write results defined by the Component Model ABI.
mod results {
    pub const BLOCKED: u32 = 0xffff_ffff;
    pub const CLOSED: u32 = 0x8000_0000;
    pub const CANCELED: u32 = 0;
}

/// Handle a progress notification from the host regarding either a call to an
/// async-lowered import or a stream/future read/write operation.
///
/// # Unsafety
///
/// This function assumes that `context_get()` returns a `FutureState`.
#[doc(hidden)]
pub unsafe fn callback(event0: u32, event1: u32, event2: u32) -> u32 {
    // Acquire our context-local state, assert it's not-null, and then reset
    // the state to null while we're running to help prevent any unintended
    // usage.
    let state = context_get().cast::<FutureState>();
    assert!(!state.is_null());
    unsafe {
        context_set(ptr::null_mut());
    }

    // Use `state` to run the `callback` function in the context of our event
    // codes we received. If the callback decides to exit then we're done with
    // our future so deallocate it. Otherwise put our future back in
    // context-local storage and forward the code.
    unsafe {
        let rc = (*state).callback(event0, event1, event2);
        if rc == CALLBACK_CODE_EXIT {
            drop(Box::from_raw(state));
        } else {
            context_set(state.cast());
        }
        rc
    }
}

/// Defer the specified future to be run after the current async-lifted export
/// task has returned a value.
///
/// The task will remain in a running state until all spawned futures have
/// completed.
pub fn spawn(future: impl Future<Output = ()> + 'static) {
    unsafe { SPAWNED.push(Box::pin(future)) }
}

/// Run the specified future to completion, returning the result.
///
/// This uses `waitable-set.wait` to poll for progress on any in-progress calls
/// to async-lowered imports as necessary.
// TODO: refactor so `'static` bounds aren't necessary
pub fn block_on<T: 'static>(future: impl Future<Output = T> + 'static) -> T {
    let (tx, mut rx) = oneshot::channel();
    let state = &mut FutureState::new(Box::pin(future.map(move |v| drop(tx.send(v)))) as BoxFuture);
    let mut event = (EVENT_NONE, 0, 0);
    loop {
        match state.callback(event.0, event.1, event.2) {
            CALLBACK_CODE_EXIT => break rx.try_recv().unwrap().unwrap(),
            _ => event = state.waitable_set.as_ref().unwrap().wait(),
        }
    }
}

/// Call the `yield` canonical built-in function.
///
/// This yields control to the host temporarily, allowing other tasks to make
/// progress.  It's a good idea to call this inside a busy loop which does not
/// otherwise ever yield control the the host.
pub fn task_yield() {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn yield_() {
        unreachable!();
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[yield]"]
        fn yield_();
    }
    unsafe { yield_() }
}

/// Call the `backpressure.set` canonical built-in function.
///
/// When `enabled` is `true`, this tells the host to defer any new calls to this
/// component instance until further notice (i.e. until `backpressure.set` is
/// called again with `enabled` set to `false`).
pub fn backpressure_set(enabled: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn backpressure_set(_: i32) {
        unreachable!();
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[backpressure-set]"]
        fn backpressure_set(_: i32);
    }

    unsafe { backpressure_set(if enabled { 1 } else { 0 }) }
}

fn context_get() -> *mut u8 {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn get() -> *mut u8 {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[context-get-1]"]
        fn get() -> *mut u8;
    }

    unsafe { get() }
}

unsafe fn context_set(value: *mut u8) {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn set(_: *mut u8) {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[context-set-1]"]
        fn set(value: *mut u8);
    }

    unsafe { set(value) }
}
