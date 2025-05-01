include!(env!("BINDINGS"));

use crate::my::test::i::{ping, pong};

fn main() {
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new();
        let f1 = ping(rx, "world".into());
        let f2 = async { tx.write("hello".into()).await.unwrap() };
        let (rx2, ()) = futures::join!(f1, f2);
        let m2 = rx2.await.unwrap();
        assert_eq!(m2, "helloworld");

        let (tx, rx) = wit_future::new();
        let f1 = async move {
            let m3 = pong(rx).await;
            assert_eq!(m3, "helloworld");
        };
        let f2 = async { tx.write(m2).await.unwrap() };
        let ((), ()) = futures::join!(f1, f2);
    });
}
