include!(env!("BINDINGS"));

use crate::exports::test::moonbit_stream_write_cancel::holder::{
    Guest, GuestLeaf, Leaf,
};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::task::Waker;
use wit_bindgen::{StreamReader, StreamResult};

struct Component;

export!(Component);

static WRITE_STARTED: AtomicBool = AtomicBool::new(false);
static WRITE_STARTED_WAKER: Mutex<Option<Waker>> = Mutex::new(None);
static LEAF_LIVE_COUNT: AtomicU32 = AtomicU32::new(0);
static LEAF_DROP_COUNT: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static HELD_STREAM: RefCell<Option<StreamReader<Leaf>>> = const { RefCell::new(None) };
}

struct MyLeaf;

impl Guest for Component {
    type Leaf = MyLeaf;

    async fn hold(value: StreamReader<Leaf>) {
        HELD_STREAM.with(|stream| assert!(stream.borrow_mut().replace(value).is_none()));
    }

    fn mark_write_started() {
        WRITE_STARTED.store(true, Ordering::SeqCst);
        if let Some(waker) = WRITE_STARTED_WAKER.lock().unwrap().take() {
            waker.wake();
        }
    }

    fn write_started() -> bool {
        WRITE_STARTED.load(Ordering::SeqCst)
    }

    async fn wait_write_started() {
        std::future::poll_fn(|cx| {
            if WRITE_STARTED.load(Ordering::SeqCst) {
                std::task::Poll::Ready(())
            } else {
                let mut waker = WRITE_STARTED_WAKER.lock().unwrap();
                *waker = Some(cx.waker().clone());
                if WRITE_STARTED.load(Ordering::SeqCst) {
                    waker.take();
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            }
        })
        .await
    }

    async fn writable_dropped() -> bool {
        let mut stream = HELD_STREAM.with(|stream| stream.borrow_mut().take().unwrap());
        let (result, values) = stream.read(Vec::with_capacity(1)).await;
        result == StreamResult::Dropped && values.is_empty()
    }

    async fn read_one_and_keep() -> bool {
        let mut stream = HELD_STREAM.with(|stream| stream.borrow_mut().take().unwrap());
        let (result, values) = stream.read(Vec::with_capacity(1)).await;
        HELD_STREAM.with(|held| assert!(held.borrow_mut().replace(stream).is_none()));
        result == StreamResult::Complete(1) && values.len() == 1
    }

    async fn read_one_and_drop() -> bool {
        let mut stream = HELD_STREAM.with(|stream| stream.borrow_mut().take().unwrap());
        let (result, values) = stream.read(Vec::with_capacity(1)).await;
        result == StreamResult::Complete(1) && values.len() == 1
    }

    fn leaf_live_count() -> u32 {
        LEAF_LIVE_COUNT.load(Ordering::SeqCst)
    }

    fn leaf_drop_count() -> u32 {
        LEAF_DROP_COUNT.load(Ordering::SeqCst)
    }
}

impl GuestLeaf for MyLeaf {
    fn new() -> Self {
        LEAF_LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for MyLeaf {
    fn drop(&mut self) {
        LEAF_LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
        LEAF_DROP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}
