include!(env!("BINDINGS"));

use wit_bindgen::yield_async;

struct Component;

export!(Component);

impl crate::exports::a::b::i::Guest for Component {
    async fn f() {
        for _ in 0..10 {
            yield_async().await;
        }
    }
}
