include!(env!("BINDINGS"));

use crate::a::b::the_test::f;

fn main() {
    wit_bindgen::block_on(async move {
        let (tx, rx) = wit_future::new(|| 0);

        drop(tx.write(4));

        f(rx).await;
    });
}
