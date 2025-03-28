include!(env!("BINDINGS"));

use crate::a::b::the_test::f;

fn main() {
    let (tx, rx) = wit_future::new();

    let rx = f(rx);
    drop(tx);
    drop(rx);
}
