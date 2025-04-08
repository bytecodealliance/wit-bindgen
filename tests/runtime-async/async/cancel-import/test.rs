use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn pending_import(x: FutureReader<()>) {
        x.await.unwrap();
    }

    fn backpressure_set(x: bool) {
        wit_bindgen::backpressure_set(x)
    }
}
