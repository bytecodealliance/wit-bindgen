//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::StreamResult;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (mut tx, rx) = wit_stream::new();
        let result = consume(rx).await;
        let (written, _) = tx.write(vec![1, 2, 3]).await;
        assert_eq!(written, StreamResult::Complete(3));
        drop(tx);
        assert_eq!(result.await, Ok(6));

        let (mut tx, rx) = wit_stream::new();
        let result = consume(rx).await;
        let (written, _) = tx.write(vec![0]).await;
        assert_eq!(written, StreamResult::Complete(1));
        drop(tx);
        assert_eq!(result.await, Err("zero".to_string()));
    }
}
