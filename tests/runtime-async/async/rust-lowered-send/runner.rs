include!(env!("BINDINGS"));

use crate::a::b::i::*;

// Explicitly require Send.
fn require_send<T: Send>(t: T) -> T {
    t
}

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        require_send(async {
            one_argument("hello".into()).await;
        })
        .await;
    }
}
