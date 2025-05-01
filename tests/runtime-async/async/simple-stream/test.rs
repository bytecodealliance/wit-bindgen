use wit_bindgen::{StreamReader, StreamResult};

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn read_stream(mut x: StreamReader<()>) {
        // read one item
        let (result, buf) = x.read(Vec::with_capacity(1)).await;
        assert_eq!(result, StreamResult::Complete(1));
        assert_eq!(buf.len(), 1);

        // read two items
        let (result, buf) = x.read(Vec::with_capacity(2)).await;
        assert_eq!(result, StreamResult::Complete(2));
        assert_eq!(buf.len(), 2);

        // close
        drop(x);
    }
}
