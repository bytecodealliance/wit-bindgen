//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;

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
    }
}
