// use wit_bindgen_symmetric_rt::{CallbackState, EventSubscription};

use std::pin::pin;

use wit_bindgen_symmetric_rt::{async_support::Stream, CallbackState};

#[link(name = "stream")]
extern "C" {
    pub fn testX3AtestX2Fstream_testX00X5BasyncX5Dcreate(
        args: *const (),
        results: *mut (),
    ) -> *mut ();
}

extern "C" fn ready(_arg: *mut ()) -> CallbackState {
    todo!()
}

fn main() {
    let mut result_stream: *mut () = core::ptr::null_mut();
    let handle = unsafe {
        testX3AtestX2Fstream_testX00X5BasyncX5Dcreate(
            core::ptr::null_mut(),
            (&mut result_stream as *mut *mut ()).cast(),
        )
    };
    assert!(handle.is_null());
    let handle = result_stream.cast::<Stream>();
    let mut target = Box::pin([0_u32, 0]);
    unsafe {
        ((&*(&*handle).vtable).read)(handle, target.as_mut_ptr().cast(), 2);
    };
    let read_ready = unsafe { (&*handle).read_ready_event_send };
    let subscription = unsafe { wit_bindgen_symmetric_rt::subscribe_event_send_ptr(read_ready) };
    println!("Register read in main");
    wit_bindgen_symmetric_rt::register(subscription, ready, ((&mut *target) as *mut u32).cast());
    wit_bindgen_symmetric_rt::run();
}
