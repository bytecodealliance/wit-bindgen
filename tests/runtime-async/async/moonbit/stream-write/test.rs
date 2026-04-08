use wit_bindgen::{FutureReader, StreamReader, StreamResult};

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

    async fn create_stream_with_values(count: u32) -> StreamReader<u32> {
        let (mut tx, rx) = wit_stream::new();
        wit_bindgen::spawn(async move {
            for i in 0..count {
                let (result, _rest) = tx.write(vec![i]).await;
                if !matches!(result, StreamResult::Complete(1)) {
                    break;
                }
            }
        });
        rx
    }

    async fn create_bridge_stream_with_signal(
        signal: FutureReader<()>,
    ) -> (StreamReader<u32>, FutureReader<bool>) {
        let (mut tx, rx) = wit_stream::new();
        let (done_tx, done_rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            let remaining = tx.write_all(vec![0]).await;
            assert!(remaining.is_empty());

            signal.await;

            let remaining = tx.write_all(vec![1, 2, 3]).await;
            done_tx.write(remaining.is_empty()).await.unwrap();
        });
        (rx, done_rx)
    }

    async fn create_unit_stream(count: u32) -> StreamReader<()> {
        let (mut tx, rx) = wit_stream::new();
        wit_bindgen::spawn(async move {
            for _ in 0..count {
                let (result, _rest) = tx.write(vec![()]).await;
                if !matches!(result, StreamResult::Complete(1)) {
                    break;
                }
            }
        });
        rx
    }

    fn active_things() -> u32 {
        ACTIVE_THINGS.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn create_bridge_unit_stream_with_signal(
        signal: FutureReader<()>,
    ) -> (StreamReader<()>, FutureReader<bool>) {
        let (mut tx, rx) = wit_stream::new();
        let (done_tx, done_rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            let remaining = tx.write_all(vec![()]).await;
            assert!(remaining.is_empty());

            signal.await;

            let remaining = tx.write_all(vec![(), (), ()]).await;
            done_tx.write(remaining.is_empty()).await.unwrap();
        });
        (rx, done_rx)
    }

    async fn create_bridge_thing_stream_with_signal(
        signal: FutureReader<()>,
    ) -> (StreamReader<Thing>, FutureReader<bool>) {
        let (mut tx, rx) = wit_stream::new();
        let (done_tx, done_rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn(async move {
            let remaining: Vec<Thing> = tx.write_all(vec![Thing::new(MyThing::make(0))]).await;
            assert!(remaining.is_empty());

            signal.await;

            let remaining: Vec<Thing> = tx
                .write_all(vec![
                    Thing::new(MyThing::make(1)),
                    Thing::new(MyThing::make(2)),
                    Thing::new(MyThing::make(3)),
                ])
                .await;
            let completed = remaining.is_empty();
            drop(remaining);
            done_tx.write(completed).await.unwrap();
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
