//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::StreamResult;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // After the writer drops, repeated reads remain Dropped.
        let (tx, mut rx) = wit_stream::new::<u8>();
        drop(tx);
        for _ in 0..2 {
            let (result, buf) = rx.read(Vec::with_capacity(1)).await;
            assert_eq!(result, StreamResult::Dropped);
            assert!(buf.is_empty());
        }

        // After the reader drops, repeated writes preserve unsent values.
        let (mut tx, rx) = wit_stream::new::<u8>();
        drop(rx);
        for _ in 0..2 {
            let (result, buf) = tx.write(vec![42]).await;
            assert_eq!(result, StreamResult::Dropped);
            assert_eq!(buf.into_vec(), vec![42]);
        }
        let (mut tx, rx) = wit_stream::new();
        let test = async {
            // write one item
            let (result, ret) = tx.write(vec![0]).await;
            assert_eq!(result, StreamResult::Complete(1));
            assert_eq!(ret.remaining(), 0);

            // write two items
            let (result, ret) = tx.write(vec![1, 2]).await;
            assert_eq!(result, StreamResult::Complete(2));
            assert_eq!(ret.remaining(), 0);

            // write 1/2 items again
            let (result, ret) = tx.write(vec![3, 4]).await;
            assert_eq!(result, StreamResult::Complete(2));
            assert_eq!(ret.remaining(), 0);

            // write to a dropped stream
            let (result, ret) = tx.write(vec![0]).await;
            assert_eq!(result, StreamResult::Dropped);
            assert_eq!(ret.remaining(), 1);
        };
        let ((), ()) = futures::join!(test, read_stream(rx));
    }
}
