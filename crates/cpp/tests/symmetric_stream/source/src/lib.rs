use std::sync::atomic::{AtomicU32, Ordering};

use wit_bindgen_symmetric_rt::{
    activate_event_send_ptr, async_support::Stream, register, subscribe_event_send_ptr,
    CallbackState, EventSubscription,
};

#[link(name = "symmetric_executor")]
extern "C" {
    fn symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(
        nanoseconds: u64,
    ) -> *mut ();
}

static COUNT: AtomicU32 = AtomicU32::new(1);

extern "C" fn timer_call(data: *mut ()) -> CallbackState {
    let count = COUNT.fetch_add(1, Ordering::SeqCst);
    let stream: *const Stream = data.cast();
    if count <= 5 {
        let size = unsafe { &*stream }.read_size.swap(0, Ordering::Acquire);
        let addr = unsafe { &*stream }
            .read_addr
            .swap(core::ptr::null_mut(), Ordering::Relaxed);
        assert!(size >= 1);
        *unsafe { &mut *addr.cast::<u32>() } = count;
        let old_ready = unsafe { &*stream }.ready_size.swap(1, Ordering::Release);
        assert_eq!(old_ready, 0);
        let read_ready_evt = unsafe { &*stream }.read_ready_event_send;
        unsafe { activate_event_send_ptr(read_ready_evt) };
        let ms_30 = unsafe {
            symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(30*1_000_000)
        } as usize;
        assert_ne!(ms_30, 0);
        let event = unsafe { EventSubscription::from_handle(ms_30) };
        register(event, timer_call, data);
    } else {
        // EOF
        let old_ready = unsafe { &*stream }
            .ready_size
            .swap(isize::MIN, Ordering::Release);
        assert_eq!(old_ready, 0);
        let read_ready_evt = unsafe { &*stream }.read_ready_event_send;
        unsafe { activate_event_send_ptr(read_ready_evt) };
    }
    CallbackState::Ready
}

extern "C" fn write_ready(data: *mut ()) -> CallbackState {
    println!("we can write now, starting timer");
    let ms_30 = unsafe {
        symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(30*1_000_000)
    } as usize;
    assert_ne!(ms_30, 0);
    let event = unsafe { EventSubscription::from_handle(ms_30) };
    register(event, timer_call, data);
    // this callback is done
    CallbackState::Ready
}

#[allow(non_snake_case)]
#[no_mangle]
pub fn testX3AtestX2Fstream_sourceX00X5BasyncX5Dcreate(
    _args: *mut u8,
    results: *mut u8,
) -> *mut u8 {
    let obj = Box::new(Stream::new());
    let event = unsafe { subscribe_event_send_ptr(obj.write_ready_event_send) };
    let addr = Box::into_raw(obj);
    register(event, write_ready, addr.cast());
    *unsafe { &mut *results.cast::<*mut Stream>() } = addr;
    std::ptr::null_mut()
}
