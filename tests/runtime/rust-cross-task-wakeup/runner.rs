//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use wit_bindgen::yield_async;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        futures::join! {
            async {
                pending_import().await;
            },
            async {
                // Ensure that the above future has hit its pending state by
                // spinning a few times here to guarantee that we've yielded to
                // it.
                for _ in 0..5 {
                    yield_async().await;
                }
                resolve_pending_import();
            },
        };
    }
}
