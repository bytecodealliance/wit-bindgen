include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::StreamResult;

fn main() {
    wit_bindgen::block_on(async {
        let (mut tx, rx) = wit_stream::new();
        let test = async {
            // write one item
            let (result, ret) = tx.write(vec![()]).await;
            assert_eq!(result, StreamResult::Complete(1));
            assert_eq!(ret.remaining(), 0);

            // write two items
            let (result, ret) = tx.write(vec![(), ()]).await;
            assert_eq!(result, StreamResult::Complete(2));
            assert_eq!(ret.remaining(), 0);

            // write two items again
            let (result, ret) = tx.write(vec![(), ()]).await;
            assert_eq!(result, StreamResult::Closed);
            assert_eq!(ret.remaining(), 2);
        };
        let ((), ()) = futures::join!(test, read_stream(rx));
    });
}
