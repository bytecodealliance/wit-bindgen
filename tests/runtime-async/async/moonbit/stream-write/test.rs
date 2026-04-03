use wit_bindgen::{FutureReader, StreamReader, StreamResult};

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
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
}
