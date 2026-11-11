//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        assert_eq!(echo("hi".to_string()).await, "hi!");

        let p = make_point(3, 4).await;
        assert_eq!((p.x, p.y, p.label.as_str()), (3, 4, "(3, 4)"));

        let all = vec!["a".to_string(), "bb".to_string(), "ccc".to_string()];
        let rx = words(all.clone()).await;
        assert_eq!(rx.collect().await, all);
    }
}
