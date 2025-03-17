use wit_bindgen_symmetric_rt::{
    async_support::Stream,
    symmetric_stream::{Address, Buffer},
    CallbackState,
};

#[link(name = "stream")]
extern "C" {
    pub fn testX3AtestX2Fstream_testX00create() -> usize;
}

const DATALEN: usize = 2;

struct CallbackInfo {
    stream: Stream,
    data: [u32; DATALEN],
}

extern "C" fn ready(arg: *mut ()) -> CallbackState {
    let info = unsafe { &*arg.cast::<CallbackInfo>() };
    let buffer = info.stream.read_result();
    if let Some(buffer) = buffer {
        let len = buffer.get_size();
        for i in 0..len as usize {
            println!("data {}", info.data[i]);
        }
        info.stream.start_reading(buffer);
        // call again
        CallbackState::Pending
    } else {
        // finished
        CallbackState::Ready
    }
}

fn main() {
    // let mut result_stream: *mut () = core::ptr::null_mut();
    let result_stream = unsafe { testX3AtestX2Fstream_testX00create() };
    // function should have completed (not async)
    // assert!(continuation.is_null());
    let stream = unsafe { Stream::from_handle(result_stream) };
    let mut info = Box::pin(CallbackInfo {
        stream: stream.clone(),
        data: [0, 0],
    });
    let buffer = Buffer::new(
        unsafe { Address::from_handle(info.data.as_mut_ptr() as usize) },
        DATALEN as u64,
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
