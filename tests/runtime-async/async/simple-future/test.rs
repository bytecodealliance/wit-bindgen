use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn read_future(x: FutureReader<()>) {
        x.await.unwrap();
    }

    async fn close_future(x: FutureReader<()>) {
        drop(x);
    }
}
