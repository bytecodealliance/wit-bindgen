include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::StreamResult;

fn main() {
    wit_bindgen::block_on(async {
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
            assert_eq!(result, StreamResult::Complete(1));
            assert_eq!(ret.remaining(), 1);

            // resume the write
            let (result, ret) = tx.write_buf(ret).await;
            assert_eq!(result, StreamResult::Complete(1));
            assert_eq!(ret.remaining(), 0);

            // write to a closed stream
            let (result, ret) = tx.write(vec![0]).await;
            assert_eq!(result, StreamResult::Closed);
            assert_eq!(ret.remaining(), 1);
        };
        let ((), ()) = futures::join!(test, read_stream(rx));
    });
}
