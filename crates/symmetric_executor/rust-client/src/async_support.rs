use futures::{channel::oneshot, task::Waker, FutureExt};
use std::{
    any::Any,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicIsize, AtomicPtr, AtomicUsize, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable},
};

use crate::module::symmetric::runtime::symmetric_executor::{
    self, CallbackState, EventGenerator, EventSubscription,
};

// See https://github.com/rust-lang/rust/issues/13231 for the limitation
// / Send constraint on futures for spawn, loosen later
// pub unsafe auto trait MaybeSend : Send {}
// unsafe impl<T> MaybeSend for T where T: Send {}

// pub trait FutureMaybeSend<T> : Future<Output = T> + MaybeSend {}

type BoxFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

struct FutureState {
    future: BoxFuture,
    // signal to activate once the current async future has finished
    completion_event: Option<EventGenerator>,
    // the event this future should wake on
    waiting_for: Option<EventSubscription>,
}

#[doc(hidden)]
pub enum Handle {
    LocalOpen,
    LocalReady(Box<dyn Any>, Waker),
    LocalWaiting(oneshot::Sender<Box<dyn Any>>),
    LocalClosed,
    Read,
    Write,
}

#[repr(C)]
pub struct StreamVtable {
    // magic value for EOF(-1) and block(MIN)
    // asynchronous function, if this blocks wait for read ready event
    pub read: extern "C" fn(stream: *mut Stream, buf: *mut (), size: usize) -> isize,
    pub close_read: extern "C" fn(stream: *mut Stream),

    pub write: extern "C" fn(stream: *mut Stream, buf: *mut (), size: usize) -> isize,
    pub close_write: extern "C" fn(stream: *mut Stream),
    // post WASI 0.3, CPB
    // pub allocate: fn(stream: *mut ()) -> (*mut (), isize),
    // pub publish: fn(stream: *mut (), size: usize),
}

#[repr(C)]
pub struct Stream {
    pub vtable: *const StreamVtable,
    pub read_ready_event_send: *mut (),
    pub write_ready_event_send: *mut (),
    pub read_addr: AtomicPtr<()>,
    pub read_size: AtomicUsize,
    pub ready_size: AtomicIsize,
}

extern "C" fn read_impl(stream: *mut Stream, buf: *mut (), size: usize) -> isize {
    let old_size = unsafe { &mut *stream }
        .read_size
        .swap(size, Ordering::Acquire);
    assert_eq!(old_size, 0);
    let old_ptr = unsafe { &mut *stream }
        .read_addr
        .swap(buf, Ordering::Release);
    assert_eq!(old_ptr, std::ptr::null_mut());
    let write_evt = unsafe { &mut *stream }.write_ready_event_send;
    let gen: EventGenerator = unsafe { EventGenerator::from_handle(write_evt as usize) };
    gen.activate();
    // don't consume
    let _ = gen.take_handle();
    isize::MIN
}

extern "C" fn write_impl(_stream: *mut Stream, _buf: *mut (), _size: usize) -> isize {
    todo!()
}

extern "C" fn read_close_impl(_stream: *mut Stream) {
    todo!()
}

extern "C" fn write_close_impl(_stream: *mut Stream) {
    todo!()
}

const STREAM_VTABLE: StreamVtable = StreamVtable {
    read: read_impl,
    close_read: read_close_impl,
    write: write_impl,
    close_write: write_close_impl,
};

impl Stream {
    pub fn new() -> Self {
        Self {
            vtable: &STREAM_VTABLE as *const StreamVtable,
            read_ready_event_send: EventGenerator::new().take_handle() as *mut (),
            write_ready_event_send: EventGenerator::new().take_handle() as *mut (),
            read_addr: AtomicPtr::new(core::ptr::null_mut()),
            read_size: AtomicUsize::new(0),
            ready_size: AtomicIsize::new(0),
        }
    }
}

// pub enum Entry<'a, K, V> {
//     Vacant(),
//     Occupied(&'a mut Stream),
// }

// #[doc(hidden)]
// pub fn with_entry<T>(h: *mut (), f: impl FnOnce(Entry<'_, u32, Handle>) -> T) -> T {
//     let entry = unsafe { &mut *(h.cast::<Stream>()) };
//     f(hash_map::Entry::Occupied(entry))
// }

static VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(core::ptr::null(), &VTABLE),
    // `wake` does nothing
    |_| {},
    // `wake_by_ref` does nothing
    |_| {},
    // Dropping does nothing as we don't allocate anything
    |_| {},
);

pub fn new_waker(waiting_for_ptr: *mut Option<EventSubscription>) -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(waiting_for_ptr.cast(), &VTABLE)) }
}

unsafe fn poll(state: *mut FutureState) -> Poll<()> {
    let mut pinned = std::pin::pin!(&mut (*state).future);
    let waker = new_waker(&mut (&mut *state).waiting_for as *mut Option<EventSubscription>);
    pinned
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
        .map(|()| {
            let state_owned = Box::from_raw(state);
            if let Some(waker) = &state_owned.completion_event {
                waker.activate();
            }
            drop(state_owned);
        })
}

async fn wait_on(wait_for: &EventSubscription) {
    std::future::poll_fn(move |cx| {
        if wait_for.ready() {
            Poll::Ready(())
        } else {
            let data = cx.waker().data();
            // dangerous duplication?
            let wait_for_copy = unsafe { EventSubscription::from_handle(wait_for.handle()) };
            let old_waiting_for =
                unsafe { &mut *(data.cast::<Option<EventSubscription>>().cast_mut()) }
                    .replace(wait_for_copy);
            // don't free the old subscription we found
            if let Some(subscription) = old_waiting_for {
                subscription.take_handle();
            }
            Poll::Pending
        }
    })
    .await
}

extern "C" fn symmetric_callback(obj: *mut ()) -> symmetric_executor::CallbackState {
    match unsafe { poll(obj.cast()) } {
        Poll::Ready(_) => CallbackState::Ready,
        Poll::Pending => {
            let state = obj.cast::<FutureState>();
            let waiting_for = unsafe { &mut *state }.waiting_for.take();
            super::register(waiting_for.unwrap(), symmetric_callback, obj);
            CallbackState::Pending
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
    fun: impl FnOnce(T) + 'static,
) -> *mut () {
    let state = Box::into_raw(Box::new(FutureState {
        future: Box::pin(future.map(fun)),
        completion_event: None,
        waiting_for: None,
    }));
    match unsafe { poll(state) } {
        Poll::Ready(()) => core::ptr::null_mut(),
        Poll::Pending => {
            let completion_event = EventGenerator::new();
            let wait_chain = completion_event.subscribe().take_handle() as *mut ();
            unsafe { &mut *state }
                .completion_event
                .replace(completion_event);
            let waiting_for = unsafe { &mut *state }.waiting_for.take();
            super::register(waiting_for.unwrap(), symmetric_callback, state.cast());
            wait_chain
        }
    }
}

/// Await the completion of a call to an async-lowered import.
#[doc(hidden)]
pub async unsafe fn await_result(
    function: unsafe extern "C" fn(*mut u8, *mut u8) -> *mut u8,
    //    _params_layout: Layout,
    params: *mut u8,
    results: *mut u8,
) {
    let wait_for = function(params, results);
    if !wait_for.is_null() {
        let wait_for = unsafe { EventSubscription::from_handle(wait_for as usize) };
        wait_on(&wait_for).await;
        let _ = wait_for.take_handle();
    }
}

#[doc(hidden)]
pub unsafe fn callback(_ctx: *mut u8, _event0: i32, _event1: i32, _event2: i32) -> i32 {
    todo!()
}

// static TASKS: Mutex<Vec<Box<dyn Future<Output = ()> + 'static + Send>>> = Mutex::new(Vec::new());

pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    let _ = first_poll(future, |()| ());
    //    TASKS.lock().unwrap().push(Box::new(future));
}

mod results {
    pub const BLOCKED: isize = -1;
    pub const CLOSED: isize = isize::MIN;
    pub const CANCELED: isize = 0;
}

// Stream handles are Send, so wrap them
#[repr(transparent)]
pub struct StreamHandle2(pub *mut Stream);
unsafe impl Send for StreamHandle2 {}
unsafe impl Sync for StreamHandle2 {}

#[repr(transparent)]
pub struct AddressSend(pub *mut ());
unsafe impl Send for AddressSend {}
// unsafe impl Sync for StreamHandle2 {}

// pub enum SubscriptionType {
//     None,
//     Read(EventSubscription),
//     Write(EventSubscription),
// }

pub struct StreamHandleRust {
    pub handle: StreamHandle2,
    pub event: EventSubscription,
}

pub async unsafe fn await_stream_result(
    import: unsafe extern "C" fn(*mut Stream, *mut (), usize) -> isize,
    stream: StreamHandle2,
    address: AddressSend,
    count: usize,
    event: &EventSubscription,
) -> Option<usize> {
    let stream_copy = stream.0;
    let result = import(stream_copy, address.0, count);
    match result {
        results::BLOCKED => {
            // assert!(!CURRENT.is_null());
            // (*CURRENT).todo += 1;
            // let (tx, rx) = oneshot::channel();
            // CALLS.insert(stream as _, tx);
            wait_on(event).await;
            let v = unsafe { &mut *stream.0 }
                .ready_size
                .swap(0, Ordering::SeqCst);
            if let results::CLOSED | results::CANCELED = v {
                None
            } else {
                Some(usize::try_from(v).unwrap())
            }
        }
        results::CLOSED | results::CANCELED => None,
        v => Some(usize::try_from(v).unwrap()),
    }
}
