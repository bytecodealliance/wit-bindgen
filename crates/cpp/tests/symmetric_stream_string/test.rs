include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::a::b::the_test::Guest;

use wit_bindgen::rt::async_support::FutureReader;

impl Guest for Component {
    fn f() -> wit::FutureReader {
        let (wr,rd) = wit_future::new();
        async_support::spawn(move || async {
            wr.write(String::from("Hello")).await;
        });
        rd
    }
}
