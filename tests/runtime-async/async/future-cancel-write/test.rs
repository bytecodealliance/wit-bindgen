use wit_bindgen::FutureReader;

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    fn take_then_drop(x: FutureReader<String>) {
        drop(x)
    }
    async fn read_and_drop(x: FutureReader<String>) {
        let _ = x.await;
    }
}
