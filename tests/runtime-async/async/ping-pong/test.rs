use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn ping(x: FutureReader<String>, y: String) -> FutureReader<String> {
        let msg = x.await.unwrap() + y.as_str();
        let (tx, rx) = wit_future::new();
        wit_bindgen::spawn(async move {
            tx.write(msg).await.unwrap();
        });
        rx
    }

    async fn pong(x: FutureReader<String>) -> String {
        x.await.unwrap()
    }
}
