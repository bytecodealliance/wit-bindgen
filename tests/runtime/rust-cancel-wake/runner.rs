//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::test::rust_cancel_wake::i::*;
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;
use wit_bindgen::yield_async;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        let (checkpoint, checkpoint_rx) = wit_future::new(|| unreachable!());
        let mut task = Box::pin(cancellable(checkpoint_rx));
        assert!(
            task.as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending()
        );

        // This callback clears the wake from spawning and lets the task sleep
        // on its Rust oneshot receiver before we cancel it.
        checkpoint.write(()).await.unwrap();
        // Two polls: the waiter registers its waker, then the root blocks.
        while get_status().polls < 2 {
            yield_async().await;
        }
        assert_eq!(get_status().polls, 2);

        // Dropping the import delivers EVENT_CANCEL to the sleeping task.
        drop(task);
        yield_async().await;
        let status = get_status();
        assert!(status.dropped);
        assert_eq!(status.polls, 2);
    }
}
