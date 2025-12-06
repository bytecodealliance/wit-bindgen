include!(env!("BINDINGS"));

use crate::a::b::the_test::f;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (tx, rx) = wit_future::new(|| ());

        let rx = f(rx);
        drop(rx);
        drop(tx);
    }
}
