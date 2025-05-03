//@ args = '--async=-none'

//include!(env!("BINDINGS"));
include!("bindings/runner.rs");

use wit_bindgen::rt::async_support;

use crate::a::b::the_test::f;

fn main() {
    async_support::block_on(async {
        let result = f().await;
        assert_eq!(result, Some(String::from("Hello")));
    });
}
