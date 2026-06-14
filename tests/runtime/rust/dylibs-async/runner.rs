//@ [lang]
//@ link_shared = true

include!(env!("BINDINGS"));

use crate::my::inline::a;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (tx, rx) = wit_future::new(|| 3u32);
        drop(tx);
        assert_eq!(a::a(rx).await, 3);
    }
}
