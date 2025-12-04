include!(env!("BINDINGS"));

use crate::my::test::i::*;
use futures::task::noop_waker_ref;
use std::future::Future;
use std::task::Context;
use wit_bindgen::yield_async;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // Test that Rust-level polling twice works.
        {
            let (tx, rx) = wit_future::new(|| unreachable!());
            let mut import = Box::pin(pending_import(rx));
            assert!(import
                .as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending());
            assert!(import
                .as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending());
            tx.write(()).await.unwrap();
            import.await;
        }

        // Start the imported function call, get it pending, then let it complete by
        // finishing `tx`, then yield a few times to ensure that the runtime gets
        // the completion of the task-at-hand, and then drop it without completing
        // it.
        {
            let (tx, rx) = wit_future::new(|| unreachable!());
            let mut import = Box::pin(pending_import(rx));
            assert!(import
                .as_mut()
                .poll(&mut Context::from_waker(noop_waker_ref()))
                .is_pending());
            tx.write(()).await.unwrap();

            for _ in 0..5 {
                yield_async().await;
            }
            drop(import);
        }
    }
}
