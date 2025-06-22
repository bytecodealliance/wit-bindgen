// include!(env!("BINDINGS"));
include!("bindings/test.rs");

struct Component;

export!(Component);

use exports::a::b::the_test::Guest;

use wit_bindgen::rt::async_support::{self, FutureReader};

impl Guest for Component {
    fn f() -> FutureReader<String> {
        let (wr, rd) = wit_future::new(String::default);
        async_support::spawn(async move {
            wr.write(String::from("Hello")).await;
        });
        rd
    }
}
