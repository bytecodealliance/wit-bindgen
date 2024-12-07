use futures::{channel::oneshot, task::Waker};
use std::{alloc::Layout, any::Any, collections::hash_map, future::Future};

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
pub fn with_entry<T>(h: u32, f: impl FnOnce(hash_map::Entry<'_, u32, Handle>) -> T) -> T {
    todo!()
}

#[doc(hidden)]
pub fn first_poll<T: 'static>(
    _future: impl Future<Output = T> + 'static,
    _fun: impl FnOnce(T) + 'static,
) -> *mut u8 {
    todo!()
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
