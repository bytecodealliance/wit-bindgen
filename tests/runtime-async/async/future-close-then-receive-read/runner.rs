include!(env!("BINDINGS"));

use crate::a::b::the_test::{get, set};

fn main() {
    let (tx, rx) = wit_future::new();

    set(rx);
    let rx = get();
    drop(tx);
    drop(rx);

    wit_future::new::<()>();
}
