include!(env!("BINDINGS"));

struct Test;

export!(Test);

impl exports::foo::Guest for Test {
    async fn hi() {
        wit_bindgen::yield_async().await;
    }
}
