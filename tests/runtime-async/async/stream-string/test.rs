include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::a::b::the_test::Guest;

use wit_bindgen::rt::async_support::{self, StreamReader};

impl Guest for Component {
    fn f() -> StreamReader<String> {
        let (mut wr, rd) = wit_stream::new();
        async_support::spawn(async move {
            wr.write(vec![String::from("Hello")]).await;
            wr.write(vec![String::from("World!")]).await;
            wr.write(vec![String::from("From")]).await;
            wr.write(vec![String::from("a")]).await;
            wr.write(vec![String::from("stream.")]).await;
        });
        rd
    }
}
