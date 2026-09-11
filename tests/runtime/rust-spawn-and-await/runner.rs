//@ wasmtime-flags = '-Wcomponent-model-async'

include!(env!("BINDINGS"));

use crate::test::rust_spawn_and_await::i::{
    await_resolve, await_task, cancel_task, resolve, start,
};
use futures::task::noop_waker_ref;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;

struct Component;

export!(Component);

impl Guest for Component {
    async fn run() {
        // Awaiting a `Task` works.
        let _cm_task = start_task();
        resolve().await;
        let result = await_task().await;
        assert_eq!(result, Some(42));

        // Cancelling a `Task` before it completes returns `None`.
        let _cm_task = start_task();
        let result = cancel_task().await;
        resolve().await;
        assert_eq!(result, None);

        // Cancelling a `Task` after it completes returns the result anyway.
        let _cm_task = start_task();
        resolve().await;
        await_resolve().await;
        let result = cancel_task().await;
        assert_eq!(result, Some(42));

        // Check that awaiting a `Task` returns None after the CM-async task has
        // been terminated.
        let cm_task = start_task();
        drop(cm_task);
        assert_eq!(await_task().await, None);
        resolve().await;

        // Check that cancelling a `Task` returns None after the CM-async task
        // has been terminated.
        let cm_task = start_task();
        drop(cm_task);
        assert_eq!(cancel_task().await, None);
        resolve().await;
    }
}

fn start_task() -> Pin<Box<dyn Future<Output = ()>>> {
    let mut task = Box::pin(start());
    assert!(
        task.as_mut()
            .poll(&mut Context::from_waker(noop_waker_ref()))
            .is_pending()
    );
    task
}
