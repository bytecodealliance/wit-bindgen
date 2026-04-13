//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::StreamResult;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (mut tx, rx) = wit_stream::new();
        let test = async {
            let (result, _ret) = tx.write(vec![10, 20, 30]).await;
            assert_eq!(result, StreamResult::Complete(3));

            // Drop the writer so the reader sees the end of the stream.
            drop(tx);
        };
        let ((), ()) = futures::join!(test, read_stream(rx));
    }
}
