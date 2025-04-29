include!(env!("BINDINGS"));

use crate::my::test::i::*;

fn main() {
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new();
        cancel_before_read(rx).await;
        drop(tx);

        let (tx, rx) = wit_future::new();
        cancel_after_read(rx).await;
        drop(tx);

        start_read_then_cancel().await;
    });
}
