//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::{StreamResult, yield_async};

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // Test creating a stream with u32 values
        let mut rx = create_stream_with_values(3).await;
        let mut total = 0u32;
        let mut count = 0u32;
        loop {
            let buf = Vec::<u32>::with_capacity(10);
            let (result, values) = rx.read(buf).await;
            match result {
                StreamResult::Complete(n) if n > 0 => {
                    // Only process the first n items that were actually read
                    for v in values.iter().take(n) {
                        total += *v;
                        count += 1;
                    }
                }
                // Complete(0) means end of stream, or Dropped/Cancelled
                _ => break,
            }
        }
        assert_eq!(count, 3);
        assert_eq!(total, 0 + 1 + 2); // 0, 1, 2

        // Test creating a unit stream
        let mut rx = create_unit_stream(5).await;
        let mut count = 0u32;
        loop {
            let buf = Vec::<()>::with_capacity(10);
            let (result, _values) = rx.read(buf).await;
            match result {
                StreamResult::Complete(n) if n > 0 => {
                    count += n as u32;
                }
                // Complete(0) means end of stream, or Dropped/Cancelled
                _ => break,
            }
        }
        assert_eq!(count, 5);

        let (tx, signal) = wit_future::new(|| ());
        let (mut rx, done) = create_bridge_stream_with_signal(signal).await;
        let (result, values) = rx.read(Vec::<u32>::with_capacity(1)).await;
        match result {
            StreamResult::Complete(1) => assert_eq!(values[0], 0),
            other => panic!("expected one value before close, got {other:?}"),
        }
        drop(rx);
        tx.write(()).await.unwrap();
        assert!(!done.await);

        let (tx, signal) = wit_future::new(|| ());
        let (mut rx, done) = create_bridge_unit_stream_with_signal(signal).await;
        let (result, values) = rx.read(Vec::<()>::with_capacity(1)).await;
        match result {
            StreamResult::Complete(1) => assert_eq!(values.len(), 1),
            other => panic!("expected one unit before close, got {other:?}"),
        }
        drop(rx);
        tx.write(()).await.unwrap();
        assert!(!done.await);

        let (tx, signal) = wit_future::new(|| ());
        let (mut rx, done) = create_bridge_thing_stream_with_signal(signal).await;
        let (result, values) = rx.read(Vec::<Thing>::with_capacity(1)).await;
        let thing = match result {
            StreamResult::Complete(1) => values.into_iter().next().unwrap(),
            other => panic!("expected one thing before close, got {other:?}"),
        };
        assert_eq!(thing.value(), 0);
        drop(rx);
        tx.write(()).await.unwrap();
        assert!(!done.await);
        drop(thing);
        for _ in 0..5 {
            yield_async().await;
        }
        assert_eq!(active_things(), 0);
    }
}
