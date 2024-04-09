use {
    futures::{channel::oneshot, future::FutureExt},
    once_cell::sync::Lazy,
    std::{
        alloc::Layout,
        collections::HashMap,
        future::Future,
        pin::{pin, Pin},
        ptr,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    },
};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

struct FutureState(BoxFuture);

static mut CALLS: Lazy<HashMap<i32, oneshot::Sender<()>>> = Lazy::new(HashMap::new);

fn dummy_waker() -> Waker {
    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }

    static WAKER: Lazy<Arc<DummyWaker>> = Lazy::new(|| Arc::new(DummyWaker));

    WAKER.clone().into()
}

pub fn first_poll<T: 'static>(
    future: impl Future<Output = T> + 'static,
    fun: impl FnOnce(T) + 'static,
) -> *mut u8 {
    let mut future = Box::pin(future.map(fun)) as BoxFuture;

    match future
        .as_mut()
        .poll(&mut Context::from_waker(&dummy_waker()))
    {
        Poll::Ready(()) => ptr::null_mut(),
        Poll::Pending => Box::into_raw(Box::new(FutureState(future))) as _,
    }
}

pub async unsafe fn await_result(
    import: unsafe extern "C" fn(*mut u8, *mut u8, *mut u8) -> i32,
    params_layout: Layout,
    params: *mut u8,
    results: *mut u8,
    call: *mut u8,
) {
    const STATUS_NOT_STARTED: i32 = 0;
    const STATUS_PARAMS_READ: i32 = 1;
    const STATUS_RESULTS_WRITTEN: i32 = 2;
    const STATUS_DONE: i32 = 3;

    match import(params, results, call) {
        STATUS_PARAMS_READ => {
            alloc::dealloc(params, params_layout);
            let (tx, rx) = oneshot::channel();
            CALLS.insert(*call.cast::<i32>(), tx);
            rx.await.unwrap()
        }
        STATUS_DONE => {
            alloc::dealloc(params, params_layout);
        }
        _ => todo!(),
    }
}

pub unsafe fn callback(ctx: *mut u8, event0: i32, event1: i32, event2: i32) -> i32 {
    const EVENT_CALL_STARTED: i32 = 0;
    const EVENT_CALL_RETURNED: i32 = 1;
    const EVENT_CALL_DONE: i32 = 2;

    match event0 {
        EVENT_CALL_DONE => {
            CALLS.remove(&event1).unwrap().send(());

            match (*(ctx as *mut FutureState))
                .0
                .as_mut()
                .poll(&mut Context::from_waker(&dummy_waker()))
            {
                Poll::Ready(()) => {
                    // TODO: consider spawned task before returning "done" here
                    drop(Box::from_raw(ctx as *mut FutureState));
                    1
                }
                Poll::Pending => 0,
            }
        }
        _ => todo!(),
    }
}
