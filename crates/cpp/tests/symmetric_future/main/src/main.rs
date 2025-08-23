use wit_bindgen_symmetric_rt::{
    async_support::Stream,
    symmetric_stream::{Address, Buffer},
    CallbackState,
};

#[link(name = "future")]
extern "C" {
    pub fn testX3AtestX2Ffuture_testX00create() -> usize;
}

struct CallbackInfo {
    stream: Stream,
    data: u32,
}

extern "C" fn ready(arg: *mut ()) -> CallbackState {
    let info = unsafe { &*arg.cast::<CallbackInfo>() };
    let buffer = info.stream.read_result();
    if let Some(buffer) = buffer {
        let len = buffer.get_size();
        if len > 0 {
            println!("data {}", info.data);
        }
    }
    // finished
    CallbackState::Ready
}

fn main() {
    let result_future = unsafe { testX3AtestX2Ffuture_testX00create() };
    // function should have completed (not async)
    // assert!(continuation.is_null());
    let stream = unsafe { Stream::from_handle(result_future) };
    let mut info = Box::pin(CallbackInfo {
        stream: stream.clone(),
        data: 0,
    });
    let buffer = Buffer::new(
        unsafe { Address::from_handle(&mut info.data as *mut u32 as usize) },
        1,
    );
    stream.start_reading(buffer);
    let subscription = stream.read_ready_subscribe();
    println!("Register read in main");
    wit_bindgen_symmetric_rt::register(
        subscription,
        ready,
        (&*info as *const CallbackInfo).cast_mut().cast(),
    );
    wit_bindgen_symmetric_rt::run();
}
