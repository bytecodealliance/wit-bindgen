use wit_bindgen::{FutureReader, StreamReader};

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn consume(data: StreamReader<u8>) -> FutureReader<Result<u32, String>> {
        let (tx, rx) = wit_future::new(|| unreachable!());
        wit_bindgen::spawn_local(async move {
            let bytes = data.collect().await;
            let result = if bytes.contains(&0) {
                Err("zero".to_string())
            } else {
                Ok(bytes.iter().map(|b| u32::from(*b)).sum())
            };
            tx.write(result).await.unwrap();
        });
        rx
    }
}
