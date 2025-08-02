include!(env!("BINDINGS"));

use crate::a::b::the_test::f;

fn main() {
    wit_bindgen::block_on(async {
        let (tx, rx) = wit_future::new(|| ());

        drop(tx);

        f(rx).await;
    });
}
