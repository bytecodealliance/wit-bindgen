use futures::{channel::oneshot, task::Waker, FutureExt};
use std::{
    alloc::Layout,
    any::Any,
    collections::hash_map,
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable},
};

use crate::module::symmetric::runtime::{self, symmetric_executor::EventGenerator};

use super::module::symmetric::runtime::symmetric_executor::EventSubscription;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

struct FutureState {
    future: BoxFuture,
    trigger: Option<EventGenerator>,
    active_subscription: Option<EventSubscription>,
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

#[doc(hidden)]
pub fn with_entry<T>(_h: u32, _f: impl FnOnce(hash_map::Entry<'_, u32, Handle>) -> T) -> T {
    todo!()
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(
    |_| RawWaker::new(core::ptr::null(), &VTABLE),
    // `wake` does nothing
    |_| {},
    // `wake_by_ref` does nothing
    |_| {},
    // Dropping does nothing as we don't allocate anything
    |_| {},
);

pub fn new_waker(call: *mut Option<EventSubscription>) -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(call.cast(), &VTABLE)) }
}

unsafe fn poll(state: *mut FutureState) -> Poll<()> {
    let mut pinned = std::pin::pin!(&mut (*state).future);
    let waker = new_waker(&mut (&mut *state).active_subscription as *mut Option<EventSubscription>);
    pinned
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
        .map(|()| {
            let dummy = Box::from_raw(state);
            if let Some(waker) = dummy.trigger {
                waker.activate();
            }
        })
}

#[doc(hidden)]
pub fn first_poll<T: 'static>(
    future: impl Future<Output = T> + 'static,
    fun: impl FnOnce(T) + 'static,
) -> *mut u8 {
    let state = Box::into_raw(Box::new(FutureState {
        future: Box::pin(future.map(fun)),
        trigger: None,
        active_subscription: None,
    }));
    match unsafe { poll(state) } {
        Poll::Ready(()) => core::ptr::null_mut(),
        Poll::Pending => {
            let trigger = EventGenerator::default();
            let subscription = unsafe { &mut *state }.active_subscription.take();
            assert!(!subscription.is_none());
            runtime::symmetric_executor::register(subscription.unwrap(), callback, unsafe {
                runtime::symmetric_executor::CallbackData::from_handle(state.cast())
            });
            let handle = trigger.subscribe().0.take_handle() as *mut ();
            unsafe { &mut *state }.trigger.replace(trigger);
            handle
        }
    }
}

#[doc(hidden)]
pub async unsafe fn await_result(
    _import: unsafe extern "C" fn(*mut u8, *mut u8) -> *mut u8,
    _params_layout: Layout,
    _params: *mut u8,
    _results: *mut u8,
) {
    todo!()
}

#[doc(hidden)]
pub unsafe fn callback(_ctx: *mut u8, _event0: i32, _event1: i32, _event2: i32) -> i32 {
    todo!()
}
