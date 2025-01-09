use std::sync::atomic::{AtomicU32, Ordering};

use wit_bindgen_symmetric_rt::{
    async_support::{
        results,
        stream_support::{self, Slice},
        Stream,
    },
    register, subscribe_event_send_ptr, CallbackState, EventSubscription,
};

static COUNT: AtomicU32 = AtomicU32::new(1);

extern "C" fn timer_call(data: *mut ()) -> CallbackState {
    let count = COUNT.fetch_add(1, Ordering::AcqRel);
    let stream: *mut Stream = data.cast();
    if count <= 5 {
        let Slice { addr, size } = unsafe { stream_support::start_writing(stream) };
        assert!(size >= 1);
        *unsafe { &mut *addr.cast::<u32>() } = count;
        unsafe { stream_support::finish_writing(stream, 1) };
    }
    CallbackState::Ready
}

extern "C" fn write_ready(data: *mut ()) -> CallbackState {
    let count = COUNT.load(Ordering::Acquire);
    if count > 5 {
        let stream: *mut Stream = data.cast();
        // EOF
        unsafe { stream_support::finish_writing(stream, results::CLOSED) };
        unsafe { stream_support::close_write(stream) };
        CallbackState::Ready
    } else {
        if count == 1 {
            println!("we can write now, starting timer");
        }
        let ms_30 = EventSubscription::from_timeout(30 * 1_000_000);
        register(ms_30, timer_call, data);
        CallbackState::Pending
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub fn testX3AtestX2Fstream_sourceX00X5BasyncX5Dcreate(
    // _args: *mut u8,
    results: *mut u8,
) -> *mut u8 {
    let stream = stream_support::create_stream();
    let event = unsafe { subscribe_event_send_ptr(stream_support::write_ready_event(stream)) };
    register(event, write_ready, stream.cast());
    *unsafe { &mut *results.cast::<*mut Stream>() } = stream;
    std::ptr::null_mut()
}
