use wit_bindgen_symmetric_rt::{
    async_support::Stream, register, subscribe_event_send_ptr, CallbackState,
};

extern "C" fn read_ready(data: *mut ()) -> CallbackState {
    CallbackState::Pending
}

#[allow(non_snake_case)]
#[no_mangle]
pub fn testX3AtestX2Fstream_sourceX00X5BasyncX5Dcreate(
    _args: *mut u8,
    results: *mut u8,
) -> *mut u8 {
    let obj = Box::new(Stream::new());
    let event = unsafe { subscribe_event_send_ptr(obj.read_ready_event_send) };
    let addr = Box::into_raw(obj);
    register(event, read_ready, addr.cast());
    *unsafe { &mut *results.cast::<*mut Stream>() } = addr;
    std::ptr::null_mut()
}
