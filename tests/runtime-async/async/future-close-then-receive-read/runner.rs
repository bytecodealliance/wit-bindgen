include!(env!("BINDINGS"));

use crate::a::b::the_test::{get, set};

fn main() {
    let (tx, rx) = wit_future::new(|| ());

    set(rx);
    let rx = get();
    drop(rx);
    drop(tx);

    let (_tx, rx) = wit_future::new::<()>(|| ());
    drop(rx);
}
