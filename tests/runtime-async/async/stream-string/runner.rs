include!(env!("BINDINGS"));

use wit_bindgen::rt::async_support;

use crate::a::b::the_test::f;
use futures::StreamExt;

fn main() {
    async_support::block_on(async {
        let mut stream = f();
        let result = stream.next().await;
        assert_eq!(result, Some(vec![String::from("Hello")]));
        let result = stream.next().await;
        assert_eq!(result, Some(vec![String::from("World!")]));
        let result = stream.next().await;
        assert_eq!(result, Some(vec![String::from("From")]));
        let result = stream.next().await;
        assert_eq!(result, Some(vec![String::from("a")]));
        let result = stream.next().await;
        assert_eq!(result, Some(vec![String::from("stream.")]));
        let result = stream.next().await;
        assert_eq!(result, None);
    });
}
