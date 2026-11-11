use wit_bindgen::StreamReader;

include!(env!("BINDINGS"));

use crate::exports::my::test::i::Point;

struct Component;

export!(Component);

impl crate::exports::my::test::i::Guest for Component {
    async fn echo(s: String) -> String {
        wit_bindgen::yield_async().await;
        s + "!"
    }

    async fn make_point(x: u32, y: u32) -> Point {
        wit_bindgen::yield_async().await;
        Point {
            x,
            y,
            label: format!("({x}, {y})"),
        }
    }

    async fn words(all: Vec<String>) -> StreamReader<String> {
        let (mut tx, rx) = wit_stream::new();
        wit_bindgen::spawn_local(async move {
            let remaining = tx.write_all(all).await;
            assert!(remaining.is_empty());
        });
        rx
    }
}
