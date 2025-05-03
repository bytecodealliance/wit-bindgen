include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::a::b::the_test::Guest;

use wit_bindgen::rt::async_support::{self, StreamReader};

impl Guest for Component {
    fn f() -> StreamReader<String> {
        let (wr, rd) = wit_future::new();
        async_support::spawn(async move {
            wr.write(String::from("Hello")).await;
            wr.write(String::from("World!")).await;
            wr.write(String::from("From")).await;
            wr.write(String::from("a")).await;
            wr.write(String::from("stream.")).await;
        });
        rd
    }
}
