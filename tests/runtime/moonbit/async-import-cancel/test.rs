include!(env!("BINDINGS"));

use crate::test::async_import_cancel::gate_control;
use crate::exports::test::async_import_cancel::pending::{
    Guest, GuestLeaf, Leaf, OwnedInput, ReturnedPayload,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use wit_bindgen::{
    FutureReader, FutureWriter, StreamReader, StreamResult, StreamWriter,
};

struct Component;

export!(Component);

static LEAF_LIVE_COUNT: AtomicU32 = AtomicU32::new(0);
static LEAF_DROP_COUNT: AtomicU32 = AtomicU32::new(0);
static PENDING_STARTED: AtomicBool = AtomicBool::new(false);
static PENDING_STREAM_STARTED: AtomicBool = AtomicBool::new(false);
static TAKE_AFTER_START_STARTED: AtomicBool = AtomicBool::new(false);
static OPEN_FUTURE_WRITER: Mutex<Option<FutureWriter<Leaf>>> = Mutex::new(None);
static RACE_FUTURE_WRITER: Mutex<Option<FutureWriter<Leaf>>> = Mutex::new(None);
static OPEN_STREAM_WRITER: Mutex<Option<StreamWriter<Leaf>>> = Mutex::new(None);
static RACE_STREAM_WRITER: Mutex<Option<StreamWriter<Leaf>>> = Mutex::new(None);

struct MyLeaf;

impl MyLeaf {
    fn tracked() -> Self {
        LEAF_LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Guest for Component {
    type Leaf = MyLeaf;

    async fn pending(value: FutureReader<Leaf>) {
        PENDING_STARTED.store(true, Ordering::SeqCst);
        let _ = value.await;
    }

    async fn consume_future_string(value: FutureReader<String>) -> bool {
        value.await == "hello"
    }

    fn pending_started() -> bool {
        PENDING_STARTED.load(Ordering::SeqCst)
    }

    async fn pending_stream(mut value: StreamReader<Leaf>) {
        PENDING_STREAM_STARTED.store(true, Ordering::SeqCst);
        let _ = value.read(Vec::with_capacity(1)).await;
    }

    fn pending_stream_started() -> bool {
        PENDING_STREAM_STARTED.load(Ordering::SeqCst)
    }

    fn open_future() -> FutureReader<Leaf> {
        let (writer, reader) = wit_future::new::<Leaf>(|| Leaf::new(MyLeaf::tracked()));
        assert!(OPEN_FUTURE_WRITER.lock().unwrap().replace(writer).is_none());
        reader
    }

    async fn open_future_reader_dropped() -> bool {
        let writer = OPEN_FUTURE_WRITER.lock().unwrap().take().unwrap();
        match writer.write(Leaf::new(MyLeaf::tracked())).await {
            Ok(()) => false,
            Err(error) => {
                drop(error.value);
                true
            }
        }
    }

    fn open_race_future() -> FutureReader<Leaf> {
        let (writer, reader) = wit_future::new::<Leaf>(|| Leaf::new(MyLeaf::tracked()));
        assert!(RACE_FUTURE_WRITER.lock().unwrap().replace(writer).is_none());
        reader
    }

    async fn complete_race_future() {
        let writer = RACE_FUTURE_WRITER.lock().unwrap().take().unwrap();
        writer
            .write(Leaf::new(MyLeaf::tracked()))
            .await
            .unwrap();
    }

    fn open_stream() -> StreamReader<Leaf> {
        let (writer, reader) = wit_stream::new::<Leaf>();
        assert!(OPEN_STREAM_WRITER.lock().unwrap().replace(writer).is_none());
        reader
    }

    async fn open_stream_reader_dropped() -> bool {
        let mut writer = OPEN_STREAM_WRITER.lock().unwrap().take().unwrap();
        let (result, remaining) = writer
            .write(vec![Leaf::new(MyLeaf::tracked())])
            .await;
        let dropped = result == StreamResult::Dropped && remaining.remaining() == 1;
        drop(remaining);
        dropped
    }

    fn open_race_stream() -> StreamReader<Leaf> {
        let (writer, reader) = wit_stream::new::<Leaf>();
        assert!(RACE_STREAM_WRITER.lock().unwrap().replace(writer).is_none());
        reader
    }

    async fn complete_race_stream() {
        let mut writer = RACE_STREAM_WRITER.lock().unwrap().take().unwrap();
        let (result, remaining) = writer
            .write(vec![Leaf::new(MyLeaf::tracked())])
            .await;
        assert_eq!(result, StreamResult::Complete(1));
        assert_eq!(remaining.remaining(), 0);
    }

    async fn drop_future_stream(value: StreamReader<FutureReader<Leaf>>) {
        drop(value);
    }

    async fn take_after_start(_value: OwnedInput) {
        TAKE_AFTER_START_STARTED.store(true, Ordering::SeqCst);
        loop {
            wit_bindgen::yield_async().await;
        }
    }

    fn take_after_start_started() -> bool {
        TAKE_AFTER_START_STARTED.load(Ordering::SeqCst)
    }

    async fn return_after_cancel(value: Leaf) -> ReturnedPayload {
        gate_control::mark_started();
        while !gate_control::released() {
            let _ = wit_bindgen::yield_blocking();
        }
        let (ready_writer, ready) = wit_future::new::<Leaf>(|| Leaf::new(MyLeaf::tracked()));
        drop(ready_writer);
        ReturnedPayload {
            label: "returned after cancellation".into(),
            leaf: value,
            leaves: vec![Leaf::new(MyLeaf::tracked())],
            ready,
        }
    }

    async fn settle() {
        let _ = wit_bindgen::yield_blocking();
    }

    fn backpressure_set(value: bool) {
        if value {
            wit_bindgen::backpressure_inc();
        } else {
            wit_bindgen::backpressure_dec();
        }
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
        Self::tracked()
    }
}

impl Drop for MyLeaf {
    fn drop(&mut self) {
        LEAF_LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
        LEAF_DROP_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}
