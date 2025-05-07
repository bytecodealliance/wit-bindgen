include!(env!("BINDINGS"));

use crate::a::b::i::*;

fn main() {
    wit_bindgen::block_on(async {
        one_argument(1).await;
        assert_eq!(one_result().await, 2);
        assert_eq!(one_argument_and_result(3).await, 4);
        two_arguments(5, 6).await;
        assert_eq!(two_arguments_and_result(7, 8).await, 9);
    });
}
