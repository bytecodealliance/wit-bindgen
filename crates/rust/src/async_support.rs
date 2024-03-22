use std::{
    alloc::Layout,
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake},
};

pub fn first_poll<T: 'static>(future: impl Future<Output = T> + 'static) -> Result<T, *mut u8> {
    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }

    let mut future = pin!(future);

    match future
        .as_mut()
        .poll(&mut Context::from_waker(&Arc::new(DummyWaker).into()))
    {
        Poll::Ready(result) => Ok(result),
        Poll::Pending => todo!(),
    }
}

const STATUS_NOT_STARTED: i32 = 0;
const STATUS_PARAMS_READ: i32 = 1;
const STATUS_RESULTS_WRITTEN: i32 = 2;
const STATUS_DONE: i32 = 3;

pub async unsafe fn await_result(
    import: unsafe extern "C" fn(*mut u8, *mut u8, *mut u8) -> i32,
    params_layout: Layout,
    params: *mut u8,
    results: *mut u8,
    call: *mut u8,
) {
    match import(params, results, call) {
        STATUS_DONE => {
            alloc::dealloc(params, params_layout);
        }
        _ => todo!(),
    }
}
