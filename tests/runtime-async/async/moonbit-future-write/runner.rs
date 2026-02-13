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
    }
}
