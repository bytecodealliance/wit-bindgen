include!(env!("BINDINGS"));

use wit_bindgen::rt::async_support;

use crate::a::b::the_test::f;

fn main() {
    async_support::block_on(async {
        let result = f().await;
        assert_eq!(result, String::from("Hello"));
    });
}
