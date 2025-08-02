include!(env!("BINDINGS"));

use crate::my::test::i::*;

fn main() {
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new(|| 0);
        cancel_before_read(rx).await;
        drop(tx);

        let (tx, rx) = wit_future::new(|| 0);
        cancel_after_read(rx).await;
        drop(tx);

        let (data_tx, data_rx) = wit_future::new(|| unreachable!());
        let (signal_tx, signal_rx) = wit_future::new(|| unreachable!());
        let ((), ()) = futures::join!(start_read_then_cancel(data_rx, signal_rx), async {
            signal_tx.write(()).await.unwrap();
            data_tx.write(4).await.unwrap();
        });
    });
}
