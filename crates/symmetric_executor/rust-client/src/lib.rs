use core::ptr::{self, NonNull};
use module::symmetric::runtime::symmetric_executor::{self, CallbackData, CallbackFunction};
pub use module::symmetric::runtime::symmetric_executor::{
    run, CallbackState, EventGenerator, EventSubscription,
};
pub use module::symmetric::runtime::symmetric_stream;
use std::alloc::{self, Layout};

pub mod async_support;
mod module;

// Re-export `bitflags` so that we can reference it from macros.
#[cfg(feature = "bitflags")]
#[doc(hidden)]
pub use bitflags;

pub struct EventSubscription2;
pub struct EventGenerator2;

pub fn register<T>(
    event: EventSubscription,
    f: extern "C" fn(*mut T) -> CallbackState,
    data: *mut T,
) {
    let callback = unsafe { CallbackFunction::from_handle(f as *const () as usize) };
    let cb_data = unsafe { CallbackData::from_handle(data as usize) };
    symmetric_executor::register(event, callback, cb_data);
}

// #[no_mangle]
// fn cabi_realloc_wit_bindgen_0_41_0(
//     _old_ptr: *mut u8,
//     _old_len: usize,
//     _align: usize,
//     _new_len: usize,
// ) -> *mut u8 {
//     todo!()
// }

pub unsafe fn subscribe_event_send_ptr(event_send: *mut EventGenerator2) -> EventSubscription {
    let gener: EventGenerator = unsafe { EventGenerator::from_handle(event_send as usize) };
    // (unsafe {Arc::from_raw(event_send.cast()) });
    let subscription = gener.subscribe();
    // avoid consuming the generator
    std::mem::forget(gener);
    subscription
}

pub unsafe fn activate_event_send_ptr(event_send: *mut EventGenerator2) {
    let gener: EventGenerator = unsafe { EventGenerator::from_handle(event_send as usize) };
    gener.activate();
    // avoid consuming the generator
    std::mem::forget(gener);
}

// stolen from guest-rust/rt/src/lib.rs
pub struct Cleanup {
    ptr: NonNull<u8>,
    layout: Layout,
}

// Usage of the returned pointer is always unsafe and must abide by these
// conventions, but this structure itself has no inherent reason to not be
// send/sync.
unsafe impl Send for Cleanup {}
unsafe impl Sync for Cleanup {}

impl Cleanup {
    pub fn new(layout: Layout) -> (*mut u8, Option<Cleanup>) {
        if layout.size() == 0 {
            return (ptr::null_mut(), None);
        }
        let ptr = unsafe { alloc::alloc(layout) };
        let ptr = match NonNull::new(ptr) {
            Some(ptr) => ptr,
            None => alloc::handle_alloc_error(layout),
        };
        (ptr.as_ptr(), Some(Cleanup { ptr, layout }))
    }
    pub fn forget(self) {
        core::mem::forget(self);
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}
