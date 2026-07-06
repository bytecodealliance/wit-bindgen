use futures::stream::StreamExt;
use wit_bindgen::StreamReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn read_stream(x: StreamReader<u8>) {
        // Convert the low-level StreamReader into a futures::Stream
        let mut stream = x.into_stream();

        // Read all items via StreamExt::next()
        let first = stream.next().await;
        assert_eq!(first, Some(10));

        let second = stream.next().await;
        assert_eq!(second, Some(20));

        let third = stream.next().await;
        assert_eq!(third, Some(30));

        // Stream should be exhausted after the writer is dropped
        let end = stream.next().await;
        assert_eq!(end, None);
    }
}
