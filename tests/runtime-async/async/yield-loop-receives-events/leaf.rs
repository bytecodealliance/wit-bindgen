include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl crate::exports::test::common::i_middle::Guest for Component {
    async fn f() {
        for _ in 0..2 {
            wit_bindgen::yield_async().await;
        }
    }
}
