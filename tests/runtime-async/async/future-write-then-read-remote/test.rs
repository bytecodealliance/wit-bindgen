include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::a::b::the_test::Guest;

use wit_bindgen::rt::async_support::FutureReader;

impl Guest for Component {
    async fn f(future: FutureReader<()>) {
        eprintln!("e1");
        future.await.unwrap();
        eprintln!("e2");
    }
}
