//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        assert_eq!(unit_ok().await, Ok(()));
        assert_eq!(unit_err().await, Err("unit".to_string()));
        assert_eq!(halve(8).await, Ok(4));
        assert_eq!(halve(7).await, Err("odd".to_string()));
    }
}
