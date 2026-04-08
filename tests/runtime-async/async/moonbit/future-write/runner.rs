//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::yield_async;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // Test creating a future with a value
        let rx = create_future_with_value(42).await;
        let value = rx.await;
        assert_eq!(value, 42);

        // Test creating a unit future
        let rx = create_unit_future().await;
        rx.await;

        // Test future<future<u32>>
        let outer = create_nested_future(7).await;
        let inner = outer.await;
        let nested_value = inner.await;
        assert_eq!(nested_value, 7);

        // Test record containing future<future<u32>>
        let record = create_nested_future_record(9).await;
        let record_inner = record.nested.await;
        let record_value = record_inner.await;
        assert_eq!(record_value, 9);

        let (tx, signal) = wit_future::new(|| ());
        let (resource_future, done) = create_dropped_resource_future_with_signal(signal).await;
        drop(resource_future);
        tx.write(()).await.unwrap();
        done.await;
        for _ in 0..5 {
            yield_async().await;
        }
        assert_eq!(active_things(), 0);
    }
}
