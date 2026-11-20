//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::my::test::i::*;
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;

struct Component;

export!(Component);

impl Guest for Component {
    // Start each import call and poll it once so it's pending, then drop
    // it to cancel the subtask.
    async fn run() {
        // Cancel a call blocked reading a future. The callee must drop its
        // reader, so our write fails instead of hanging.
        let (tx, rx) = wit_future::new(|| unreachable!());
        let mut import = Box::pin(pending_future(rx));
        assert!(
            import
                .as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending()
        );
        drop(import);
        assert!(tx.write(()).await.is_err());

        // Cancel a call blocked reading a stream. The callee must drop its
        // reader, so our writer sees the stream as dropped instead of hanging.
        let (mut tx, rx) = wit_stream::new::<()>();
        let mut import = Box::pin(pending_stream(rx));
        assert!(
            import
                .as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending()
        );
        drop(import);
        assert!(tx.write_one(()).await.is_some());

        // Cancel a call blocked writing to a stream. The callee must drop its
        // writer, so our reader sees the stream as dropped instead of hanging.
        let mut rx = make_stream();
        let mut import = Box::pin(pending_stream_write());
        assert!(
            import
                .as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending()
        );
        drop(import);
        assert!(rx.next().await.is_none());
    }
}
