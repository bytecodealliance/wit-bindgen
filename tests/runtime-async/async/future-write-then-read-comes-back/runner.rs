include!(env!("BINDINGS"));

use wit_bindgen::rt::async_support;

use crate::a::b::the_test::f;

fn main() {
    async_support::block_on(async {
        let (tx, rx) = wit_future::new(|| unreachable!());

        let a = async { tx.write(()).await };
        let b = async { f(rx).await };
        let (a_result, ()) = futures::join!(a, b);
        a_result.unwrap()
    });
}
