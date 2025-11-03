include!(env!("BINDINGS"));

use std::future::Future;

use crate::a::b::i::*;

// Explicitly require Send.
#[allow(dead_code)]
fn require_send<T: Send>(_t: &T) {}

// This is the type of block_on with a Send requirement added.
pub fn block_on_require_send<T: 'static>(future: impl Future<Output = T> + Send + 'static) -> T {
    require_send(&future);
    wit_bindgen::block_on(future)
}

fn main() {
    block_on_require_send(async {
        one_argument("hello".into()).await;
    });
}
