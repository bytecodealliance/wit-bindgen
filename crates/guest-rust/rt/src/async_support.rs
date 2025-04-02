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
use std::fmt::{self, Debug, Display};
use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::ptr;
use std::string::String;
use std::sync::Arc;
use std::task::{Context, Poll, Wake};
use std::vec::Vec;

use futures::channel::oneshot;
use futures::future::FutureExt;
use futures::stream::{FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;

macro_rules! rtdebug {
    ($($f:tt)*) => {
        // Change this flag to enable debugging, right now we're not using a
        // crate like `log` or such to reduce runtime deps. Intended to be used
        // during development for now.
        if false {
            std::println!($($f)*);
        }
    }

}

mod abi_buffer;
mod cabi;
mod future_support;
mod stream_support;
mod waitable;

pub use abi_buffer::*;
pub use future_support::*;
pub use stream_support::*;

pub use futures;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

/// Represents a task created by either a call to an async-lifted export or a
/// future run using `block_on` or `poll_future`.
struct FutureState {
    /// Number of in-progress async-lowered import calls and/or stream/future reads/writes.
    todo: usize,
    /// Remaining work to do (if any) before this task can be considered "done".
    ///
    /// Note that we won't tell the host the task is done until this is drained
    /// and `todo` is zero.
    tasks: Option<FuturesUnordered<BoxFuture>>,
    /// The waitable set containing waitables created by this task, if any.
    waitable_set: Option<u32>,

    /// State of all waitables in `waitable_set`, and the ptr/callback they're
    /// associated with.
    waitables: HashMap<u32, (*mut c_void, unsafe extern "C" fn(*mut c_void, u32))>,

    /// Raw structure used to pass to `cabi::wasip3_task_set`
    wasip3_task: cabi::wasip3_task,
}

impl FutureState {
    fn new(future: BoxFuture) -> FutureState {
        FutureState {
            todo: 0,
            tasks: Some([future].into_iter().collect()),
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

    fn get_or_create_waitable_set(&mut self) -> u32 {
        *self.waitable_set.get_or_insert_with(waitable_set_new)
    }

    fn add_waitable(&mut self, waitable: u32) {
        unsafe { waitable_join(waitable, self.get_or_create_waitable_set()) }
    }

    fn remove_waitable(&mut self, waitable: u32) {
        unsafe { waitable_join(waitable, 0) }
    }

    fn remaining_work(&self) -> bool {
        self.todo > 0 || !self.waitables.is_empty()
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

impl Drop for FutureState {
    fn drop(&mut self) {
        if let Some(set) = self.waitable_set.take() {
            waitable_set_drop(set);
        }
    }
}

/// The current task being polled (or null if none).
static mut CURRENT: *mut FutureState = ptr::null_mut();

/// Map of any in-progress calls to async-lowered imports, keyed by the
/// identifiers issued by the host.
static mut CALLS: Lazy<HashMap<i32, oneshot::Sender<u32>>> = Lazy::new(HashMap::new);

/// Any newly-deferred work queued by calls to the `spawn` function while
/// polling the current task.
static mut SPAWNED: Vec<BoxFuture> = Vec::new();

const EVENT_NONE: i32 = 0;
const _EVENT_CALL_STARTING: i32 = 1;
const EVENT_CALL_STARTED: i32 = 2;
const EVENT_CALL_RETURNED: i32 = 3;
const EVENT_STREAM_READ: i32 = 5;
const EVENT_STREAM_WRITE: i32 = 6;
const EVENT_FUTURE_READ: i32 = 7;
const EVENT_FUTURE_WRITE: i32 = 8;

const CALLBACK_CODE_EXIT: i32 = 0;
const _CALLBACK_CODE_YIELD: i32 = 1;
const CALLBACK_CODE_WAIT: i32 = 2;
const _CALLBACK_CODE_POLL: i32 = 3;

const STATUS_STARTING: u32 = 1;
const STATUS_STARTED: u32 = 2;
const STATUS_RETURNED: u32 = 3;

/// Poll the specified task until it either completes or can't make immediate
/// progress.
unsafe fn poll(state: *mut FutureState) -> Poll<()> {
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
    (*state).wasip3_task.ptr = state.cast();
    let prev = cabi::wasip3_task_set(&mut (*state).wasip3_task);
    let _reset = ResetTask(prev);

    loop {
        if let Some(futures) = (*state).tasks.as_mut() {
            let old = CURRENT;
            CURRENT = state;
            let waker: Arc<FutureWaker> = Arc::default();
            let poll =
                futures.poll_next_unpin(&mut Context::from_waker(&Arc::clone(&waker).into()));
            CURRENT = old;

            if SPAWNED.is_empty() {
                match poll {
                    Poll::Ready(Some(())) => (),
                    Poll::Ready(None) => {
                        (*state).tasks = None;
                        break Poll::Ready(());
                    }
                    Poll::Pending => {
                        // TODO: Return `CallbackCode.YIELD` (see
                        // https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md#canon-lift)
                        // to the host before polling again once a host
                        // implementation exists to support it.
                        if !waker.0.load(Ordering::Relaxed) {
                            break Poll::Pending;
                        }
                    }
                }
            } else {
                futures.extend(SPAWNED.drain(..));
            }
        } else {
            break Poll::Ready(());
        }
    }
}

/// Poll the future generated by a call to an async-lifted export once, calling
/// the specified closure (presumably backed by a call to `task.return`) when it
/// generates a value.
///
/// This will return a non-null pointer representing the task if it hasn't
/// completed immediately; otherwise it returns null.
#[doc(hidden)]
pub fn first_poll<T: 'static>(
    future: impl Future<Output = T> + 'static,
    fun: impl FnOnce(&T) + 'static,
) -> i32 {
    let state = Box::into_raw(Box::new(FutureState::new(Box::pin(
        future.map(|v| fun(&v)),
    ))));
    let done = unsafe { poll(state).is_ready() };
    unsafe { callback_code(state, done) }
}

/// Await the completion of a call to an async-lowered import.
#[doc(hidden)]
pub async unsafe fn await_result(
    import: unsafe extern "C" fn(*mut u8, *mut u8) -> i32,
    params: *mut u8,
    results: *mut u8,
) {
    let result = import(params, results) as u32;
    let status = result >> 30;
    let call = (result & !(0b11 << 30)) as i32;

    if status != STATUS_RETURNED {
        assert!(!CURRENT.is_null());
        (*CURRENT).todo += 1;
    }

    let trap_on_drop = TrapOnDrop;

    match status {
        STATUS_STARTING | STATUS_STARTED => {
            (*CURRENT).add_waitable(call as u32);
            let (tx, rx) = oneshot::channel();
            CALLS.insert(call, tx);
            rx.await.unwrap();
        }
        STATUS_RETURNED => {}
        _ => unreachable!("unrecognized async call status"),
    }

    mem::forget(trap_on_drop);

    struct TrapOnDrop;

    impl Drop for TrapOnDrop {
        fn drop(&mut self) {
            trap_because_of_future_drop();
        }
    }
}

#[cold]
fn trap_because_of_future_drop() {
    panic!(
        "an imported function is being dropped/cancelled before being fully \
         awaited, but that is not sound at this time so the program is going \
         to be aborted; for more information see \
         https://github.com/bytecodealliance/wit-bindgen/issues/1175"
    );
}

/// stream/future read/write results defined by the Component Model ABI.
mod results {
    pub const BLOCKED: u32 = 0xffff_ffff;
    pub const CLOSED: u32 = 0x8000_0000;
    pub const CANCELED: u32 = 0;
}

/// Call the `subtask.drop` canonical built-in function.
fn subtask_drop(subtask: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn subtask_drop(_: u32) {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[subtask-drop]"]
        fn subtask_drop(_: u32);
    }
    unsafe { subtask_drop(subtask) }
}

unsafe fn callback_code(state: *mut FutureState, done: bool) -> i32 {
    if done && !(*state).remaining_work() {
        context_set(0);
        drop(Box::from_raw(state));
        CALLBACK_CODE_EXIT
    } else {
        context_set(i32::try_from(state as isize).unwrap() as u32);
        CALLBACK_CODE_WAIT | (((*state).waitable_set.unwrap() as i32) << 4)
    }
}

/// Handle a progress notification from the host regarding either a call to an
/// async-lowered import or a stream/future read/write operation.
#[doc(hidden)]
pub unsafe fn callback(event0: i32, event1: i32, event2: i32) -> i32 {
    let state = isize::try_from(context_get()).unwrap() as *mut FutureState;
    assert!(!state.is_null());

    callback_with_state(state, event0, event1, event2)
}

unsafe fn callback_with_state(
    state: *mut FutureState,
    event0: i32,
    event1: i32,
    event2: i32,
) -> i32 {
    match event0 {
        EVENT_NONE => {
            rtdebug!("EVENT_NONE");
            let done = poll(state).is_ready();
            callback_code(state, done)
        }
        EVENT_CALL_STARTED => {
            rtdebug!("EVENT_CALL_STARTED");
            callback_code(state, false)
        }
        EVENT_CALL_RETURNED => {
            rtdebug!("EVENT_CALL_RETURNED({event1:#x}, {event2:#x})");
            (*state).remove_waitable(event1 as _);

            if let Some(call) = CALLS.remove(&event1) {
                _ = call.send(event2 as _);
            }

            let done = poll(state).is_ready();

            if event0 == EVENT_CALL_RETURNED {
                subtask_drop(event1 as u32);
            }

            (*state).todo -= 1;

            callback_code(state, done)
        }

        EVENT_STREAM_READ | EVENT_STREAM_WRITE | EVENT_FUTURE_READ | EVENT_FUTURE_WRITE => {
            rtdebug!(
                "EVENT_{{STREAM,FUTURE}}_{{READ,WRITE}}({event0:#x}, {event1:#x}, {event2:#x})"
            );
            (*state).remove_waitable(event1 as u32);
            let (ptr, callback) = (*state).waitables.remove(&(event1 as u32)).unwrap();
            callback(ptr, event2 as u32);

            let done = poll(state).is_ready();
            callback_code(state, done)
        }

        _ => unreachable!(),
    }
}

/// Represents the Component Model `error-context` type.
#[derive(PartialEq, Eq)]
pub struct ErrorContext {
    handle: u32,
}

impl ErrorContext {
    /// Call the `error-context.new` canonical built-in function.
    pub fn new(debug_message: &str) -> ErrorContext {
        #[cfg(not(target_arch = "wasm32"))]
        unsafe fn context_new(_: *const u8, _: usize) -> i32 {
            unreachable!()
        }

        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "$root")]
        extern "C" {
            #[link_name = "[error-context-new-utf8]"]
            fn context_new(_: *const u8, _: usize) -> i32;
        }

        unsafe {
            let handle = context_new(debug_message.as_ptr(), debug_message.len());
            // SAFETY: Handles (including error context handles are guaranteed to
            // fit inside u32 by the Component Model ABI
            ErrorContext::from_handle(u32::try_from(handle).unwrap())
        }
    }

    #[doc(hidden)]
    pub fn from_handle(handle: u32) -> Self {
        Self { handle }
    }

    #[doc(hidden)]
    pub fn handle(&self) -> u32 {
        self.handle
    }

    /// Extract the debug message from a given [`ErrorContext`]
    pub fn debug_message(&self) -> String {
        #[repr(C)]
        struct RetPtr {
            ptr: *mut u8,
            len: usize,
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn error_context_debug_message(_: u32, _: &mut RetPtr) {
            unreachable!()
        }

        #[cfg(target_arch = "wasm32")]
        #[link(wasm_import_module = "$root")]
        extern "C" {
            #[link_name = "[error-context-debug-message-utf8]"]
            fn error_context_debug_message(_: u32, _: &mut RetPtr);
        }

        unsafe {
            let mut ret = RetPtr {
                ptr: ptr::null_mut(),
                len: 0,
            };
            error_context_debug_message(self.handle, &mut ret);
            String::from_raw_parts(ret.ptr, ret.len, ret.len)
        }
    }
}

impl Debug for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorContext")
            .field("debug_message", &self.debug_message())
            .finish()
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.debug_message(), f)
    }
}

impl std::error::Error for ErrorContext {}

impl Drop for ErrorContext {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            #[link(wasm_import_module = "$root")]
            extern "C" {
                #[link_name = "[error-context-drop]"]
                fn error_drop(_: u32);
            }
            if self.handle != 0 {
                unsafe { error_drop(self.handle) }
            }
        }
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
    loop {
        match unsafe { poll(state) } {
            Poll::Ready(()) => break rx.try_recv().unwrap().unwrap(),
            Poll::Pending => waitable_set_wait(state),
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

fn context_get() -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn get() -> u32 {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[context-get-1]"]
        fn get() -> u32;
    }

    unsafe { get() }
}

fn context_set(value: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn set(_: u32) {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[context-set-1]"]
        fn set(value: u32);
    }

    unsafe { set(value) }
}

fn waitable_set_new() -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn new() -> u32 {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[waitable-set-new]"]
        fn new() -> u32;
    }

    unsafe { new() }
}

fn waitable_set_drop(set: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn drop(_: u32) {
        unreachable!()
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[waitable-set-drop]"]
        fn drop(set: u32);
    }

    unsafe { drop(set) }
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn waitable_join(_: u32, _: u32) {
    unreachable!()
}
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "$root")]
extern "C" {
    #[link_name = "[waitable-join]"]
    fn waitable_join(waitable: u32, set: u32);
}

fn waitable_set_wait(state: &mut FutureState) {
    #[cfg(not(target_arch = "wasm32"))]
    unsafe fn wait(_: u32, _: *mut i32) -> i32 {
        unreachable!();
    }

    #[cfg(target_arch = "wasm32")]
    #[link(wasm_import_module = "$root")]
    extern "C" {
        #[link_name = "[waitable-set-wait]"]
        fn wait(_: u32, _: *mut i32) -> i32;
    }

    unsafe {
        let mut payload = [0i32; 2];
        let event0 = wait(state.get_or_create_waitable_set(), payload.as_mut_ptr());
        callback_with_state(state as *mut _, event0, payload[0], payload[1]);
    }
}
