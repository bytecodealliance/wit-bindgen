use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
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
}
