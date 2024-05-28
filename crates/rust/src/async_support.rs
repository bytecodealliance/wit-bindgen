use {
    futures::{
        channel::oneshot,
        future::FutureExt,
        sink::Sink,
        stream::{FuturesUnordered, Stream, StreamExt},
    },
    once_cell::sync::Lazy,
    std::{
        alloc::{self, Layout},
        collections::HashMap,
        fmt::{self, Debug, Display},
        future::{Future, IntoFuture},
        marker::PhantomData,
        mem::ManuallyDrop,
        pin::{pin, Pin},
        ptr,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    },
};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

struct FutureState(FuturesUnordered<BoxFuture>);

static mut CALLS: Lazy<HashMap<i32, oneshot::Sender<()>>> = Lazy::new(HashMap::new);

static mut SPAWNED: Vec<BoxFuture> = Vec::new();

fn dummy_waker() -> Waker {
    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }

    static WAKER: Lazy<Arc<DummyWaker>> = Lazy::new(|| Arc::new(DummyWaker));

    WAKER.clone().into()
}

unsafe fn poll(state: *mut FutureState) -> Poll<()> {
    loop {
        let poll = pin!((*state).0.next()).poll(&mut Context::from_waker(&dummy_waker()));

        if SPAWNED.is_empty() {
            match poll {
                Poll::Ready(Some(())) => (),
                Poll::Ready(None) => break Poll::Ready(()),
                Poll::Pending => break Poll::Pending,
            }
        } else {
            (*state).0.extend(SPAWNED.drain(..));
        }
    }
}

pub fn first_poll<T: 'static>(
    future: impl Future<Output = T> + 'static,
    fun: impl FnOnce(T) + 'static,
) -> *mut u8 {
    let state = Box::into_raw(Box::new(FutureState(
        [Box::pin(future.map(fun)) as BoxFuture]
            .into_iter()
            .collect(),
    )));
    match unsafe { poll(state) } {
        Poll::Ready(()) => ptr::null_mut(),
        Poll::Pending => state as _,
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
        STATUS_NOT_STARTED => {
            let (tx, rx) = oneshot::channel();
            CALLS.insert(*call.cast::<i32>(), tx);
            rx.await.unwrap();
            alloc::dealloc(params, params_layout);
        }
        STATUS_PARAMS_READ => {
            alloc::dealloc(params, params_layout);
            let (tx, rx) = oneshot::channel();
            CALLS.insert(*call.cast::<i32>(), tx);
            rx.await.unwrap()
        }
        STATUS_RESULTS_WRITTEN | STATUS_DONE => {
            alloc::dealloc(params, params_layout);
        }
        status => unreachable!(),
    }
}

pub unsafe fn callback(ctx: *mut u8, event0: i32, event1: i32, event2: i32) -> i32 {
    const EVENT_CALL_STARTED: i32 = 0;
    const EVENT_CALL_RETURNED: i32 = 1;
    const EVENT_CALL_DONE: i32 = 2;

    match event0 {
        EVENT_CALL_STARTED => {
            // TODO: could dealloc params here if we attached the pointer to the call
            1
        }
        EVENT_CALL_RETURNED | EVENT_CALL_DONE => {
            if let Some(call) = CALLS.remove(&event1) {
                call.send(());

                match poll(ctx as *mut FutureState) {
                    Poll::Ready(()) => {
                        drop(Box::from_raw(ctx as *mut FutureState));
                        1
                    }
                    Poll::Pending => 0,
                }
            } else {
                1
            }
        }
        _ => unreachable!(),
    }
}

#[doc(hidden)]
pub trait FuturePayload: Sized + 'static {
    fn new() -> (u32, u32);
    async fn send(sender: u32, value: Self) -> Result<(), Error>;
    async fn receive(receiver: u32) -> Result<Self, Error>;
    fn drop_sender(sender: u32);
    fn drop_receiver(receiver: u32);
}

pub struct FutureSender<T: FuturePayload> {
    handle: u32,
    _phantom: PhantomData<T>,
}

impl<T: FuturePayload> FutureSender<T> {
    pub async fn send(self, v: T) -> Result<(), Error> {
        T::send(ManuallyDrop::new(self).handle, v).await
    }
}

impl<T: FuturePayload> Drop for FutureSender<T> {
    fn drop(&mut self) {
        T::drop_sender(self.handle)
    }
}

pub struct FutureReceiver<T: FuturePayload> {
    handle: u32,
    _phantom: PhantomData<T>,
}

impl<T: FuturePayload> FutureReceiver<T> {
    #[doc(hidden)]
    pub fn into_handle(self) -> u32 {
        ManuallyDrop::new(self).handle
    }
}

impl<T: FuturePayload> IntoFuture for FutureReceiver<T> {
    type Output = Result<T, Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'static>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(T::receive(ManuallyDrop::new(self).handle))
    }
}

impl<T: FuturePayload> Drop for FutureReceiver<T> {
    fn drop(&mut self) {
        T::drop_receiver(self.handle)
    }
}

#[doc(hidden)]
pub trait StreamPayload: Unpin + Sized + 'static {
    fn new() -> (u32, u32);
    async fn send(sender: u32, values: Vec<Self>) -> Result<(), Error>;
    async fn receive(receiver: u32) -> Option<Result<Vec<Self>, Error>>;
    fn drop_sender(sender: u32);
    fn drop_receiver(receiver: u32);
}

pub struct StreamSender<T: StreamPayload> {
    handle: u32,
    future: Option<Pin<Box<dyn Future<Output = Result<(), Error>> + 'static>>>,
    _phantom: PhantomData<T>,
}

impl<T: StreamPayload> Sink<Vec<T>> for StreamSender<T> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        let me = self.get_mut();

        if let Some(future) = &mut me.future {
            match future.as_mut().poll(cx) {
                Poll::Ready(v) => {
                    me.future = None;
                    Poll::Ready(v)
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<T>) -> Result<(), Self::Error> {
        assert!(self.future.is_none());
        self.get_mut().future = Some(Box::pin(T::send(self.handle, item)));
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }
}

impl<T: StreamPayload> Drop for StreamSender<T> {
    fn drop(&mut self) {
        T::drop_sender(self.handle)
    }
}

pub struct StreamReceiver<T: StreamPayload> {
    handle: u32,
    future: Option<Pin<Box<dyn Future<Output = Option<Result<Vec<T>, Error>>> + 'static>>>,
    _phantom: PhantomData<T>,
}

impl<T: StreamPayload> StreamReceiver<T> {
    #[doc(hidden)]
    pub fn from_handle(handle: u32) -> Self {
        Self {
            handle,
            future: None,
            _phantom: PhantomData,
        }
    }

    #[doc(hidden)]
    pub fn into_handle(self) -> u32 {
        ManuallyDrop::new(self).handle
    }
}

impl<T: StreamPayload> Stream for StreamReceiver<T> {
    type Item = Result<Vec<T>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let me = self.get_mut();

        if me.future.is_none() {
            me.future = Some(Box::pin(T::receive(me.handle)));
        }

        match me.future.as_mut().unwrap().as_mut().poll(cx) {
            Poll::Ready(v) => {
                me.future = None;
                Poll::Ready(v)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: StreamPayload> Drop for StreamReceiver<T> {
    fn drop(&mut self) {
        T::drop_receiver(self.handle)
    }
}

pub struct Error {
    handle: u32,
}

impl Error {
    #[doc(hidden)]
    pub fn from_handle(handle: u32) -> Self {
        Self { handle }
    }

    #[doc(hidden)]
    pub fn handle(&self) -> u32 {
        self.handle
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error").finish()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error")
    }
}

impl std::error::Error for Error {}

impl Drop for Error {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            unreachable!();
        }

        #[cfg(target_arch = "wasm32")]
        {
            #[link(wasm_import_module = "$root")]
            extern "C" {
                #[link_name = "[error-drop]"]
                fn drop(_: u32);
            }
            if self.handle != 0 {
                unsafe { drop(self.handle) }
            }
        }
    }
}

pub fn new_future<T: FuturePayload>() -> (FutureSender<T>, FutureReceiver<T>) {
    let (tx, rx) = T::new();
    (
        FutureSender {
            handle: tx,
            _phantom: PhantomData,
        },
        FutureReceiver {
            handle: rx,
            _phantom: PhantomData,
        },
    )
}

pub fn new_stream<T: StreamPayload>() -> (StreamSender<T>, StreamReceiver<T>) {
    let (tx, rx) = T::new();
    (
        StreamSender {
            handle: tx,
            future: None,
            _phantom: PhantomData,
        },
        StreamReceiver {
            handle: rx,
            future: None,
            _phantom: PhantomData,
        },
    )
}

pub fn spawn(future: impl Future<Output = ()> + 'static) {
    unsafe { SPAWNED.push(Box::pin(future)) }
}

fn wait(state: &mut FutureState) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        unreachable!();
    }

    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "$root")]
        extern "C" {
            #[link_name = "[task-wait]"]
            fn wait(_: *mut i32) -> i32;
        }
        let mut payload = [0i32; 2];
        unsafe {
            let event0 = wait(payload.as_mut_ptr());
            callback(state as *mut _ as _, event0, payload[0], payload[1]);
        }
    }
}

// TODO: refactor so `'static` bounds aren't necessary
pub fn block_on<T: 'static>(future: impl Future<Output = T> + 'static) -> T {
    let (mut tx, mut rx) = oneshot::channel();
    let state = &mut FutureState(
        [Box::pin(future.map(move |v| drop(tx.send(v)))) as BoxFuture]
            .into_iter()
            .collect(),
    );
    loop {
        match unsafe { poll(state) } {
            Poll::Ready(()) => break rx.try_recv().unwrap().unwrap(),
            Poll::Pending => wait(state),
        }
    }
}
