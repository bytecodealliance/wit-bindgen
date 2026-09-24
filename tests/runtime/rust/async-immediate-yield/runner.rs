include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        foo::hi().await;

        wit_bindgen::block_on(async {
            wit_bindgen::yield_async().await;
        });
        println!("done");
    }
}
