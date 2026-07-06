//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::a::b::the_test::f;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (tx, rx) = wit_future::new(|| unreachable!());

        let a = async { tx.write(()).await };
        let b = async { f(rx).await };
        let (a_result, ()) = futures::join!(a, b);
        a_result.unwrap()
    }
}
