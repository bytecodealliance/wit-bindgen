use wit_bindgen::{StreamReader, StreamResult};

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn create_stream_with_values(count: u32) -> StreamReader<u32> {
        let (mut tx, rx) = wit_stream::new();
        wit_bindgen::spawn(async move {
            for i in 0..count {
                let (result, _rest) = tx.write(vec![i]).await;
                if !matches!(result, StreamResult::Complete(1)) {
                    break;
                }
            }
        });
        rx
    }

    async fn create_unit_stream(count: u32) -> StreamReader<()> {
        let (mut tx, rx) = wit_stream::new();
        wit_bindgen::spawn(async move {
            for _ in 0..count {
                let (result, _rest) = tx.write(vec![()]).await;
                if !matches!(result, StreamResult::Complete(1)) {
                    break;
                }
            }
        });
        rx
    }
}
