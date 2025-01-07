use wit_bindgen_symmetric_rt::{
    async_support::{stream, Stream},
    CallbackState,
};

#[link(name = "stream")]
extern "C" {
    pub fn testX3AtestX2Fstream_testX00X5BasyncX5Dcreate(
        // args: *const (),
        results: *mut (),
    ) -> *mut ();
}

const DATALEN: usize = 2;

struct CallbackInfo {
    stream: *mut Stream,
    data: [u32; DATALEN],
}

extern "C" fn ready(arg: *mut ()) -> CallbackState {
    let info = unsafe { &*arg.cast::<CallbackInfo>() };
    let len = unsafe { stream::read_amount(info.stream) };
    if len > 0 {
        for i in 0..len as usize {
            println!("data {}", info.data[i]);
        }
        unsafe {
            stream::start_reading(info.stream, info.data.as_ptr().cast_mut().cast(), DATALEN);
        };
        // call again
        CallbackState::Pending
    } else {
        // finished
        CallbackState::Ready
    }
}

fn main() {
    let mut result_stream: *mut () = core::ptr::null_mut();
    let continuation = unsafe {
        testX3AtestX2Fstream_testX00X5BasyncX5Dcreate(
            // core::ptr::null_mut(),
            (&mut result_stream as *mut *mut ()).cast(),
        )
    };
    // function should have completed (not async)
    assert!(continuation.is_null());
    let stream = result_stream.cast::<Stream>();
    let mut info = Box::pin(CallbackInfo {
        stream,
        data: [0, 0],
    });
    unsafe {
        stream::start_reading(stream, info.data.as_mut_ptr().cast(), DATALEN);
    };
    let subscription = unsafe {
        wit_bindgen_symmetric_rt::subscribe_event_send_ptr(stream::read_ready_event(stream))
    };
    println!("Register read in main");
    wit_bindgen_symmetric_rt::register(
        subscription,
        ready,
        (&*info as *const CallbackInfo).cast_mut().cast(),
    );
    wit_bindgen_symmetric_rt::run();
    unsafe { stream::close_read(stream) };
}
