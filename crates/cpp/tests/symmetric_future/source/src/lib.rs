use wit_bindgen_symmetric_rt::{async_support::Stream, register, CallbackState, EventSubscription};

extern "C" fn timer_call(data: *mut ()) -> CallbackState {
    let stream: Stream = unsafe { Stream::from_handle(data as usize) };
    let buffer = stream.start_writing();
    let addr = buffer.get_address().take_handle() as *mut u32;
    let size = buffer.capacity();
    assert!(size >= 1);
    *unsafe { &mut *addr } = 21;
    buffer.set_size(1);
    stream.finish_writing(Some(buffer));
    // let _ = stream.take_handle();
    CallbackState::Ready
}

extern "C" fn write_ready(data: *mut ()) -> CallbackState {
    println!("we can write now, starting timer");
    let ms_30 = EventSubscription::from_timeout(30 * 1_000_000);
    register(ms_30, timer_call, data);
    CallbackState::Ready
}

#[allow(non_snake_case)]
#[no_mangle]
pub fn testX3AtestX2Ffuture_sourceX00create() -> usize {
    let stream = Stream::new();
    let event = stream.write_ready_subscribe();
    let stream_copy = stream.clone();
    register(event, write_ready, stream_copy.take_handle() as *mut ());
    stream.take_handle()
}
