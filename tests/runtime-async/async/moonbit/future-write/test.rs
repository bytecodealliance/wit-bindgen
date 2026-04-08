use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));
use crate::exports::my::test::i::Thing;

struct Component;

struct MyThing {
    value: u32,
}

static ACTIVE_THINGS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

impl MyThing {
    fn make(value: u32) -> Self {
        ACTIVE_THINGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self { value }
    }
}

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    type Thing = MyThing;

    async fn create_future_with_value(value: u32) -> FutureReader<u32> {
        let (tx, rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            tx.write(value).await.unwrap();
        });
        rx
    }

    async fn create_unit_future() -> FutureReader<()> {
        let (tx, rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            tx.write(()).await.unwrap();
        });
        rx
    }

    async fn create_nested_future(value: u32) -> FutureReader<FutureReader<u32>> {
        let (inner_tx, inner_rx) = wit_future::new(|| unreachable!());
        let (outer_tx, outer_rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            outer_tx.write(inner_rx).await.unwrap();
            inner_tx.write(value).await.unwrap();
        });
        outer_rx
    }

    async fn create_nested_future_record(value: u32) -> crate::exports::my::test::i::NestedFutureRecord {
        let (inner_tx, inner_rx) = wit_future::new(|| unreachable!());
        let (outer_tx, outer_rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            outer_tx.write(inner_rx).await.unwrap();
            inner_tx.write(value).await.unwrap();
        });
        crate::exports::my::test::i::NestedFutureRecord { nested: outer_rx }
    }

    fn active_things() -> u32 {
        ACTIVE_THINGS.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn create_dropped_resource_future_with_signal(
        signal: FutureReader<()>,
    ) -> (FutureReader<Thing>, FutureReader<()>) {
        let (tx, rx) = wit_future::new(|| unreachable!());
        let (done_tx, done_rx) = wit_future::new(|| ());
        wit_bindgen::spawn(async move {
            signal.await;
            if let Err(err) = tx.write(Thing::new(MyThing::make(7))).await {
                drop(err.value);
            }
            done_tx.write(()).await.unwrap();
        });
        (rx, done_rx)
    }
}

impl crate::exports::my::test::i::GuestThing for MyThing {
    fn new(value: u32) -> Self {
        Self::make(value)
    }

    fn value(&self) -> u32 {
        self.value
    }
}

impl Drop for MyThing {
    fn drop(&mut self) {
        ACTIVE_THINGS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
