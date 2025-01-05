use std::sync::atomic::{AtomicU32, Ordering};

use wit_bindgen_symmetric_rt::{
    async_support::{self, results, stream::Slice, Stream},
    register, subscribe_event_send_ptr, CallbackState, EventSubscription,
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
    let stream: *mut Stream = data.cast();
    if count <= 5 {
        let Slice { addr, size } = unsafe { async_support::stream::start_writing(stream) };
        // let size = unsafe { &*stream }.read_size.swap(0, Ordering::Acquire);
        // let addr = unsafe { &*stream }
        //     .read_addr
        //     .swap(core::ptr::null_mut(), Ordering::Relaxed);
        assert!(size >= 1);
        *unsafe { &mut *addr.cast::<u32>() } = count;
        unsafe { async_support::stream::finish_writing(stream, 1) };
        // let old_ready = unsafe { &*stream }.ready_size.swap(1, Ordering::Release);
        // assert_eq!(old_ready, results::BLOCKED);
        // unsafe { activate_event_send_ptr(async_support::stream::read_ready_event(stream)) };
        // let ms_30 = unsafe {
        //     symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(30*1_000_000)
        // } as usize;
        // assert_ne!(ms_30, 0);
        // let event = unsafe { EventSubscription::from_handle(ms_30) };
        // register(event, timer_call, data);
    }
    CallbackState::Ready
}

extern "C" fn write_ready(data: *mut ()) -> CallbackState {
    let count = COUNT.load(Ordering::SeqCst);
    if count > 5 {
        let stream: *mut Stream = data.cast();
        // EOF
        unsafe { async_support::stream::finish_writing(stream, results::CLOSED) };
        // let old_ready = unsafe { &*stream }
        //     .ready_size
        //     .swap(isize::MIN, Ordering::Release);
        // assert_eq!(old_ready, results::BLOCKED);
        // unsafe { activate_event_send_ptr(async_support::stream::read_ready_event(stream)) };
        CallbackState::Ready
    } else {
        if count == 1 {
            println!("we can write now, starting timer");
        }
        let ms_30 = unsafe {
            symmetricX3AruntimeX2Fsymmetric_executorX400X2E1X2E0X00X5BstaticX5Devent_subscriptionX2Efrom_timeout(30*1_000_000)
        } as usize;
        assert_ne!(ms_30, 0);
        let event = unsafe { EventSubscription::from_handle(ms_30) };
        register(event, timer_call, data);
        // this callback is done
        CallbackState::Pending
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub fn testX3AtestX2Fstream_sourceX00X5BasyncX5Dcreate(
    _args: *mut u8,
    results: *mut u8,
) -> *mut u8 {
    let obj = Box::new(Stream::new());
    let addr = Box::into_raw(obj);
    let event = unsafe { subscribe_event_send_ptr(async_support::stream::write_ready_event(addr)) };
    register(event, write_ready, addr.cast());
    *unsafe { &mut *results.cast::<*mut Stream>() } = addr;
    std::ptr::null_mut()
}
